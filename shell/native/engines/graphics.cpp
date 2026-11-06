#include "engines.h"
#include <cairo.h>
#include <pango/pangocairo.h>

extern "C" MeridianRenderPlan meridian_graphics_plan(int font, int acceleration, int performance, int animations, int transparency) {
    MeridianRenderPlan result = {};
    result.hint_style = font == 1 ? CAIRO_HINT_STYLE_FULL : font == 2 ? CAIRO_HINT_STYLE_NONE : CAIRO_HINT_STYLE_SLIGHT;
    result.acceleration = acceleration >= 0 && acceleration <= 2 ? acceleration : 0;
    result.animations = animations && performance != 2;
    result.transparency = transparency && performance != 2;
    // Keep even the efficient profile above normal dock icon display sizes.
    result.icon_size = performance == 0 ? 192 : performance == 2 ? 96 : 128;
    return result;
}

struct Sink { MeridianWrite write; void *context; };
static cairo_status_t emit(void *closure, const unsigned char *data, unsigned int length) {
    auto *sink = static_cast<Sink *>(closure);
    return sink->write(data, length, sink->context) == 0 ? CAIRO_STATUS_SUCCESS : CAIRO_STATUS_WRITE_ERROR;
}

extern "C" int meridian_graphics_preview(MeridianRenderPlan plan, int scale, MeridianWrite write, void *context) {
    if (!write || scale < 1 || scale > 3) return -1;
    if (plan.hint_style != CAIRO_HINT_STYLE_NONE && plan.hint_style != CAIRO_HINT_STYLE_SLIGHT && plan.hint_style != CAIRO_HINT_STYLE_FULL) return -1;
    cairo_surface_t *surface = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, 600 * scale, 180 * scale);
    if (cairo_surface_status(surface) != CAIRO_STATUS_SUCCESS) { cairo_surface_destroy(surface); return -1; }
    cairo_surface_set_device_scale(surface, scale, scale);
    cairo_t *cr = cairo_create(surface);
    if (cairo_status(cr) != CAIRO_STATUS_SUCCESS) { cairo_destroy(cr); cairo_surface_destroy(surface); return -1; }
    cairo_set_source_rgb(cr, .035, .045, .065); cairo_paint(cr);
    cairo_font_options_t *options = cairo_font_options_create();
    if (cairo_font_options_status(options) != CAIRO_STATUS_SUCCESS) {
        cairo_font_options_destroy(options); cairo_destroy(cr); cairo_surface_destroy(surface); return -1;
    }
    cairo_font_options_set_antialias(options, CAIRO_ANTIALIAS_GRAY);
    cairo_font_options_set_hint_style(options, static_cast<cairo_hint_style_t>(plan.hint_style));
    cairo_font_options_set_hint_metrics(options, CAIRO_HINT_METRICS_ON);
    PangoLayout *layout = pango_cairo_create_layout(cr);
    if (!layout) { cairo_font_options_destroy(options); cairo_destroy(cr); cairo_surface_destroy(surface); return -1; }
    pango_cairo_context_set_font_options(pango_layout_get_context(layout), options);
    PangoFontDescription *font = pango_font_description_from_string("Inter 18");
    if (!font) { g_object_unref(layout); cairo_font_options_destroy(options); cairo_destroy(cr); cairo_surface_destroy(surface); return -1; }
    pango_layout_set_font_description(layout, font);
    pango_layout_set_text(layout, "Meridian · Crisp by design", -1);
    cairo_set_source_rgb(cr, .95, .96, 1); cairo_move_to(cr, 24, 20); pango_cairo_show_layout(cr, layout);
    pango_font_description_set_size(font, 11 * PANGO_SCALE);
    pango_layout_set_font_description(layout, font);
    pango_layout_set_text(layout, "Aa Bb Cc 0123456789 · العربية · 日本語\nNative text shaping, curves, and pixel-aligned lines", -1);
    cairo_set_source_rgb(cr, .7, .75, .83); cairo_move_to(cr, 24, 62); pango_cairo_show_layout(cr, layout);
    // Pixel alignment happens at the backing-store scale, not just logical CSS pixels.
    cairo_set_line_width(cr, 1.0 / scale);
    cairo_set_source_rgb(cr, .3, .65, 1);
    for (int i = 0; i < 12; ++i) {
        const double x = (24 + i * 10) + .5 / scale;
        cairo_move_to(cr, x, 128); cairo_line_to(cr, x, 158);
    }
    cairo_stroke(cr);
    for (int i = 0; i < 4; ++i) {
        cairo_arc(cr, 220 + i * 55, 143, 8 + i * 3, 0, 6.283185307179586);
        cairo_stroke(cr);
    }
    const cairo_status_t draw_status = cairo_status(cr);
    Sink sink{write, context};
    const cairo_status_t png_status = draw_status == CAIRO_STATUS_SUCCESS ? cairo_surface_write_to_png_stream(surface, emit, &sink) : draw_status;
    pango_font_description_free(font); g_object_unref(layout); cairo_font_options_destroy(options);
    cairo_destroy(cr); cairo_surface_destroy(surface);
    return png_status == CAIRO_STATUS_SUCCESS ? 0 : -1;
}
