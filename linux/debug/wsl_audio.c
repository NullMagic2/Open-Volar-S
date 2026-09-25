// Private SDL callback adapter for WSL mpv. The inherited, authenticated socket
// carries only bounded float PCM blocks requested by the Windows audio clock.
#define _GNU_SOURCE
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <pthread.h>
#include <stdatomic.h>
#include <dlfcn.h>
#include <signal.h>
#include <errno.h>

// Stable SDL2 public ABI, kept here so packaging needs no SDL development SDK.
typedef struct {
    int freq; uint16_t format; uint8_t channels, silence;
    uint16_t samples, padding; uint32_t size;
    void (*callback)(void *, uint8_t *, int); void *userdata;
} AudioSpec;
static AudioSpec spec;
static int fd = -1, opened, paused = 1;
static atomic_int closing;
static pthread_t worker;
static pthread_mutex_t gate = PTHREAD_RECURSIVE_MUTEX_INITIALIZER_NP;
static _Thread_local unsigned locks;
static int transfer(void *data, size_t size, int output) {
    unsigned char *p = data;
    while (size) {
        ssize_t n = output ? send(fd, p, size, MSG_NOSIGNAL) : recv(fd, p, size, 0);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) return -1;
        p += n; size -= (size_t)n;
    }
    return 0;
}
static void *pump(void *unused) {
    (void)unused;
    uint8_t *pcm = malloc(spec.size);
    if (!pcm) goto failed;
    for (;;) {
        uint32_t frames = 0;
        if (transfer(&frames, 4, 0) || frames != spec.samples) break;
        uint32_t active = !atomic_load(&closing);
        if (!active) { transfer(&active, 4, 1); free(pcm); return NULL; }
        pthread_mutex_lock(&gate);
        memset(pcm, 0, spec.size);
        if (!paused && !atomic_load(&closing)) spec.callback(spec.userdata, pcm, (int)spec.size);
        pthread_mutex_unlock(&gate);
        if (transfer(&active, 4, 1) || transfer(pcm, spec.size, 1)) break;
    }
    free(pcm);
failed:
    if (!atomic_load(&closing)) {
        fprintf(stderr, "[ERROR]: Windows WASAPI audio connection lost; restart playback\n");
        // Do not continue silently or play audio against a stopped clock.
        kill(getpid(), SIGTERM);
    }
    return NULL;
}
int SDL_OpenAudio(AudioSpec *desired, AudioSpec *obtained) {
    const char *value = getenv("OVS_WSL_AUDIO_FD");
    if (!value) {
        int (*original)(AudioSpec *, AudioSpec *) = dlsym(RTLD_NEXT, "SDL_OpenAudio");
        return original(desired, obtained);
    }
    if (opened || !desired || !desired->callback || !obtained) return -1;
    fd = atoi(value);
    unsigned channels = desired->channels;
    if (channels != 1 && channels != 2 && channels != 6 && channels != 8) channels = 2;
    uint32_t config[2] = {48000, channels}, answer[4];
    if (transfer(config, sizeof config, 1) || transfer(answer, sizeof answer, 0) ||
        answer[0] != 0x4153564f || answer[1] != 48000 || answer[2] != channels ||
        answer[3] < 64 || answer[3] > 4096) return -1;
    spec = *desired;
    spec.freq = 48000; spec.format = 0x8120; // AUDIO_F32LSB
    spec.channels = (uint8_t)channels; spec.silence = 0;
    spec.samples = (uint16_t)answer[3]; spec.size = spec.samples * channels * 4;
    *obtained = spec;
    paused = 1; atomic_store(&closing, 0);
    if (pthread_create(&worker, NULL, pump, NULL)) return -1;
    opened = 1;
    return 0;
}
void SDL_PauseAudio(int pause) {
    if (!opened) { void (*original)(int) = dlsym(RTLD_NEXT, "SDL_PauseAudio"); original(pause); return; }
    pthread_mutex_lock(&gate); paused = pause; pthread_mutex_unlock(&gate);
}
void SDL_LockAudio(void) {
    if (!opened) { void (*original)(void) = dlsym(RTLD_NEXT, "SDL_LockAudio"); original(); return; }
    pthread_mutex_lock(&gate); ++locks;
}
void SDL_UnlockAudio(void) {
    if (!opened) { void (*original)(void) = dlsym(RTLD_NEXT, "SDL_UnlockAudio"); original(); return; }
    if (locks) { --locks; pthread_mutex_unlock(&gate); }
}
void SDL_CloseAudio(void) {
    if (!opened) { void (*original)(void) = dlsym(RTLD_NEXT, "SDL_CloseAudio"); original(); return; }
    atomic_store(&closing, 1);
    // mpv locks SDL before QuitSubSystem. Release that caller's lock before
    // joining the callback, matching SDL's close-while-locked contract.
    while (locks) { --locks; pthread_mutex_unlock(&gate); }
    pthread_join(worker, NULL); opened = 0;
}
void SDL_QuitSubSystem(uint32_t flags) {
    if ((flags & 0x10) && opened) SDL_CloseAudio();
    void (*original)(uint32_t) = dlsym(RTLD_NEXT, "SDL_QuitSubSystem"); original(flags);
}
