#include "meridian_surface.h"
#include "meridian_style.h"
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
        gtk_window_set_default_size(window, 240, 48);
        gtk_window_set_child(window, gtk_label_new("Surface contract"));
        require(gtk_layer_get_layer(window) == (wallpaper ? GTK_LAYER_SHELL_LAYER_BACKGROUND : GTK_LAYER_SHELL_LAYER_TOP));
        require(gtk_layer_get_keyboard_mode(window) == GTK_LAYER_SHELL_KEYBOARD_MODE_ON_DEMAND);
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
            if (gtk_layer_get_exclusive_zone(window) <= 0)
                std::cerr << "role=" << role << " height=" << gtk_widget_get_height(GTK_WIDGET(window))
                          << " zone=" << gtk_layer_get_exclusive_zone(window) << '\n';
            require(gtk_layer_get_exclusive_zone(window) > 0);
        }
        gtk_window_destroy(window);
        g_object_unref(window);
        while (g_main_context_iteration(nullptr, FALSE)) {}
    }
    struct MenuProbe { int actions = 0; int destroyed = 0; } probe;
    require(!meridian_widget_style(nullptr, MERIDIAN_STYLE_CHROME));
    auto *icon = gtk_image_new();
    g_object_ref_sink(icon);
    meridian_image_icon(icon, "folder", 44);
    require(gtk_image_get_pixel_size(GTK_IMAGE(icon)) == 44);
    meridian_image_icon(icon, nullptr, 1000);
    require(gtk_image_get_pixel_size(GTK_IMAGE(icon)) == 96);
    g_object_unref(icon);
    auto *target = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    g_object_ref_sink(target);
    require(meridian_widget_style(target, MERIDIAN_STYLE_SECTION));
    require(gtk_widget_has_css_class(target, "mars-kit"));
    require(gtk_widget_has_css_class(target, "mars-section"));
    const auto callback = +[](meridian_desktop_action action, void *data) {
        auto *probe = static_cast<MenuProbe *>(data);
        require(action == MERIDIAN_DESKTOP_REFRESH);
        ++probe->actions;
    };
    const auto destroy = +[](void *data) { ++static_cast<MenuProbe *>(data)->destroyed; };
    require(!meridian_desktop_menu_attach(nullptr, callback, &probe, destroy));
    require(meridian_desktop_menu_attach(target, callback, &probe, destroy));
    require(!meridian_desktop_menu_attach(target, callback, &probe, destroy));
    require(gtk_widget_activate_action(target, "desktop.refresh", nullptr));
    require(probe.actions == 1 && probe.destroyed == 0);
    g_object_unref(target);
    require(probe.destroyed == 1);
    auto *late = GTK_WINDOW(gtk_window_new());
    g_object_ref_sink(late);
    gtk_widget_realize(GTK_WIDGET(late));
    require(!meridian_surface_applications(late, error, sizeof error));
    require(error[0] != '\0');
    gtk_window_destroy(late);
    g_object_unref(late);
    std::cout << "Overlay, wallpaper, top bar and dock configuration, map, teardown and reopen passed\n";
}
