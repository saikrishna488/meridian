#include "meridian_surface.h"
#include <gtk/gtk.h>
#include <cstring>
#include <new>

namespace {
struct DesktopMenu {
    GtkWidget *parent;
    GtkWidget *popover = nullptr;
    meridian_desktop_callback callback;
    void *data;
    meridian_desktop_destroy destroy;
    GSimpleActionGroup *actions = nullptr;
    GtkEventController *gesture = nullptr;
    int pending = -1;
    ~DesktopMenu() { if (actions) g_object_unref(actions); if (destroy) destroy(data); }
};
constexpr const char *names[] = {"files", "new-folder", "refresh", "appearance", "settings"};
constexpr const char *labels[] = {"Open Files", "New Folder…", "Refresh Desktop", "Change Wallpaper…", "Desktop Settings"};
void closed(GtkPopover *, gpointer data) {
    auto *menu = static_cast<DesktopMenu *>(data);
    if (auto *popover = menu->popover) {
        menu->popover = nullptr;
        gtk_widget_unparent(popover);
    }
    // Popup grabs can consume the original button release. Reset the desktop
    // gesture so the next secondary click starts a fresh sequence.
    if (menu->gesture) gtk_event_controller_reset(menu->gesture);
    const int pending = menu->pending;
    menu->pending = -1;
    if (pending >= 0) menu->callback(static_cast<meridian_desktop_action>(pending), menu->data);
}
void unrealize(GtkWidget *, gpointer data) {
    static_cast<DesktopMenu *>(data)->pending = -1;
    closed(nullptr, data);
}
void activate(GSimpleAction *action, GVariant *, gpointer data) {
    auto *menu = static_cast<DesktopMenu *>(data);
    for (int i = 0; i < 5; ++i) {
        if (std::strcmp(g_action_get_name(G_ACTION(action)), names[i]) == 0) {
            // popdown can animate. Dispatch after "closed" releases the grab,
            // otherwise its focus restoration steals focus from the new dialog.
            if (menu->popover) {
                menu->pending = i;
                gtk_popover_popdown(GTK_POPOVER(menu->popover));
            } else { menu->callback(static_cast<meridian_desktop_action>(i), menu->data); }
            return;
        }
    }
}
void pressed(GtkGestureClick *gesture, int, double x, double y, gpointer data) {
    auto *menu = static_cast<DesktopMenu *>(data);
    if (menu->popover) closed(nullptr, menu);
    menu->popover = gtk_popover_new();
    auto *items = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    for (int i = 0; i < 5; ++i) {
        auto *button = gtk_button_new_with_label(labels[i]);
        gtk_widget_add_css_class(button, "flat");
        gtk_widget_set_halign(button, GTK_ALIGN_FILL);
        gtk_widget_set_halign(gtk_button_get_child(GTK_BUTTON(button)), GTK_ALIGN_START);
        g_object_set_data(G_OBJECT(button), "meridian-action-index", GINT_TO_POINTER(i));
        g_signal_connect(button, "clicked", G_CALLBACK(+[](GtkButton *button, gpointer data) {
            auto *menu = static_cast<DesktopMenu *>(data);
            const auto index = GPOINTER_TO_INT(g_object_get_data(G_OBJECT(button), "meridian-action-index"));
            auto *action = g_action_map_lookup_action(G_ACTION_MAP(menu->actions), names[index]);
            g_action_activate(action, nullptr);
        }), menu);
        gtk_box_append(GTK_BOX(items), button);
    }
    gtk_popover_set_child(GTK_POPOVER(menu->popover), items);
    gtk_widget_add_css_class(menu->popover, "mars-menu");
    gtk_widget_insert_action_group(menu->popover, "desktop", G_ACTION_GROUP(menu->actions));
    gtk_widget_set_parent(menu->popover, menu->parent);
    const GdkRectangle point{static_cast<int>(x), static_cast<int>(y), 1, 1};
    gtk_popover_set_pointing_to(GTK_POPOVER(menu->popover), &point);
    gtk_popover_set_has_arrow(GTK_POPOVER(menu->popover), FALSE);
    g_signal_connect(menu->popover, "closed", G_CALLBACK(closed), menu);
    (void)gesture;
    gtk_popover_popup(GTK_POPOVER(menu->popover));
    gtk_popover_present(GTK_POPOVER(menu->popover));
    gtk_widget_grab_focus(menu->popover);
}
}
extern "C" int meridian_desktop_menu_attach(void *pointer, meridian_desktop_callback callback,
                                            void *data, meridian_desktop_destroy destroy) {
    if (!pointer || !GTK_IS_WIDGET(pointer) || !callback) return 0;
    auto *widget = GTK_WIDGET(pointer);
    if (g_object_get_data(G_OBJECT(widget), "meridian-desktop-menu")) return 0;
    auto *menu = new (std::nothrow) DesktopMenu{widget, nullptr, callback, data, destroy};
    if (!menu) return 0;
    g_object_set_data_full(G_OBJECT(widget), "meridian-desktop-menu", menu,
        [](gpointer value) { delete static_cast<DesktopMenu *>(value); });
    auto *group = g_simple_action_group_new();
    menu->actions = group;
    for (const auto *name : names) {
        auto *action = g_simple_action_new(name, nullptr);
        g_signal_connect(action, "activate", G_CALLBACK(activate), menu);
        g_action_map_add_action(G_ACTION_MAP(group), G_ACTION(action));
        g_object_unref(action);
    }
    gtk_widget_insert_action_group(widget, "desktop", G_ACTION_GROUP(group));
    auto *gesture = gtk_gesture_click_new();
    menu->gesture = GTK_EVENT_CONTROLLER(gesture);
    gtk_gesture_single_set_button(GTK_GESTURE_SINGLE(gesture), GDK_BUTTON_SECONDARY);
    g_signal_connect(gesture, "pressed", G_CALLBACK(pressed), menu);
    gtk_widget_add_controller(widget, GTK_EVENT_CONTROLLER(gesture));
    g_signal_connect(widget, "unrealize", G_CALLBACK(unrealize), menu);
    return 1;
}
