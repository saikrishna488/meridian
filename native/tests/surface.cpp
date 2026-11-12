#include "meridian_surface.h"
#include <gtk/gtk.h>
#include <gtk4-layer-shell.h>
#include <cstring>
#include <cstdlib>
#include <iostream>
#include <stdexcept>
#include <source_location>

void require(bool value, std::source_location location = std::source_location::current()) {
    if (!value) throw std::runtime_error("surface contract failed at line " + std::to_string(location.line()));
}
int main() {
    char error[128];
    require(!meridian_surface_applications(nullptr, error, sizeof error));
    require(error[0] != '\0');
    if (!std::getenv("MERIDIAN_NATIVE_WAYLAND_TEST") || !gtk_init_check() || !gtk_layer_is_supported()) {
        std::cout << "SKIP: run tools/tests/native-wayland.py under headless labwc\n";
        return 77;
    }
    for (int iteration = 0; iteration < 3; ++iteration) {
        auto *window = GTK_WINDOW(gtk_window_new());
        g_object_ref_sink(window);
        require(meridian_surface_applications(window, error, sizeof error));
        require(error[0] == '\0');
        require(gtk_layer_is_layer_window(window));
        require(std::strcmp(gtk_layer_get_namespace(window), "meridian-native-applications") == 0);
        require(gtk_layer_get_layer(window) == GTK_LAYER_SHELL_LAYER_OVERLAY);
        require(gtk_layer_get_exclusive_zone(window) == -1);
        require(gtk_layer_get_keyboard_mode(window) == GTK_LAYER_SHELL_KEYBOARD_MODE_EXCLUSIVE);
        require(gtk_layer_get_anchor(window, GTK_LAYER_SHELL_EDGE_TOP));
        require(gtk_layer_get_anchor(window, GTK_LAYER_SHELL_EDGE_BOTTOM));
        require(gtk_layer_get_anchor(window, GTK_LAYER_SHELL_EDGE_LEFT));
        require(gtk_layer_get_anchor(window, GTK_LAYER_SHELL_EDGE_RIGHT));
        require(meridian_surface_applications(window, error, sizeof error));
        gtk_window_present(window);
        const auto deadline = g_get_monotonic_time() + 3 * G_TIME_SPAN_SECOND;
        while (!gtk_widget_get_mapped(GTK_WIDGET(window)) && g_get_monotonic_time() < deadline) {
            g_main_context_iteration(nullptr, FALSE);
            g_usleep(1000);
        }
        require(gtk_widget_get_mapped(GTK_WIDGET(window)));
        require(meridian_surface_applications(window, error, sizeof error));
        gtk_window_destroy(window);
        g_object_unref(window);
        while (g_main_context_iteration(nullptr, FALSE)) {}
    }
    for (const auto role : {MERIDIAN_SURFACE_WALLPAPER, MERIDIAN_SURFACE_TOPBAR, MERIDIAN_SURFACE_DOCK}) {
        auto *window = GTK_WINDOW(gtk_window_new());
        g_object_ref_sink(window);
        require(meridian_surface_configure(window, role, error, sizeof error));
        require(meridian_surface_configure(window, role, error, sizeof error));
        require(!meridian_surface_applications(window, error, sizeof error));
        const bool wallpaper = role == MERIDIAN_SURFACE_WALLPAPER;
        require(gtk_layer_get_layer(window) == (wallpaper ? GTK_LAYER_SHELL_LAYER_BACKGROUND : GTK_LAYER_SHELL_LAYER_TOP));
        require(gtk_layer_get_keyboard_mode(window) == (wallpaper ? GTK_LAYER_SHELL_KEYBOARD_MODE_NONE : GTK_LAYER_SHELL_KEYBOARD_MODE_ON_DEMAND));
        require(gtk_layer_get_anchor(window, GTK_LAYER_SHELL_EDGE_TOP) == (role != MERIDIAN_SURFACE_DOCK));
        require(gtk_layer_get_anchor(window, GTK_LAYER_SHELL_EDGE_BOTTOM) == (role != MERIDIAN_SURFACE_TOPBAR));
        gtk_window_present(window);
        const auto deadline = g_get_monotonic_time() + 3 * G_TIME_SPAN_SECOND;
        while (!gtk_widget_get_mapped(GTK_WIDGET(window)) && g_get_monotonic_time() < deadline) {
            g_main_context_iteration(nullptr, FALSE);
            g_usleep(1000);
        }
        require(gtk_widget_get_mapped(GTK_WIDGET(window)));
        if (!wallpaper) {
            while (gtk_layer_get_exclusive_zone(window) <= 0 && g_get_monotonic_time() < deadline) {
                g_main_context_iteration(nullptr, FALSE);
                g_usleep(1000);
            }
            require(gtk_layer_get_exclusive_zone(window) > 0);
        }
        gtk_window_destroy(window);
        g_object_unref(window);
        while (g_main_context_iteration(nullptr, FALSE)) {}
    }
    auto *late = GTK_WINDOW(gtk_window_new());
    g_object_ref_sink(late);
    gtk_widget_realize(GTK_WIDGET(late));
    require(!meridian_surface_applications(late, error, sizeof error));
    require(error[0] != '\0');
    gtk_window_destroy(late);
    g_object_unref(late);
    std::cout << "Overlay, wallpaper, top bar and dock configuration, map, teardown and reopen passed\n";
}
