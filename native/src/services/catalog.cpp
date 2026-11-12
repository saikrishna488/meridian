#include "meridian.h"
#include "services/fuzzy.hpp"
#include <gio/gdesktopappinfo.h>
#include <algorithm>
#include <cstring>
#include <memory>
#include <string>
#include <vector>

namespace {
struct Unref { void operator()(GAppInfo *p) const noexcept { g_object_unref(p); } };
using AppInfo = std::unique_ptr<GAppInfo, Unref>;
struct Entry {
    AppInfo info;
    std::string id, name, icon, search, generic_name, description, source, wm_class;
    meridian::search::Fields fields;
};
std::string folded(const char *text) {
    if (!text) return {};
    std::unique_ptr<char, decltype(&g_free)> value(g_utf8_casefold(text, -1), g_free);
    return value ? value.get() : "";
}
struct FreeList {
    void operator()(GList *list) const noexcept { g_list_free_full(list, g_object_unref); }
};
void message(char *out, size_t size, const char *text) noexcept {
    if (!out || size == 0) return;
    const auto count = std::min(size - 1, std::strlen(text));
    std::memcpy(out, text, count);
    out[count] = '\0';
}
}
struct meridian_catalog { std::vector<Entry> entries; };
extern "C" {
meridian_catalog *meridian_catalog_create(void) {
    try {
        auto catalog = std::make_unique<meridian_catalog>();
        std::unique_ptr<GList, FreeList> apps(g_app_info_get_all());
        for (auto *node = apps.get(); node; node = node->next) {
            auto *app = G_APP_INFO(node->data);
            const auto *id = g_app_info_get_id(app);
            const auto *name = g_app_info_get_display_name(app);
            if (!G_IS_DESKTOP_APP_INFO(app) || !id || !name || !g_app_info_should_show(app)) continue;
            AppInfo owned(G_APP_INFO(g_object_ref(app)));
            auto *desktop = G_DESKTOP_APP_INFO(app);
            std::string search = folded(name) + '\n' + folded(id) + '\n'
                + folded(g_app_info_get_description(app)) + '\n'
                + folded(g_desktop_app_info_get_generic_name(desktop));
            const auto *keywords = g_desktop_app_info_get_keywords(desktop);
            if (keywords) for (size_t i = 0; keywords[i]; ++i)
                search += '\n' + folded(keywords[i]);
            auto *icon = g_app_info_get_icon(app);
            std::unique_ptr<char, decltype(&g_free)> serialized(icon ? g_icon_to_string(icon) : nullptr, g_free);
            const auto text = [](const char *value) { return value ? std::string(value) : std::string{}; };
            auto generic = text(g_desktop_app_info_get_generic_name(desktop));
            auto description = text(g_app_info_get_description(app));
            meridian::search::Fields fields;
            fields.name = meridian::search::Prepared(name);
            fields.description = meridian::search::Prepared(description);
            if (!generic.empty()) fields.aliases.emplace_back(generic);
            if (keywords) for (size_t i = 0; keywords[i]; ++i) fields.aliases.emplace_back(keywords[i]);
            catalog->entries.push_back({std::move(owned), id, name,
                serialized ? serialized.get() : "", std::move(search), std::move(generic), std::move(description),
                text(g_desktop_app_info_get_filename(desktop)), text(g_desktop_app_info_get_startup_wm_class(desktop)), std::move(fields)});
        }
        std::sort(catalog->entries.begin(), catalog->entries.end(), [](const Entry &a, const Entry &b) {
            const auto comparison = g_utf8_collate(a.name.c_str(), b.name.c_str());
            return comparison == 0 ? a.id < b.id : comparison < 0;
        });
        return catalog.release();
    } catch (...) { return nullptr; }
}
void meridian_catalog_destroy(meridian_catalog *catalog) { delete catalog; }
size_t meridian_catalog_count(const meridian_catalog *catalog) { return catalog ? catalog->entries.size() : 0; }
const char *meridian_catalog_id(const meridian_catalog *catalog, size_t index) {
    return catalog && index < catalog->entries.size() ? catalog->entries[index].id.c_str() : nullptr;
}
const char *meridian_catalog_name(const meridian_catalog *catalog, size_t index) {
    return catalog && index < catalog->entries.size() ? catalog->entries[index].name.c_str() : nullptr;
}
const char *meridian_catalog_icon(const meridian_catalog *catalog, size_t index) {
    if (!catalog || index >= catalog->entries.size()) return nullptr;
    const auto &icon = catalog->entries[index].icon;
    return icon.empty() ? nullptr : icon.c_str();
}
const char *meridian_catalog_generic_name(const meridian_catalog *catalog, size_t index) {
    return catalog && index < catalog->entries.size() && !catalog->entries[index].generic_name.empty()
        ? catalog->entries[index].generic_name.c_str() : nullptr;
}
const char *meridian_catalog_description(const meridian_catalog *catalog, size_t index) {
    return catalog && index < catalog->entries.size() && !catalog->entries[index].description.empty()
        ? catalog->entries[index].description.c_str() : nullptr;
}
const char *meridian_catalog_source(const meridian_catalog *catalog, size_t index) {
    return catalog && index < catalog->entries.size() && !catalog->entries[index].source.empty()
        ? catalog->entries[index].source.c_str() : nullptr;
}
const char *meridian_catalog_wm_class(const meridian_catalog *catalog, size_t index) {
    return catalog && index < catalog->entries.size() && !catalog->entries[index].wm_class.empty()
        ? catalog->entries[index].wm_class.c_str() : nullptr;
}
size_t meridian_catalog_query(const meridian_catalog *catalog, const char *query, size_t *indices, size_t capacity) {
    if (!catalog || !query || (capacity && !indices)) return 0;
    const auto length = strnlen(query, 4097);
    if (length > 4096 || !g_utf8_validate(query, static_cast<gssize>(length), nullptr)) return 0;
    try {
        const auto tokens = meridian::search::tokens(query);
        std::vector<std::pair<int, size_t>> matches;
        for (size_t i = 0; i < catalog->entries.size(); ++i)
            if (const auto score = catalog->entries[i].fields.score(tokens)) matches.emplace_back(*score, i);
        std::stable_sort(matches.begin(), matches.end(), [](const auto &a, const auto &b) { return a.first > b.first; });
        for (size_t i = 0; i < std::min(capacity, matches.size()); ++i) indices[i] = matches[i].second;
        return matches.size();
    } catch (...) { return 0; }
}
size_t meridian_catalog_search(const meridian_catalog *catalog, const char *query,
                               size_t *indices, size_t capacity) {
    if (!catalog || !query || (capacity && !indices)) return 0;
    const auto length = strnlen(query, 4097);
    if (length > 4096 || !g_utf8_validate(query, static_cast<gssize>(length), nullptr)) return 0;
    try {
        const auto key = folded(query);
        size_t count = 0;
        for (size_t i = 0; i < catalog->entries.size(); ++i) {
            if (catalog->entries[i].search.find(key) == std::string::npos) continue;
            if (count < capacity) indices[count] = i;
            ++count;
        }
        return count;
    } catch (...) { return 0; }
}
int meridian_catalog_launch(meridian_catalog *catalog, const char *id, char *error, size_t capacity) {
    message(error, capacity, "");
    if (!catalog || !id) { message(error, capacity, "Invalid application catalog or ID."); return 0; }
    for (const auto &entry : catalog->entries) {
        if (entry.id != id) continue;
        GError *failure = nullptr;
        auto *context = g_app_launch_context_new();
        const bool launched = g_app_info_launch(entry.info.get(), nullptr, context, &failure);
        g_object_unref(context);
        if (!launched) message(error, capacity, failure ? failure->message : "Application launch failed.");
        g_clear_error(&failure);
        return launched ? 1 : 0;
    }
    message(error, capacity, "Application is not in the catalog.");
    return 0;
}
}
