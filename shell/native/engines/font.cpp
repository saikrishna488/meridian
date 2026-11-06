#include "engines.h"
#include <fontconfig/fontconfig.h>
#include <ft2build.h>
#include FT_FREETYPE_H
#include <pango/pango.h>
#include <cairo.h>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

extern "C" int meridian_font_xml(int profile, const char *base, char *output, size_t capacity) {
    if (!base || !output || profile < 0 || profile > 2) return -1;
    try {
        std::string escaped;
        for (const char *p = base; *p; ++p) {
            switch (*p) {
            case '&': escaped += "&amp;"; break;
            case '<': escaped += "&lt;"; break;
            case '>': escaped += "&gt;"; break;
            case '"': escaped += "&quot;"; break;
            default: escaped += *p;
            }
        }
        const char *hint = profile == 1 ? "hintfull" : profile == 2 ? "hintnone" : "hintslight";
        const std::string xml = "<?xml version=\"1.0\"?>\n<!DOCTYPE fontconfig SYSTEM \"urn:fontconfig:fonts.dtd\">\n"
            "<fontconfig><include ignore_missing=\"no\">" + escaped + "</include>"
            "<match target=\"font\"><edit name=\"antialias\" mode=\"assign\"><bool>true</bool></edit>"
            "<edit name=\"hinting\" mode=\"assign\"><bool>" + std::string(profile == 2 ? "false" : "true") + "</bool></edit>"
            "<edit name=\"hintstyle\" mode=\"assign\"><const>" + hint + "</const></edit>"
            "<edit name=\"rgba\" mode=\"assign\"><const>none</const></edit>"
            "</match></fontconfig>\n";
        if (xml.size() + 1 > capacity) return -1;
        std::memcpy(output, xml.c_str(), xml.size() + 1);
        return static_cast<int>(xml.size());
    } catch (...) { return -1; }
}

extern "C" int meridian_font_activate(void) {
    FcConfig *config = FcConfigCreate();
    if (!config) return -1;
    const auto *path = reinterpret_cast<const FcChar8 *>(std::getenv("FONTCONFIG_FILE"));
    const bool result = FcConfigParseAndLoad(config, path, FcTrue) && FcConfigBuildFonts(config) && FcConfigSetCurrent(config);
    FcConfigDestroy(config);
    return result ? 0 : -1;
}

extern "C" int meridian_font_versions(char *output, size_t capacity) {
    if (!output || !capacity) return -1;
    FT_Library library = nullptr;
    if (FT_Init_FreeType(&library)) return -1;
    FT_Int major = 0, minor = 0, patch = 0;
    FT_Library_Version(library, &major, &minor, &patch);
    FT_Done_FreeType(library);
    const int fc = FcGetVersion();
    const int count = std::snprintf(output, capacity, "FreeType %d.%d.%d · Fontconfig %d.%d.%d · Pango %s · Cairo %s", major, minor, patch,
                                  fc / 10000, (fc / 100) % 100, fc % 100, pango_version_string(), cairo_version_string());
    return count >= 0 && static_cast<size_t>(count) < capacity ? count : -1;
}
