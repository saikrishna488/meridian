#include "services/startup.hpp"
#include <fstream>
#include <stdexcept>
#include <unistd.h>

using namespace meridian;
void require(bool ok) { if (!ok) throw std::runtime_error("startup contract failed"); }
template<class Action> void rejects(Action action) {
    try { action(); } catch (const Error &) { return; }
    throw std::runtime_error("invalid startup operation accepted");
}
Json item(const Json &entries, const char *id) {
    for (size_t i = 0; i < entries.size(); ++i) if (entries.at(i).get("id").text() == id) return entries.at(i);
    throw std::runtime_error("startup entry missing");
}
int main() {
    char pattern[] = "/tmp/meridian-startup-XXXXXX";
    const auto *directory = mkdtemp(pattern);
    require(directory != nullptr);
    const std::filesystem::path root(directory);
    struct Cleanup { std::filesystem::path path; ~Cleanup() { std::filesystem::remove_all(path); } } cleanup{root};
    const StartupRoots roots{root / "user", root / "system"};
    for (const auto &path : roots) std::filesystem::create_directory(path);
    const std::string source = "[Desktop Entry]\nType=Application\nName=Example\nExec=/bin/true\nComment=Start me\n";
    std::ofstream(roots[1] / "example.desktop") << source;
    std::ofstream(roots[1] / "other.desktop") << source << "OnlyShowIn=GNOME;\n";
    std::ofstream(roots[1] / "missing.desktop") << source << "TryExec=meridian-program-that-does-not-exist-123\n";
    require(startup_list(roots, "MarsOS").size() == 3);
    rejects([&] { startup_set(roots, "MarsOS", "other.desktop", true); });
    rejects([&] { startup_set(roots, "MarsOS", "missing.desktop", true); });
    require(!item(startup_set(roots, "MarsOS", "example.desktop", false), "example.desktop").get("enabled").boolean());
    require(read_bounded(roots[1] / "example.desktop", config_limit).value() == source);
    require(item(startup_set(roots, "MarsOS", "example.desktop", true), "example.desktop").get("enabled").boolean());
    std::ofstream(roots[0] / "example.desktop") << "[Desktop Entry]\nHidden=true\n";
    require(item(startup_set(roots, "MarsOS", "example.desktop", true), "example.desktop").get("command").text() == "/bin/true");
    for (const char *id : {"../outside.desktop", "/tmp/app.desktop", "bad\\name.desktop", ".desktop", "file.txt"})
        rejects([&] { startup_set(roots, "MarsOS", id, true); });
    std::ofstream(roots[0] / "example.desktop") << "not a desktop file";
    require(startup_list(roots, "MarsOS").size() == 2);
    require(item(startup_add(roots, "MarsOS", "added.desktop", roots[1] / "example.desktop"), "added.desktop").get("enabled").boolean());
    rejects([&] { startup_add(roots, "MarsOS", "added.desktop", roots[1] / "example.desktop"); });
}
