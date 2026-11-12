#include "services/services.hpp"
#include "meridian.h"
#include <gio/gdesktopappinfo.h>
#include <array>
#include <cstring>

namespace meridian {
namespace {
using Catalog = std::unique_ptr<meridian_catalog, decltype(&meridian_catalog_destroy)>;
Catalog catalog() {
    Catalog result(meridian_catalog_create(), meridian_catalog_destroy);
    if (!result) throw Error("failed", "Application discovery failed.");
    return result;
}
Json maybe_string(const char *value) { return value ? Json::string(value) : Json{}; }
bool contains(const meridian_catalog *catalog, std::string_view id) {
    for (size_t i = 0; i < meridian_catalog_count(catalog); ++i)
        if (id == meridian_catalog_id(catalog, i)) return true;
    return false;
}
constexpr std::array<std::pair<const char *, const char *>, 5> purposes{{
    {"browser", "x-scheme-handler/https"}, {"mail", "x-scheme-handler/mailto"},
    {"pdf", "application/pdf"}, {"images", "image/png"}, {"text", "text/plain"}}};
Json shell_defaults(ConfigStore &config) {
    auto value = config.load("default-apps.json", Json::object());
    if (!value.is_object()) return Json::object();
    for (const char *key : {"files", "terminal"})
        if (value.get(key).exists() && !value.get(key).is_null() && !value.get(key).is_string()) return Json::object();
    return value;
}
Json defaults(ConfigStore &config) {
    const auto local = shell_defaults(config);
    std::vector<Json> result;
    for (const auto &[purpose, fallback] : std::array<std::pair<const char *, const char *>, 2>{{
        {"files", "meridian-finder.desktop"}, {"terminal", "meridian-terminal.desktop"}}}) {
        const auto value = local.get(purpose);
        result.push_back(Json::object({{"purpose", Json::string(purpose)}, {"app_id", value.is_string() ? value : Json::string(fallback)}}));
    }
    for (const auto &[purpose, mime] : purposes) {
        GObjectPtr<GAppInfo> app(g_app_info_get_default_for_type(mime, false));
        result.push_back(Json::object({{"purpose", Json::string(purpose)}, {"app_id", maybe_string(app ? g_app_info_get_id(app.get()) : nullptr)}}));
    }
    return Json::array(result);
}
}
std::optional<Json> dispatch_applications(const Request &request, ServiceContext &context) {
    const auto &method = request.method;
    if (method == "settings.default_apps") return defaults(context.config);
    if (method != "search.query" && method != "search.activate" && method != "settings.default_app_set"
        && method != "settings.startup_apps") return std::nullopt;
    auto apps = catalog();
    check_cancelled(context.cancel);
    if (method == "search.activate") {
        if (request.params.get("provider").text() != "apps")
            throw Error("unavailable", "This search provider requires host integration.");
        const std::string id(request.params.get("item_id").text());
        if (!contains(apps.get(), id)) throw Error("not_found", "No such application.");
        char error[1024];
        if (!meridian_catalog_launch(apps.get(), id.c_str(), error, sizeof error)) throw Error("failed", error);
        return Json{};
    }
    if (method == "settings.default_app_set") {
        std::lock_guard lock(context.config.mutex());
        const auto purpose = request.params.get("purpose").text();
        const std::string id(request.params.get("app_id").text());
        if (!contains(apps.get(), id) && id != "meridian-finder.desktop" && id != "meridian-terminal.desktop")
            throw Error("not_found", "Select an installed application.");
        if (purpose == "files" || purpose == "terminal") {
            if ((purpose == "files" && id == "meridian-terminal.desktop") || (purpose == "terminal" && id == "meridian-finder.desktop"))
                invalid("This application cannot handle that purpose.");
            auto value = shell_defaults(context.config);
            value.set(std::string(purpose).c_str(), Json::string(id));
            check_cancelled(context.cancel);
            context.config.save("default-apps.json", value);
        } else {
            const char *mime = nullptr;
            for (const auto &entry : purposes) if (purpose == entry.first) mime = entry.second;
            if (!mime) invalid("Unknown default app purpose.");
            GObjectPtr<GDesktopAppInfo> app(g_desktop_app_info_new(id.c_str()));
            if (!app) throw Error("not_found", "Application desktop entry unavailable.");
            GioError error;
            if (!g_app_info_set_as_default_for_type(G_APP_INFO(app.get()), mime, error.out())) error.raise();
        }
        return defaults(context.config);
    }
    std::vector<size_t> indices(meridian_catalog_count(apps.get()));
    const std::string query = method == "search.query" ? std::string(request.params.get("query").text()) : "";
    const auto count = meridian_catalog_query(apps.get(), query.c_str(), indices.data(), indices.size());
    std::vector<Json> items;
    for (size_t i = 0; i < count; ++i) {
        check_cancelled(context.cancel);
        const auto index = indices[i];
        if (method == "settings.startup_apps" && !meridian_catalog_source(apps.get(), index)) continue;
        const auto *subtitle = method == "settings.startup_apps" ? meridian_catalog_description(apps.get(), index)
                                                                : meridian_catalog_generic_name(apps.get(), index);
        if (!subtitle) subtitle = meridian_catalog_description(apps.get(), index);
        items.push_back(Json::object({{"id", Json::string(meridian_catalog_id(apps.get(), index))},
            {"title", Json::string(meridian_catalog_name(apps.get(), index))}, {"subtitle", maybe_string(subtitle)},
            {"icon", maybe_string(meridian_catalog_icon(apps.get(), index))}}));
    }
    if (method == "settings.startup_apps") return Json::array(items);
    std::vector<Json> sections;
    if (!items.empty()) sections.push_back(Json::object({{"provider", Json::string("apps")},
        {"title", Json::string("Applications")}, {"items", Json::array(items)}}));
    return Json::object({{"serial", request.params.get("serial")}, {"sections", Json::array(sections)}});
}
}
