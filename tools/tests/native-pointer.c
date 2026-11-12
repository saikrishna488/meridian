/* Private nested-session input fixture. Protocol: swaywm/wlr-protocols,
 * unstable/wlr-virtual-pointer-unstable-v1.xml (MIT, vendored alongside).
 * Build with wayland-scanner output and libwayland-client. */
#define _POSIX_C_SOURCE 200809L
#include <wayland-client.h>
#include "virtual-pointer-client.h"
#include <errno.h>
#include <linux/input-event-codes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static struct zwlr_virtual_pointer_manager_v1 *manager;
static void global(void *data, struct wl_registry *registry, uint32_t name, const char *interface, uint32_t version) {
    (void)data; (void)version;
    if (strcmp(interface, zwlr_virtual_pointer_manager_v1_interface.name) == 0)
        manager = wl_registry_bind(registry, name, &zwlr_virtual_pointer_manager_v1_interface, 1);
}
static void removed(void *data, struct wl_registry *registry, uint32_t name) { (void)data; (void)registry; (void)name; }
static const struct wl_registry_listener listener = {global, removed};
static void pause_input(void) {
    const struct timespec delay = {.tv_nsec = 150000000};
    nanosleep(&delay, NULL);
}
static uint32_t timestamp(void) {
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return (uint32_t)((uint64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000);
}
int main(void) {
    struct wl_display *display = wl_display_connect(NULL);
    if (!display) return 1;
    struct wl_registry *registry = wl_display_get_registry(display);
    wl_registry_add_listener(registry, &listener, NULL);
    if (wl_display_roundtrip(display) < 0 || !manager) { fprintf(stderr, "Virtual pointer protocol unavailable\n"); return 1; }
    struct zwlr_virtual_pointer_v1 *pointer = zwlr_virtual_pointer_manager_v1_create_virtual_pointer(manager, NULL);
    unsigned x, y;
    char name[16];
    while (scanf("%u %u %15s", &x, &y, name) == 3) {
        if (x > 1280 || y > 720) return 2;
        uint32_t button;
        if (!strcmp(name, "left")) button = BTN_LEFT;
        else if (!strcmp(name, "right")) button = BTN_RIGHT;
        else return 2;
        zwlr_virtual_pointer_v1_motion_absolute(pointer, timestamp(), (uint32_t)x, (uint32_t)y, 1280, 720);
        zwlr_virtual_pointer_v1_frame(pointer);
        wl_display_roundtrip(display);
        pause_input();
        zwlr_virtual_pointer_v1_button(pointer, timestamp(), button, WL_POINTER_BUTTON_STATE_PRESSED);
        zwlr_virtual_pointer_v1_frame(pointer);
        wl_display_roundtrip(display);
        pause_input();
        zwlr_virtual_pointer_v1_button(pointer, timestamp(), button, WL_POINTER_BUTTON_STATE_RELEASED);
        zwlr_virtual_pointer_v1_frame(pointer);
        wl_display_roundtrip(display);
        pause_input();
        puts("done");
        fflush(stdout);
    }
    zwlr_virtual_pointer_v1_destroy(pointer);
    zwlr_virtual_pointer_manager_v1_destroy(manager);
    wl_registry_destroy(registry);
    wl_display_flush(display);
    wl_display_disconnect(display);
    return 0;
}
