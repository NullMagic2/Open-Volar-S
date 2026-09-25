/* Run with the built wsl-clock.so in LD_PRELOAD. Tests public pthread semantics,
 * including a caller that already uses CLOCK_MONOTONIC. */
#define _GNU_SOURCE
#include <assert.h>
#include <errno.h>
#include <pthread.h>
#include <stdio.h>
#include <time.h>

static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t condition = PTHREAD_COND_INITIALIZER;
static int ready;
static double seconds(void) {
    struct timespec t; clock_gettime(CLOCK_MONOTONIC, &t);
    return t.tv_sec + t.tv_nsec / 1e9;
}
static struct timespec after(clockid_t clock, long ns) {
    struct timespec t; clock_gettime(clock, &t); t.tv_nsec += ns;
    if (t.tv_nsec >= 1000000000) { t.tv_nsec -= 1000000000; t.tv_sec++; }
    return t;
}
static void timeout_test(clockid_t clock) {
    struct timespec deadline = after(clock, 30000000);
    double begin = seconds();
    pthread_mutex_lock(&lock);
    int result;
    do { result = pthread_cond_timedwait(&condition, &lock, &deadline); } while (result == 0);
    pthread_mutex_unlock(&lock);
    double elapsed = seconds() - begin;
    assert(result == ETIMEDOUT && elapsed >= .02 && elapsed < .3);
}
static void *signal_thread(void *unused) {
    (void)unused;
    struct timespec delay = { .tv_nsec = 10000000 }; nanosleep(&delay, NULL);
    pthread_mutex_lock(&lock); ready = 1; pthread_cond_signal(&condition); pthread_mutex_unlock(&lock);
    return NULL;
}
int main(void) {
    timeout_test(CLOCK_REALTIME); /* static initializer */
    pthread_mutex_lock(&lock);
    struct timespec invalid = { .tv_nsec = 1000000000 };
    assert(pthread_cond_timedwait(&condition, &lock, &invalid) == EINVAL);
    struct timespec expired = {0};
    assert(pthread_cond_timedwait(&condition, &lock, &expired) == ETIMEDOUT);
    pthread_mutex_unlock(&lock);
    for (int i = 0; i < 8; i++) {
        assert(!pthread_cond_destroy(&condition));
        pthread_condattr_t attr; assert(!pthread_condattr_init(&attr));
        clockid_t clock = i % 2 ? CLOCK_REALTIME : CLOCK_MONOTONIC;
        assert(!pthread_condattr_setclock(&attr, clock));
        assert(!pthread_cond_init(&condition, &attr)); assert(!pthread_condattr_destroy(&attr));
        timeout_test(clock);
        ready = 0;
        pthread_t thread; assert(!pthread_create(&thread, NULL, signal_thread, NULL));
        pthread_mutex_lock(&lock);
        struct timespec deadline = after(clock, 200000000);
        while (!ready) assert(!pthread_cond_timedwait(&condition, &lock, &deadline));
        pthread_mutex_unlock(&lock);
        assert(!pthread_join(thread, NULL));
    }
    assert(!pthread_cond_destroy(&condition));
    puts("WSL clock: realtime, monotonic, reuse, signal, timeout, and invalid-deadline checks passed");
}
