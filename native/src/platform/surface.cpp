#include "meridian_surface.h"
#include <gtk/gtk.h>
#include <gtk4-layer-shell.h>
#include <cstdio>
#include <initializer_list>

namespace {
int fail(char *error, size_t capacity, const char *message) noexcept {
    if (error && capacity) std::snprintf(error, capacity, "%s", message);
    return 0;
}
gboolean key_pressed(GtkEventControllerKey *, guint key, guint, GdkModifierType, gpointer window) {
    if (key != GDK_KEY_Escape) return FALSE;
    gtk_window_close(GTK_WINDOW(window));
    return TRUE;
}
}
extern "C" int meridian_surface_configure(void *pointer, meridian_surface_role role, char *error, size_t capacity) {
    if (role < MERIDIAN_SURFACE_APPLICATIONS || role > MERIDIAN_SURFACE_DOCK)
        return fail(error, capacity, "Unknown surface role.");
    if (error && capacity) error[0] = '\0';
    if (!pointer || !GTK_IS_WINDOW(pointer)) return fail(error, capacity, "A GTK window is required.");
    auto *window = GTK_WINDOW(pointer);
    if (const auto configured = g_object_get_data(G_OBJECT(window), "meridian-surface-role")) {
        if (GPOINTER_TO_INT(configured) == static_cast<int>(role) + 1) return 1;
        return fail(error, capacity, "Surface already has a different role.");
    }
    if (gtk_widget_get_realized(GTK_WIDGET(window))) return fail(error, capacity, "Surface must be configured before realization.");
    if (!gtk_layer_is_supported()) return fail(error, capacity, "The display does not support Wayland layer-shell.");
    if (gtk_layer_is_layer_window(window)) return fail(error, capacity, "Surface is already configured.");
    gtk_window_set_decorated(window, FALSE);
    gtk_layer_init_for_window(window);
    const char *names[] = {"meridian-native-applications", "meridian-native-wallpaper", "meridian-native-topbar", "meridian-native-dock"};
    gtk_layer_set_namespace(window, names[role]);
    const bool overlay = role == MERIDIAN_SURFACE_APPLICATIONS;
    const bool wallpaper = role == MERIDIAN_SURFACE_WALLPAPER;
    gtk_layer_set_layer(window, overlay ? GTK_LAYER_SHELL_LAYER_OVERLAY
        : wallpaper ? GTK_LAYER_SHELL_LAYER_BACKGROUND : GTK_LAYER_SHELL_LAYER_TOP);
    gtk_layer_set_keyboard_mode(window, overlay ? GTK_LAYER_SHELL_KEYBOARD_MODE_EXCLUSIVE
        : wallpaper ? GTK_LAYER_SHELL_KEYBOARD_MODE_NONE : GTK_LAYER_SHELL_KEYBOARD_MODE_ON_DEMAND);
    gtk_layer_set_exclusive_zone(window, -1);
    if (overlay || wallpaper || role == MERIDIAN_SURFACE_TOPBAR) {
        gtk_layer_set_anchor(window, GTK_LAYER_SHELL_EDGE_LEFT, TRUE);
        gtk_layer_set_anchor(window, GTK_LAYER_SHELL_EDGE_RIGHT, TRUE);
    }
    gtk_layer_set_anchor(window, GTK_LAYER_SHELL_EDGE_TOP, overlay || wallpaper || role == MERIDIAN_SURFACE_TOPBAR);
    gtk_layer_set_anchor(window, GTK_LAYER_SHELL_EDGE_BOTTOM, overlay || wallpaper || role == MERIDIAN_SURFACE_DOCK);
    if (!overlay && !wallpaper) gtk_layer_auto_exclusive_zone_enable(window);
    if (overlay) {
        auto *keys = gtk_event_controller_key_new();
        g_signal_connect(keys, "key-pressed", G_CALLBACK(key_pressed), window);
        gtk_widget_add_controller(GTK_WIDGET(window), keys);
    }
    g_object_set_data(G_OBJECT(window), "meridian-surface-role", GINT_TO_POINTER(static_cast<int>(role) + 1));
    return 1;
}

extern "C" int meridian_surface_applications(void *pointer, char *error, size_t capacity) {
    return meridian_surface_configure(pointer, MERIDIAN_SURFACE_APPLICATIONS, error, capacity);
}
