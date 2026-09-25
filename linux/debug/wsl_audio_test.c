// Protocol/lifecycle test: compile with wsl_audio.c and libSDL2, run on Linux.
#define _GNU_SOURCE
#include <assert.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include <sys/socket.h>
#include <pthread.h>
#include <stdatomic.h>
#include <string.h>
typedef struct {
    int freq; uint16_t format; uint8_t channels, silence;
    uint16_t samples, padding; uint32_t size;
    void (*callback)(void *, uint8_t *, int); void *userdata;
} AudioSpec;
extern int SDL_OpenAudio(AudioSpec *, AudioSpec *);
extern void SDL_PauseAudio(int);
extern void SDL_LockAudio(void);
extern void SDL_CloseAudio(void);
static atomic_int callbacks, nonzero, quiet;
static void callback(void *data, uint8_t *buffer, int size) {
    assert(data == &callbacks);
    for (int i=0; i<size/4; ++i) ((float *)buffer)[i]=0.25f;
    atomic_fetch_add(&callbacks,1);
}
static void exact(int fd,void *p,size_t n,int output) {
    while(n) {ssize_t r=output?write(fd,p,n):read(fd,p,n);assert(r>0);p=(char *)p+r;n-=(size_t)r;}
}
static void *host(void *arg) {
    int fd=*(int *)arg;
    for(int session=0;session<2;++session) {
        uint32_t config[2];exact(fd,config,sizeof config,0);
        assert(config[0]==48000 && config[1]==(session?6:2));
        uint32_t header[4]={0x4153564f,48000,config[1],480};exact(fd,header,sizeof header,1);
        for(;;) {
            uint32_t frames=480,active;exact(fd,&frames,4,1);exact(fd,&active,4,0);
            if(!active)break;
            assert(active==1);
            float pcm[480*8];exact(fd,pcm,480*config[1]*4,0);
            int any=0;for(unsigned i=0;i<480*config[1];++i){assert(pcm[i]==0 || pcm[i]==0.25f);any|=pcm[i]!=0;}
            if(any)atomic_fetch_add(&nonzero,1);else atomic_fetch_add(&quiet,1);
            usleep(10000);
        }
    }
    return NULL;
}
static void until(atomic_int *counter,int value) {
    for(int i=0;i<200 && atomic_load(counter)<value;++i)usleep(5000);
    assert(atomic_load(counter)>=value);
}
int main(void) {
    alarm(10);
    int sockets[2];assert(socketpair(AF_UNIX,SOCK_STREAM,0,sockets)==0);
    char fd[32];snprintf(fd,sizeof fd,"%d",sockets[0]);setenv("OVS_WSL_AUDIO_FD",fd,1);
    pthread_t thread;assert(!pthread_create(&thread,NULL,host,&sockets[1]));
    for(int session=0;session<2;++session) {
        AudioSpec wanted={.freq=44100,.format=0x8010,.channels=session?6:2,.callback=callback,.userdata=&callbacks},got;
        int start=atomic_load(&callbacks),silent=atomic_load(&quiet);
        assert(SDL_OpenAudio(&wanted,&got)==0);
        assert(got.freq==48000 && got.format==0x8120 && got.channels==wanted.channels && got.samples==480);
        until(&quiet,silent+3);assert(atomic_load(&callbacks)==start);
        SDL_PauseAudio(0);until(&nonzero,atomic_load(&nonzero)+4);
        SDL_PauseAudio(1);start=atomic_load(&callbacks);until(&quiet,atomic_load(&quiet)+3);
        assert(atomic_load(&callbacks)==start);
        // mpv quits while holding the SDL audio lock.
        SDL_LockAudio();SDL_CloseAudio();
    }
    pthread_join(thread,NULL);close(sockets[0]);close(sockets[1]);
    puts("PASS: negotiated stereo/5.1, paused silence, clocked callback, close while locked, reopen");
}
