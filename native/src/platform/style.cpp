// Linux rendering for the Luminare-adapted MarsOSKit. See swift-ui/THIRD-PARTY.md.
#include "meridian_style.h"
#include <gtk/gtk.h>
#include <algorithm>

namespace {
constexpr const char *classes[] = {"mars-chrome", "mars-dock", "mars-section", "mars-panel",
    "mars-dock-item", "mars-control", "mars-primary", "mars-desktop-item"};
constexpr const char *css = R"CSS(
window.mars-layer { background-color: transparent; border-radius: 0; }
.mars-kit { font-family: Inter, "Noto Sans", sans-serif; font-size: 13px; color: @theme_fg_color; }
.mars-chrome { background-color: alpha(@theme_bg_color, 0.94); border-bottom: 1px solid alpha(@theme_fg_color, 0.12); }
.mars-dock { background-color: alpha(@theme_bg_color, 0.90); border: 1px solid alpha(@theme_fg_color, 0.16); border-radius: 20px; box-shadow: 0 4px 14px alpha(black, 0.18); }
.mars-section { background-color: alpha(@theme_base_color, 0.88); border: 1px solid alpha(@theme_fg_color, 0.10); border-radius: 12px; }
.mars-panel { background-color: @theme_bg_color; border-radius: 16px; }
button.mars-control, .mars-chrome button, .mars-panel button { background: transparent; border: none; box-shadow: none; border-radius: 7px; min-height: 24px; padding: 0 7px; }
button.mars-control:hover, .mars-chrome button:hover, .mars-panel button:hover { background-color: alpha(@theme_fg_color, 0.08); }
button.mars-control:active, .mars-chrome button:active, .mars-panel button:active { background-color: alpha(@theme_fg_color, 0.14); }
button.mars-primary { background: #5553ce; color: white; border: none; border-radius: 8px; box-shadow: none; }
button.mars-primary:hover { background: #6563de; }
button.mars-primary:active { background: #4543b8; }
button.mars-dock-item { background: transparent; border: none; border-radius: 13px; box-shadow: none; padding: 0; }
button.mars-dock-item:hover { background-color: alpha(@theme_fg_color, 0.10); }
button.mars-dock-item:active { background-color: alpha(@theme_fg_color, 0.18); }
button.mars-desktop-item { background: transparent; border: 1px solid transparent; border-radius: 8px; box-shadow: none; padding: 4px; }
button.mars-desktop-item:hover { background: alpha(white, 0.12); }
button.mars-desktop-item:active { background: alpha(white, 0.20); }
.mars-kit button:focus-visible { outline: 2px solid #8a88ed; outline-offset: 2px; }
.mars-kit button:disabled { opacity: 0.45; }
.mars-menu contents, .mars-kit popover contents { background-color: alpha(@theme_bg_color, 0.98); border: 1px solid alpha(@theme_fg_color, 0.16); border-radius: 12px; padding: 5px; box-shadow: 0 6px 18px alpha(black, 0.18); }
.mars-menu modelbutton, .mars-menu button, .mars-kit popover modelbutton { border-radius: 6px; min-height: 27px; padding: 0 10px; }
.mars-menu modelbutton:hover, .mars-menu button:hover, .mars-kit popover modelbutton:hover { background-color: #5553ce; color: white; }
)CSS";
void install(GtkWidget *widget) {
    auto *display = gtk_widget_get_display(widget);
    if (g_object_get_data(G_OBJECT(display), "mars-kit-style")) return;
    auto *provider = gtk_css_provider_new();
    gtk_css_provider_load_from_string(provider, css);
    gtk_style_context_add_provider_for_display(display, GTK_STYLE_PROVIDER(provider), GTK_STYLE_PROVIDER_PRIORITY_APPLICATION + 1);
    g_object_set_data_full(G_OBJECT(display), "mars-kit-style", provider, g_object_unref);
}
}
extern "C" int meridian_widget_style(void *pointer, meridian_style_role role) {
    if (!pointer || !GTK_IS_WIDGET(pointer) || role < MERIDIAN_STYLE_CHROME || role > MERIDIAN_STYLE_DESKTOP_ITEM) return 0;
    auto *widget = GTK_WIDGET(pointer);
    install(widget);
    gtk_widget_add_css_class(widget, "mars-kit");
    gtk_widget_add_css_class(widget, classes[role]);
    return 1;
}
extern "C" void meridian_widget_appearance(void *pointer, int dark) {
    if (!pointer || !GTK_IS_WIDGET(pointer)) return;
    auto *settings = gtk_settings_get_for_display(gtk_widget_get_display(GTK_WIDGET(pointer)));
    if (!settings) return;
    gboolean current = FALSE;
    g_object_get(settings, "gtk-application-prefer-dark-theme", &current, nullptr);
    const gboolean requested = dark != 0;
    if (current != requested) g_object_set(settings, "gtk-application-prefer-dark-theme", requested, nullptr);
}
extern "C" void meridian_widget_describe(void *pointer, const char *label) {
    if (!pointer || !GTK_IS_WIDGET(pointer) || !label) return;
    gtk_widget_set_tooltip_text(GTK_WIDGET(pointer), label);
    gtk_accessible_update_property(GTK_ACCESSIBLE(pointer), GTK_ACCESSIBLE_PROPERTY_LABEL, label, -1);
}
extern "C" void meridian_image_icon(void *pointer, const char *serialized, int pixels) {
    if (!pointer || !GTK_IS_IMAGE(pointer)) return;
    GError *error = nullptr;
    GIcon *icon = serialized ? g_icon_new_for_string(serialized, &error) : nullptr;
    if (error) g_error_free(error);
    if (!icon) icon = g_themed_icon_new("application-x-executable");
    gtk_image_set_from_gicon(GTK_IMAGE(pointer), icon);
    gtk_image_set_pixel_size(GTK_IMAGE(pointer), std::max(12, std::min(pixels, 96)));
    g_object_unref(icon);
}
