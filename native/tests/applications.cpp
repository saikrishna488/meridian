#include "services/services.hpp"
#include <filesystem>
#include <fstream>
#include <stdexcept>
#include <unistd.h>

using namespace meridian;
void require(bool ok) { if (!ok) throw std::runtime_error("applications contract failed"); }
int main() {
    char pattern[] = "/tmp/meridian-applications-XXXXXX";
    const auto *directory = mkdtemp(pattern);
    require(directory != nullptr);
    const std::filesystem::path root(directory);
    struct Cleanup { std::filesystem::path path; ~Cleanup() { std::filesystem::remove_all(path); } } cleanup{root};
    std::filesystem::create_directory(root / "applications");
    std::ofstream(root / "applications/firefox.desktop") << "[Desktop Entry]\nType=Application\nName=Firefox\nGenericName=Web Browser\nKeywords=Internet;WWW;\nExec=/usr/bin/true\nIcon=firefox\n";
    std::ofstream(root / "applications/editor.desktop") << "[Desktop Entry]\nType=Application\nName=Text Editor\nExec=/usr/bin/true\n";
    setenv("XDG_DATA_HOME", directory, 1);
    setenv("XDG_DATA_DIRS", directory, 1);
    setenv("XDG_CONFIG_HOME", directory, 1);
    ConfigStore config(root / "meridian");
    ServiceContext context{config, nullptr};
    Request query{"search.query", Json::object({{"query", Json::string("ffx")}, {"serial", Json::unsigned_number(42)}})};
    const auto result = dispatch_applications(query, context).value();
    require(result.get("serial").unsigned_number() == 42);
    const auto items = result.get("sections").at(0).get("items");
    require(items.size() == 1 && items.at(0).get("id").text() == "firefox.desktop");
    Request activate{"search.activate", Json::object({{"provider", Json::string("apps")}, {"item_id", Json::string("firefox.desktop")}})};
    require(dispatch_applications(activate, context).value().is_null());
    activate.params.set("item_id", Json::string("/usr/bin/true"));
    bool denied = false;
    try { dispatch_applications(activate, context); } catch (const Error &error) { denied = error.code == "not_found"; }
    require(denied);
    Request choice{"settings.default_app_set", Json::object({{"purpose", Json::string("terminal")}, {"app_id", Json::string("editor.desktop")}})};
    require(dispatch_applications(choice, context).value().size() == 7);
    require(config.load("default-apps.json", Json{}).get("terminal").text() == "editor.desktop");
    choice.params.set("purpose", Json::string("text"));
    dispatch_applications(choice, context);
    const auto defaults = dispatch_applications(Request{"settings.default_apps", Json{}}, context).value();
    require(defaults.at(6).get("app_id").text() == "editor.desktop");
}
