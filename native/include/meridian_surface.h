#ifndef MERIDIAN_SURFACE_H
#define MERIDIAN_SURFACE_H
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Configure a borrowed GtkWindow before realization on the GTK main thread.
 * Returns 0 with an error if the display lacks layer-shell or it is too late.
 * Repeated calls for a window configured here are harmless, even after mapping.
 * Never use this for authentication: lockers require ext-session-lock-v1. */
enum meridian_surface_role {
    MERIDIAN_SURFACE_APPLICATIONS = 0,
    MERIDIAN_SURFACE_WALLPAPER = 1,
    MERIDIAN_SURFACE_TOPBAR = 2,
    MERIDIAN_SURFACE_DOCK = 3
};
/* Bars reserve their actual height; desktop and bars take focus on interaction.
 * Applies to the compositor-selected output. Multi-output hosting is pending. */
int meridian_surface_configure(void *gtk_window, enum meridian_surface_role role, char *error, size_t capacity);
enum meridian_desktop_action {
    MERIDIAN_DESKTOP_FILES = 0,
    MERIDIAN_DESKTOP_NEW_FOLDER = 1,
    MERIDIAN_DESKTOP_REFRESH = 2,
    MERIDIAN_DESKTOP_APPEARANCE = 3,
    MERIDIAN_DESKTOP_SETTINGS = 4
};
typedef void (*meridian_desktop_callback)(enum meridian_desktop_action, void *);
typedef void (*meridian_desktop_destroy)(void *);
/* GTK main thread only. Attaches a pointer-positioned secondary-click menu.
 * On success takes ownership of data until widget destruction; on failure the
 * caller retains ownership. Reinstallation is rejected. Popup cleanup follows
 * widget unrealization. Never attach to authentication surfaces. */
int meridian_desktop_menu_attach(void *gtk_widget, meridian_desktop_callback callback,
                                void *data, meridian_desktop_destroy destroy);
int meridian_surface_applications(void *gtk_window, char *error, size_t capacity);
#ifdef __cplusplus
}
#endif
#endif
