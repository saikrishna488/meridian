#include "services/services.hpp"
#include <algorithm>
#include <glib.h>

namespace meridian {
namespace {
Json checked_load(ConfigStore &store, const char *file, std::string_view type, const Json &fallback, bool versioned = false) {
    const auto value = store.load(file, fallback, versioned);
    try { validate_type(value, type); return value; }
    catch (const Error &) { return fallback; }
}
void validate_events(const Json &events) {
    validate_type(events, "Vec<CalendarEvent>");
    if (events.size() > 10000) invalid("Too many calendar events.");
    for (size_t i = 0; i < events.size(); ++i) {
        const auto event = events.at(i);
        const std::string title(event.get("title").text());
        GMemory<char> trimmed(g_strdup(title.c_str()));
        if (event.get("id").text().size() > 100 || event.get("title").text().size() > 800
            || !*g_strstrip(trimmed.get()) || event.get("date").text().size() != 10 || event.get("time").text().size() != 5)
            invalid("Invalid calendar events.");
    }
}
Json bookmark_locations(const Json &paths) {
    std::vector<Json> result;
    if (paths.is_array()) for (size_t i = 0; i < paths.size(); ++i) {
        const auto value = paths.at(i);
        if (!value.is_string()) continue;
        const std::filesystem::path path(value.text());
        if (!path.is_absolute()) continue;
        auto name = path.filename().string();
        if (name.empty()) name = "/";
        result.push_back(Json::object({{"name", Json::string(name)}, {"path", value}}));
    }
    return Json::array(result);
}
}
std::optional<Json> dispatch_stores(const Request &request, ServiceContext &context) {
    const auto &method = request.method;
    auto &store = context.config;
    if (method == "calendar.list") {
        auto value = store.load("calendar.json", Json::array());
        validate_type(value, "Vec<CalendarEvent>");
        return value;
    }
    if (method == "calendar.save") {
        validate_events(request.params);
        check_cancelled(context.cancel);
        store.save("calendar.json", request.params);
        return Json{};
    }
    if (method == "settings.search" || method == "settings.search_set") {
        const auto fallback = Json::object({{"settings", Json::boolean(true)}, {"calculator", Json::boolean(false)}});
        if (method == "settings.search") return checked_load(store, "search.json", "SearchPreferences", fallback, true);
        check_cancelled(context.cancel);
        store.save("search.json", request.params, true);
        return request.params;
    }
    if (method == "settings.idle" || method == "settings.idle_set") {
        const auto fallback = Json::object({{"lock_after", Json::unsigned_number(300)}, {"blank_after", Json::unsigned_number(330)}});
        const auto valid = [](const Json &value) {
            const auto lock = value.get("lock_after").unsigned_number();
            const auto blank = value.get("blank_after").unsigned_number();
            const std::vector<uint64_t> locks{60, 120, 300, 600, 900, 1800}, delays{30, 60, 120, 300};
            return blank > lock && std::find(locks.begin(), locks.end(), lock) != locks.end()
                && std::find(delays.begin(), delays.end(), blank - lock) != delays.end();
        };
        if (method == "settings.idle") {
            auto value = checked_load(store, "power.json", "IdleSettings", fallback);
            return valid(value) ? value : fallback;
        }
        if (!valid(request.params)) invalid("Choose a supported lock and screen-off delay.");
        check_cancelled(context.cancel);
        store.save("power.json", request.params);
        return request.params;
    }
    if (method == "files.bookmarks") return bookmark_locations(store.load("orbit-bookmarks.json", Json::array()));
    if (method == "files.bookmark_set") {
        std::lock_guard lock(store.mutex());
        const auto path = absolute_path(request.params.get("path").text());
        auto saved = checked_load(store, "orbit-bookmarks.json", "Vec<String>", Json::array());
        std::vector<Json> paths;
        bool found = false;
        for (size_t i = 0; i < saved.size(); ++i) {
            const auto item = saved.at(i);
            if (std::filesystem::path(item.text()) == path) found = true;
            else paths.push_back(item);
        }
        if (!found) {
            const auto canonical = std::filesystem::canonical(path);
            if (!std::filesystem::is_directory(canonical)) invalid("Choose a folder to bookmark.");
            const auto text = canonical.string();
            bool duplicate = false;
            for (const auto &item : paths) duplicate |= item.text() == text;
            if (!duplicate) paths.push_back(Json::string(text));
        }
        const auto result = Json::array(paths);
        check_cancelled(context.cancel);
        store.save("orbit-bookmarks.json", result);
        return bookmark_locations(result);
    }
    return std::nullopt;
}
}
