#ifndef MERIDIAN_STYLE_H
#define MERIDIAN_STYLE_H
#ifdef __cplusplus
extern "C" {
#endif
enum meridian_style_role {
    MERIDIAN_STYLE_CHROME, MERIDIAN_STYLE_DOCK, MERIDIAN_STYLE_SECTION,
    MERIDIAN_STYLE_PANEL, MERIDIAN_STYLE_DOCK_ITEM, MERIDIAN_STYLE_CONTROL,
    MERIDIAN_STYLE_PRIMARY, MERIDIAN_STYLE_DESKTOP_ITEM
};
/* GTK main thread; fixed local stylesheet only, installed once per display. */
int meridian_widget_style(void *widget, enum meridian_style_role role);
void meridian_widget_appearance(void *widget, int dark);
void meridian_widget_describe(void *widget, const char *label);
/* Serialized GIcon or icon-theme name. Falls back to application-x-executable. */
void meridian_image_icon(void *image, const char *icon, int pixels);
#ifdef __cplusplus
}
#endif
#endif
