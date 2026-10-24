/* Synthetic test ELF only: real fork/exec, procfs ownership and child-owned IPC.
 * This is not COSMIC, a native driver or native qualification evidence. */
#define _GNU_SOURCE
#include <sys/socket.h>
#include <sys/un.h>
#include <sys/wait.h>
#include <poll.h>
#include <signal.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
static volatile sig_atomic_t stop;
static void stopping(int sig) { (void)sig; stop = 1; }
static void event(const char *name, int pid) {
    FILE *f = fopen(getenv("FIXTURE_EVENTS"), "a");
    if (!f) _exit(90);
    struct timespec now; clock_gettime(CLOCK_MONOTONIC, &now);
    fprintf(f, "%s %d %lld\n", name, pid, (long long)now.tv_sec*1000000000+now.tv_nsec); fclose(f);
}
static void spawn(const char *mode) {
    pid_t child = fork();
    if (child == 0) {
        if (!strcmp(mode, "wrong-session")) setenv("XDG_SESSION_ID", "other-fixture-session", 1);
        const char *path = getenv(!strcmp(mode, "mismatch") ? "FIXTURE_OTHER_APPLET" : "FIXTURE_APPLET");
        execl(path, path, "90", NULL); _exit(91);
    }
    if (child < 0) _exit(92);
    event("applet", child);
}
int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "--islands-probe")) {
        event("probe", getpid());
        if (getenv("WAYLAND_SOCKET") || getenv("PANEL_NOTIFICATIONS_FD") || getenv("COSMIC_SESSION_SOCK")) return 93;
        puts("{\"probe\":1,\"ready\":true}"); return 0;
    }
    const char *name = strrchr(argv[0], '/'); name = name ? name+1 : argv[0];
    if (!strcmp(name, "stock")) { event("stock", getpid()); return 0; }
    event("host", getpid());
    signal(SIGTERM, stopping); signal(SIGINT, stopping); signal(SIGHUP, stopping);
    signal(SIGPIPE, SIG_IGN);
    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0);
    struct sockaddr_un address = { .sun_family = AF_UNIX };
    snprintf(address.sun_path, sizeof(address.sun_path), "%s/desktop-cards-islands-v1.sock", getenv("XDG_RUNTIME_DIR"));
    unlink(address.sun_path);
    if (bind(fd, (struct sockaddr *)&address, sizeof(address)) || listen(fd, 16)) return 94;
    const char *mode = getenv("FIXTURE_MODE");
    struct timespec start, now; clock_gettime(CLOCK_MONOTONIC, &start);
    int spawned = 0;
    while (!stop) {
        clock_gettime(CLOCK_MONOTONIC, &now);
        if (!spawned && access(getenv("FIXTURE_ALLOW_SPAWN"), F_OK) == 0 && (now.tv_sec-start.tv_sec)*1000+(now.tv_nsec-start.tv_nsec)/1000000 > 250) {
            spawned = 1;
            if (!strcmp(mode, "replace")) {
                if (rename(getenv("FIXTURE_REPLACEMENT"), getenv("FIXTURE_APPLET"))) return 95;
            }
            if (strcmp(mode, "missing") && strcmp(mode, "foreign")) spawn(mode);
            if (!strcmp(mode, "excess")) spawn(mode);
        }
        struct pollfd pollfd = { .fd=fd, .events=POLLIN };
        if (poll(&pollfd, 1, 20) <= 0) continue;
        int client = accept4(fd, NULL, NULL, SOCK_CLOEXEC);
        if (client < 0) continue;
        char request[256];
        if (read(client, request, sizeof(request)) <= 0) { close(client); continue; }
        const char *reply = "{\"Capabilities\":{\"protocol\":1,\"together\":true,\"separate\":true}}\n";
        if (write(client, reply, strlen(reply)) < 0) { close(client); continue; }
        close(client);
    }
    close(fd); unlink(address.sun_path);
    while (waitpid(-1, NULL, 0) > 0) {}
    return 0;
}
