#include "engines.h"
#include <cstdio>
#include <cstring>
// PipeWire performs the actual SIMD resampling. Keep native policy code small,
// avoid forced rates/volume/EQ and retain the device's negotiated audio format.
extern "C" int meridian_audio_config(char *output, size_t capacity) {
    constexpr char config[] = "# Meridian audio engine: managed high-quality resampling\n"
        "stream.properties = {\n    resample.quality = 10\n}\n";
    if (!output || capacity < sizeof(config)) return -1;
    std::memcpy(output, config, sizeof(config));
    return static_cast<int>(sizeof(config) - 1);
}
