/* An independent application window used only by the disposable Wayland test. */
#include <gtk/gtk.h>
static const char *ready_path;
static void mapped(GtkWidget *widget, gpointer data) {
    (void)widget; (void)data;
    g_file_set_contents(ready_path, "mapped", -1, NULL);
}
static void activate(GtkApplication *app, gpointer data) {
    (void)data;
    GtkWidget *window = gtk_application_window_new(app);
    gtk_window_set_title(GTK_WINDOW(window), "Meridian survival test");
    gtk_window_set_default_size(GTK_WINDOW(window), 240, 120);
    g_signal_connect(window, "map", G_CALLBACK(mapped), NULL);
    gtk_window_present(GTK_WINDOW(window));
}
int main(int argc, char **argv) {
    if (argc != 2) return 2;
    ready_path = argv[1];
    GtkApplication *app = gtk_application_new("org.meridian.Survivor", G_APPLICATION_NON_UNIQUE);
    g_signal_connect(app, "activate", G_CALLBACK(activate), NULL);
    int code = g_application_run(G_APPLICATION(app), 1, argv);
    g_object_unref(app);
    return code;
}
