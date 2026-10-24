/* Disposable protocol-only server: no compositor, GPU, session or user config.
 * libwayland owns the registry/object map and rejects invalid new object IDs. */
#include <wayland-server.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static void bind_output(struct wl_client *client, void *data,
                        uint32_t version, uint32_t id) {
    struct wl_resource *output = wl_resource_create(client, &wl_output_interface, version, id);
    char name[32];
    snprintf(name, sizeof(name), "TEST-%u", (unsigned)(uintptr_t)data);
    wl_output_send_geometry(output, 0, 0, 300, 200, WL_OUTPUT_SUBPIXEL_UNKNOWN,
                            "fixture", "fixture", WL_OUTPUT_TRANSFORM_NORMAL);
    wl_output_send_mode(output, WL_OUTPUT_MODE_CURRENT, 800, 600, 60000);
    wl_output_send_scale(output, 1);
    wl_output_send_name(output, name);
    wl_output_send_description(output, "Isolated protocol fixture");
    wl_output_send_done(output);
}
int main(int argc, char **argv) {
    if (argc != 3) return 2;
    struct wl_display *display = wl_display_create();
    if (!display || wl_display_add_socket_fd(display, atoi(argv[1]))) return 3;
    for (unsigned n = 1; n <= (unsigned)atoi(argv[2]); n++)
        if (!wl_global_create(display, &wl_output_interface, 4,
                              (void *)(uintptr_t)n, bind_output)) return 4;
    puts("ready"); fflush(stdout);
    wl_display_run(display);
    wl_display_destroy_clients(display);
    wl_display_destroy(display);
    return 0;
}
