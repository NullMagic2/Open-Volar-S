/* WSL-only compatibility for older presentation/audio libraries using realtime
 * condition deadlines. In affected WSL kernels, short realtime waits can sleep
 * for seconds. Preserve each condition's clock, converting only realtime waits
 * to an equivalent monotonic deadline. Loaded only into our mpv child process.
 * No system clock, shared library, or other application is modified. */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <pthread.h>
#include <stdlib.h>
#include <time.h>

typedef struct ClockEntry {
    pthread_cond_t *condition;
    clockid_t clock;
    struct ClockEntry *next;
} ClockEntry;
static pthread_mutex_t clocks_lock = PTHREAD_MUTEX_INITIALIZER;
static ClockEntry *clocks;
static pthread_once_t resolve_once = PTHREAD_ONCE_INIT;
static int (*real_init)(pthread_cond_t *, const pthread_condattr_t *);
static int (*real_destroy)(pthread_cond_t *);
static int (*real_wait)(pthread_cond_t *, pthread_mutex_t *, const struct timespec *);
static int (*clock_wait)(pthread_cond_t *, pthread_mutex_t *, clockid_t, const struct timespec *);

static void resolve(void) {
    /* Explicit symbol versions avoid glibc's incompatible legacy condvar ABI. */
#if defined(__x86_64__)
    real_init = dlvsym(RTLD_NEXT, "pthread_cond_init", "GLIBC_2.3.2");
    real_destroy = dlvsym(RTLD_NEXT, "pthread_cond_destroy", "GLIBC_2.3.2");
    real_wait = dlvsym(RTLD_NEXT, "pthread_cond_timedwait", "GLIBC_2.3.2");
#else
    real_init = dlsym(RTLD_NEXT, "pthread_cond_init");
    real_destroy = dlsym(RTLD_NEXT, "pthread_cond_destroy");
    real_wait = dlsym(RTLD_NEXT, "pthread_cond_timedwait");
#endif
    clock_wait = dlsym(RTLD_NEXT, "pthread_cond_clockwait");
    if (!real_init || !real_destroy || !real_wait) abort();
}

int pthread_cond_init(pthread_cond_t *condition, const pthread_condattr_t *attr) {
    pthread_once(&resolve_once, resolve);
    clockid_t clock = CLOCK_REALTIME;
    if (attr) {
        int error = pthread_condattr_getclock(attr, &clock);
        if (error) return error;
    }
    ClockEntry *entry = NULL;
    if (clock != CLOCK_REALTIME) {
        entry = malloc(sizeof(*entry));
        if (!entry) return ENOMEM;
        *entry = (ClockEntry){ .condition = condition, .clock = clock };
    }
    int error = real_init(condition, attr);
    if (error) { free(entry); return error; }
    if (entry) {
        pthread_mutex_lock(&clocks_lock);
        entry->next = clocks;
        clocks = entry;
        pthread_mutex_unlock(&clocks_lock);
    }
    return 0;
}

int pthread_cond_destroy(pthread_cond_t *condition) {
    pthread_once(&resolve_once, resolve);
    int error = real_destroy(condition);
    if (error) return error;
    pthread_mutex_lock(&clocks_lock);
    ClockEntry **entry = &clocks;
    while (*entry && (*entry)->condition != condition) entry = &(*entry)->next;
    if (*entry) {
        ClockEntry *old = *entry;
        *entry = old->next;
        free(old);
    }
    pthread_mutex_unlock(&clocks_lock);
    return 0;
}

int pthread_cond_timedwait(pthread_cond_t *condition, pthread_mutex_t *mutex,
                          const struct timespec *deadline) {
    pthread_once(&resolve_once, resolve);
    if (!clock_wait || deadline->tv_nsec < 0 || deadline->tv_nsec >= 1000000000)
        return real_wait(condition, mutex, deadline);
    clockid_t clock = CLOCK_REALTIME;
    pthread_mutex_lock(&clocks_lock);
    for (ClockEntry *entry = clocks; entry; entry = entry->next)
        if (entry->condition == condition) { clock = entry->clock; break; }
    pthread_mutex_unlock(&clocks_lock);
    if (clock != CLOCK_REALTIME) return real_wait(condition, mutex, deadline);

    struct timespec realtime, monotonic, target;
    clock_gettime(CLOCK_REALTIME, &realtime);
    clock_gettime(CLOCK_MONOTONIC, &monotonic);
    /* Use wide arithmetic for arbitrarily distant or expired POSIX deadlines. */
    __int128 nanos = ((__int128)deadline->tv_sec - realtime.tv_sec + monotonic.tv_sec)
                    * 1000000000 + deadline->tv_nsec - realtime.tv_nsec + monotonic.tv_nsec;
    if (nanos < 0) nanos = 0;
    const __int128 maximum = (__int128)0x7fffffffffffffffLL * 1000000000 + 999999999;
    if (nanos > maximum) nanos = maximum;
    target.tv_sec = nanos / 1000000000;
    target.tv_nsec = nanos % 1000000000;
    return clock_wait(condition, mutex, CLOCK_MONOTONIC, &target);
}
