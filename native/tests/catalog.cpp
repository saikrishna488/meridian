#include "meridian.h"
#include <filesystem>
#include <fstream>
#include <cstdlib>
#include <cstring>
#include <stdexcept>
#include <unistd.h>

void require(bool ok) { if (!ok) throw std::runtime_error("catalog contract failed"); }
int main() {
    char pattern[] = "/tmp/meridian-catalog-XXXXXX";
    const char *root = mkdtemp(pattern);
    require(root != nullptr);
    struct Cleanup { std::filesystem::path path; ~Cleanup() { std::filesystem::remove_all(path); } } cleanup{root};
    const auto apps = std::filesystem::path(root) / "applications";
    std::filesystem::create_directories(apps);
    std::ofstream(apps / "visible.desktop") << "[Desktop Entry]\nType=Application\nName=Visible App\nExec=/usr/bin/true\nIcon=utilities-terminal\nGenericName=Console\nKeywords=Straße;command;\nComment=Run commands\n";
    std::ofstream(apps / "hidden.desktop") << "[Desktop Entry]\nType=Application\nName=Hidden App\nExec=/usr/bin/true\nNoDisplay=true\n";
    setenv("XDG_DATA_HOME", root, 1);
    setenv("XDG_DATA_DIRS", root, 1);
    auto *catalog = meridian_catalog_create();
    require(catalog != nullptr);
    require(meridian_catalog_count(catalog) == 1);
    require(std::strcmp(meridian_catalog_id(catalog, 0), "visible.desktop") == 0);
    require(std::strcmp(meridian_catalog_name(catalog, 0), "Visible App") == 0);
    require(meridian_catalog_name(catalog, 1) == nullptr);
    require(std::strcmp(meridian_catalog_icon(catalog, 0), "utilities-terminal") == 0);
    require(!meridian_catalog_icon(catalog, 1));
    size_t indices[2] = {99, 77};
    for (const char *query : {"VISIBLE", "Console", "STRASSE", "commands", "visible.desktop", ""}) {
        require(meridian_catalog_search(catalog, query, indices, 1) == 1);
        require(indices[0] == 0 && indices[1] == 77);
    }
    require(meridian_catalog_search(catalog, "", nullptr, 0) == 1);
    require(!meridian_catalog_search(catalog, "absent", indices, 1));
    require(!meridian_catalog_search(catalog, "\xff", indices, 1));
    require(!meridian_catalog_search(catalog, std::string(4097, 'a').c_str(), indices, 1));
    require(!meridian_catalog_search(catalog, "", nullptr, 1));
    char error[8];
    require(!meridian_catalog_launch(catalog, "hidden.desktop", error, sizeof error));
    require(error[7] == '\0');
    require(!meridian_catalog_launch(catalog, "/usr/bin/true", nullptr, 0));
    require(!meridian_catalog_launch(nullptr, nullptr, error, 1));
    require(error[0] == '\0');
    require(meridian_catalog_launch(catalog, "visible.desktop", error, sizeof error));
    require(error[0] == '\0');
    meridian_catalog_destroy(catalog);
}
