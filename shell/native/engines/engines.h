#ifndef MERIDIAN_ENGINES_H
#define MERIDIAN_ENGINES_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Versioned, fixed-width C ABI. No ownership crosses the Rust/C++ boundary. */
typedef struct {
    int32_t hint_style; /* Cairo: none=1, slight=2, full=4 */
    int32_t acceleration; /* automatic=0, accelerated=1, software=2 */
    int32_t animations;
    int32_t transparency;
    int32_t icon_size;
} MeridianRenderPlan;
typedef struct {
    uint64_t memory_total_kib;
    uint64_t memory_available_kib;
    double memory_pressure_avg10; /* -1 when PSI is unavailable */
    uint32_t cpu_threads;
} MeridianEngineMetrics;
int meridian_audio_config(char *output, size_t capacity);
int meridian_cpu_activate(void);
int meridian_font_xml(int profile, const char *base, char *output, size_t capacity);
int meridian_font_activate(void);
int meridian_font_versions(char *output, size_t capacity);
MeridianRenderPlan meridian_graphics_plan(int font, int acceleration, int performance, int animations, int transparency);
int meridian_performance_sample(MeridianEngineMetrics *output);
typedef int (*MeridianWrite)(const unsigned char *, size_t, void *);
int meridian_graphics_preview(MeridianRenderPlan plan, int scale, MeridianWrite write, void *context);
#ifdef __cplusplus
}
#endif
#endif
