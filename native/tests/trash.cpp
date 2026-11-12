#include "services/trash.hpp"
#include <fstream>
#include <stdexcept>
#include <unistd.h>

using namespace meridian;
namespace fs = std::filesystem;
void require(bool ok) { if (!ok) throw std::runtime_error("trash contract failed"); }
template<class Action> void rejects(Action action) {
    try { action(); } catch (const std::exception &) { return; }
    throw std::runtime_error("unsafe restore accepted");
}
int main() {
    char pattern[] = "/tmp/meridian-trash-XXXXXX";
    require(mkdtemp(pattern));
    const fs::path root(pattern);
    struct Cleanup { fs::path root; ~Cleanup() { fs::remove_all(root); } } cleanup{root};
    fs::create_directory(root / "trash");
    std::ofstream(root / "trash/item") << "restore me";
    std::ofstream(root / "original") << "keep me";
    GObjectPtr<GFile> directory(g_file_new_for_path((root / "trash").c_str()));
    GObjectPtr<GFile> source(g_file_new_for_path((root / "trash/item").c_str()));
    const auto listing = trash_listing(directory.get(), nullptr);
    require(listing.get("entries").size() == 1 && !listing.get("truncated").boolean());
    rejects([&] { restore_trash_file(source.get(), (root / "original").string(), nullptr); });
    require(fs::file_size(root / "original") == 7 && fs::exists(root / "trash/item"));
    for (const char *path : {"", "relative", "/", "/tmp/../etc/file"})
        rejects([&] { restore_trash_file(source.get(), path, nullptr); });
    rejects([&] { restore_trash_file(source.get(), (root / "absent/file").string(), nullptr); });
    GObjectPtr<GCancellable> cancel(g_cancellable_new());
    g_cancellable_cancel(cancel.get());
    rejects([&] { trash_listing(directory.get(), cancel.get()); });
    rejects([&] { restore_trash_file(source.get(), (root / "restored").string(), cancel.get()); });
    restore_trash_file(source.get(), (root / "restored").string(), nullptr);
    require(fs::file_size(root / "restored") == 10 && !fs::exists(root / "trash/item"));
    fs::create_symlink("missing", root / "trash/link");
    source.reset(g_file_new_for_path((root / "trash/link").c_str()));
    restore_trash_file(source.get(), (root / "link").string(), nullptr);
    require(fs::read_symlink(root / "link") == "missing");
    directory.reset(g_file_new_for_path((root / "absent").c_str()));
    rejects([&] { trash_listing(directory.get(), nullptr); });
}
