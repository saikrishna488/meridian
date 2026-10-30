/* Test-only wlroots virtual pointer, controlled over stdin in a disposable compositor. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <wayland-client.h>
#include "virtual-pointer.h"
static struct zwlr_virtual_pointer_manager_v1 *manager;
static struct wl_output *outputs[16];
static char names[16][128];
static int count;
static void geometry(void *d, struct wl_output *o, int32_t x, int32_t y, int32_t pw, int32_t ph, int32_t sub, const char *make, const char *model, int32_t transform) {(void)d;(void)o;(void)x;(void)y;(void)pw;(void)ph;(void)sub;(void)make;(void)model;(void)transform;}
static void mode(void *d, struct wl_output *o, uint32_t flags, int32_t w, int32_t h, int32_t refresh) {(void)d;(void)o;(void)flags;(void)w;(void)h;(void)refresh;}
static void done(void *d, struct wl_output *o) {(void)d;(void)o;}
static void scale(void *d, struct wl_output *o, int32_t s) {(void)d;(void)o;(void)s;}
static void name(void *d, struct wl_output *o, const char *n) {(void)o; snprintf(names[(int)(intptr_t)d], 128, "%s", n);}
static void description(void *d, struct wl_output *o, const char *n) {(void)d;(void)o;(void)n;}
static const struct wl_output_listener output_listener = {geometry, mode, done, scale, name, description};
static void global(void *data, struct wl_registry *reg, uint32_t id, const char *interface, uint32_t version) {
    (void)data;
    if (!strcmp(interface, zwlr_virtual_pointer_manager_v1_interface.name)) manager = wl_registry_bind(reg, id, &zwlr_virtual_pointer_manager_v1_interface, version < 2 ? version : 2);
    if (!strcmp(interface, wl_output_interface.name) && count < 16) { int i = count++; outputs[i] = wl_registry_bind(reg, id, &wl_output_interface, version < 4 ? version : 4); wl_output_add_listener(outputs[i], &output_listener, (void *)(intptr_t)i); }
}
static void removed(void *d, struct wl_registry *r, uint32_t id) {(void)d;(void)r;(void)id;}
static const struct wl_registry_listener registry_listener = {global, removed};
int main(int argc, char **argv) {
    if (argc != 4) return 2;
    struct wl_display *display = wl_display_connect(NULL); if (!display) return 3;
    struct wl_registry *registry = wl_display_get_registry(display); wl_registry_add_listener(registry, &registry_listener, NULL);
    wl_display_roundtrip(display); wl_display_roundtrip(display);
    struct wl_output *output = NULL; for (int i = 0; i < count; i++) if (!strcmp(names[i], argv[1])) output = outputs[i];
    if (!manager || !output) return 4;
    struct zwlr_virtual_pointer_v1 *pointer = zwlr_virtual_pointer_manager_v1_create_virtual_pointer_with_output(manager, NULL, output);
    unsigned width = (unsigned)atoi(argv[2]), height = (unsigned)atoi(argv[3]), x, y, button, state;
    char line[128]; puts("ready"); fflush(stdout);
    while (fgets(line, sizeof(line), stdin)) {
        if (sscanf(line, "move %u %u", &x, &y) == 2) zwlr_virtual_pointer_v1_motion_absolute(pointer, 0, x, y, width, height);
        else if (sscanf(line, "button %u %u", &button, &state) == 2) zwlr_virtual_pointer_v1_button(pointer, 0, button, state);
        else break;
        zwlr_virtual_pointer_v1_frame(pointer); wl_display_flush(display); puts("ok"); fflush(stdout);
    }
    zwlr_virtual_pointer_v1_destroy(pointer); wl_display_disconnect(display); return 0;
}
