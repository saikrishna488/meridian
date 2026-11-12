#include "services/startup.hpp"
#include "meridian.h"
#include <algorithm>
#include <map>
#include <set>

namespace meridian {
namespace {
constexpr const char *group = "Desktop Entry";
struct KeyUnref { void operator()(GKeyFile *value) const { g_key_file_unref(value); } };
using KeyFile = std::unique_ptr<GKeyFile, KeyUnref>;
KeyFile load(const std::filesystem::path &path) {
    const auto data = read_bounded(path, config_limit);
    if (!data) throw Error("not_found", "Startup entry no longer exists.");
    KeyFile file(g_key_file_new());
    GioError error;
    if (!g_key_file_load_from_data(file.get(), data->data(), data->size(),
        GKeyFileFlags(G_KEY_FILE_KEEP_COMMENTS | G_KEY_FILE_KEEP_TRANSLATIONS), error.out())) error.raise();
    return file;
}
std::string text(GKeyFile *file, const char *key, bool localized = false) {
    GMemory<char> value(localized ? g_key_file_get_locale_string(file, group, key, nullptr, nullptr)
                                 : g_key_file_get_string(file, group, key, nullptr));
    return value ? value.get() : "";
}
bool boolean(GKeyFile *file, const char *key, bool fallback = false) {
    if (!g_key_file_has_key(file, group, key, nullptr)) return fallback;
    return g_key_file_get_boolean(file, group, key, nullptr);
}
bool valid_id(std::string_view id) {
    return !id.empty() && id.ends_with(".desktop") && id.size() <= 255 && !id.starts_with('.')
        && id.find_first_of("/\\") == std::string_view::npos && id.find('\0') == std::string_view::npos;
}
struct Entry { Json value; std::filesystem::path path; };
std::vector<Entry> scan(const StartupRoots &roots, std::string_view desktops, GCancellable *cancel) {
    std::set<std::string> seen;
    std::map<std::string, Entry> entries;
    for (size_t index = 0; index < roots.size(); ++index) {
        std::error_code error;
        std::filesystem::directory_iterator files(roots[index], error);
        if (error) continue;
        for (const auto &entry : files) {
            check_cancelled(cancel);
            const auto id = entry.path().filename().string();
            if (!valid_id(id) || !seen.insert(id).second) continue;
            KeyFile file;
            try { file = load(entry.path()); } catch (const Error &) { continue; }
            KeyFile fallback;
            if (text(file.get(), "Name").empty()) for (size_t i = index + 1; i < roots.size(); ++i) {
                try { fallback = load(roots[i] / id); break; } catch (const Error &) {}
            }
            auto *metadata = fallback ? fallback.get() : file.get();
            if (text(metadata, "Type") != "Application") continue;
            const auto matches = [&](const std::string &list) {
                GStrv items(g_strsplit(list.c_str(), ";", -1));
                bool match = false;
                GStrv environments(g_strsplit(std::string(desktops).c_str(), ":", -1));
                for (size_t i = 0; items.get()[i]; ++i) for (size_t j = 0; environments.get()[j]; ++j)
                    if (*items.get()[i] && std::string_view(items.get()[i]) == environments.get()[j]) match = true;
                return match;
            };
            const auto only = text(metadata, "OnlyShowIn"), not_in = text(metadata, "NotShowIn");
            const auto try_exec = text(metadata, "TryExec"), command = text(metadata, "Exec");
            std::string reason;
            if (!only.empty() && !matches(only)) reason = "For another desktop environment";
            else if (matches(not_in)) reason = "Excluded from this desktop environment";
            else if (!try_exec.empty()) {
                GMemory<char> executable(g_find_program_in_path(try_exec.c_str()));
                if (!executable) reason = "Required program is not installed";
            }
            if (reason.empty() && command.empty() && !boolean(metadata, "DBusActivatable")) reason = "No startup command";
            auto name = text(metadata, "Name", true);
            if (name.empty()) name = id;
            entries.emplace(id, Entry{Json::object({{"id", Json::string(id)}, {"name", Json::string(name)},
                {"description", Json::string(text(metadata, "Comment", true))}, {"command", Json::string(command)},
                {"enabled", Json::boolean(!boolean(file.get(), "Hidden") && boolean(file.get(), "X-GNOME-Autostart-enabled", true))},
                {"available", Json::boolean(reason.empty())}, {"reason", Json::string(reason)}, {"user", Json::boolean(index == 0)}}), entry.path()});
        }
    }
    std::vector<Entry> result;
    for (auto &[id, entry] : entries) result.push_back(std::move(entry));
    std::stable_sort(result.begin(), result.end(), [](const auto &a, const auto &b) {
        GMemory<char> left(g_utf8_strdown(std::string(a.value.get("name").text()).c_str(), -1));
        GMemory<char> right(g_utf8_strdown(std::string(b.value.get("name").text()).c_str(), -1));
        return std::string_view(left.get()) < right.get();
    });
    return result;
}
void save(GKeyFile *file, const std::filesystem::path &path, bool exclusive) {
    gsize length = 0;
    GioError error;
    GMemory<char> data(g_key_file_to_data(file, &length, error.out()));
    if (!data) error.raise();
    if (length > config_limit) throw Error("failed", "Startup entry exceeds size limit.");
    write_atomic(path, std::string_view(data.get(), length), exclusive);
}
}
Json startup_list(const StartupRoots &roots, std::string_view desktops, GCancellable *cancel) {
    std::vector<Json> result;
    for (auto &entry : scan(roots, desktops, cancel)) result.push_back(std::move(entry.value));
    return Json::array(result);
}
Json startup_set(const StartupRoots &roots, std::string_view desktops, std::string_view id, bool enabled, GCancellable *cancel) {
    if (roots.empty() || !valid_id(id)) invalid("Invalid startup application.");
    for (const auto &entry : scan(roots, desktops, cancel)) {
        if (entry.value.get("id").text() != id) continue;
        if (enabled && !entry.value.get("available").boolean()) invalid("This application is unavailable for this desktop.");
        auto file = load(entry.path);
        if (text(file.get(), "Name").empty()) {
            bool found = false;
            for (size_t i = 1; i < roots.size(); ++i) {
                try { file = load(roots[i] / id); found = true; break; } catch (const Error &) {}
            }
            if (!found) throw Error("not_found", "Original startup application is missing.");
        }
        g_key_file_set_boolean(file.get(), group, "Hidden", !enabled);
        g_key_file_set_boolean(file.get(), group, "X-GNOME-Autostart-enabled", enabled);
        check_cancelled(cancel);
        save(file.get(), roots.front() / id, false);
        return startup_list(roots, desktops, cancel);
    }
    throw Error("not_found", "Startup application no longer exists.");
}
Json startup_add(const StartupRoots &roots, std::string_view desktops, std::string_view id,
                 const std::filesystem::path &source, GCancellable *cancel) {
    if (roots.empty() || !valid_id(id)) invalid("Invalid application id.");
    auto file = load(source);
    if (text(file.get(), "Type") != "Application") invalid("Choose an installed application.");
    g_key_file_set_boolean(file.get(), group, "Hidden", false);
    g_key_file_set_boolean(file.get(), group, "X-GNOME-Autostart-enabled", true);
    check_cancelled(cancel);
    save(file.get(), roots.front() / id, true);
    return startup_list(roots, desktops, cancel);
}
std::optional<Json> dispatch_startup(const Request &request, ServiceContext &context) {
    if (request.method != "settings.startup" && request.method != "settings.startup_set" && request.method != "settings.startup_add") return std::nullopt;
    std::lock_guard lock(context.config.mutex());
    StartupRoots roots{context.config.root().parent_path() / "autostart"};
    const auto *system = g_get_system_config_dirs();
    for (size_t i = 0; system[i]; ++i) roots.emplace_back(std::filesystem::path(system[i]) / "autostart");
    const auto *environment = g_getenv("XDG_CURRENT_DESKTOP");
    const std::string desktops = environment ? environment : "MarsOS:wlroots";
    if (request.method == "settings.startup") return startup_list(roots, desktops, context.cancel);
    const auto id = request.params.get("id").text();
    if (request.method == "settings.startup_set") return startup_set(roots, desktops, id, request.params.get("enabled").boolean(), context.cancel);
    std::unique_ptr<meridian_catalog, decltype(&meridian_catalog_destroy)> apps(meridian_catalog_create(), meridian_catalog_destroy);
    if (!apps) throw Error("failed", "Application discovery failed.");
    for (size_t i = 0; i < meridian_catalog_count(apps.get()); ++i) {
        if (id != meridian_catalog_id(apps.get(), i)) continue;
        if (const auto *source = meridian_catalog_source(apps.get(), i)) return startup_add(roots, desktops, id, source, context.cancel);
    }
    invalid("Choose an installed application.");
}
}
