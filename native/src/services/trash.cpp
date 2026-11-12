#include "services/trash.hpp"
#include <algorithm>

namespace meridian {
Json trash_listing(GFile *root, GCancellable *cancel) {
    GioError error;
    GObjectPtr<GFileEnumerator> enumerator(g_file_enumerate_children(root,
        "standard::name,standard::display-name,standard::type,standard::size,trash::orig-path,trash::deletion-date",
        G_FILE_QUERY_INFO_NOFOLLOW_SYMLINKS, cancel, error.out()));
    if (!enumerator) error.raise();
    std::vector<Json> entries;
    bool truncated = false;
    while (true) {
        check_cancelled(cancel);
        GObjectPtr<GFileInfo> info(g_file_enumerator_next_file(enumerator.get(), cancel, error.out()));
        if (!info) {
            if (error.failed()) error.raise();
            break;
        }
        if (entries.size() >= 5000) { truncated = true; break; }
        GObjectPtr<GFile> child(g_file_enumerator_get_child(enumerator.get(), info.get()));
        GMemory<char> uri(g_file_get_uri(child.get()));
        const auto *name = g_file_info_get_display_name(info.get());
        const auto *original = g_file_info_get_attribute_byte_string(info.get(), "trash::orig-path");
        const auto *deleted = g_file_info_get_attribute_string(info.get(), "trash::deletion-date");
        entries.push_back(Json::object({{"uri", Json::string(uri.get())}, {"name", Json::string(name ? name : "")},
            {"original", Json::string(original && g_utf8_validate(original, -1, nullptr) ? original : "")},
            {"deleted", Json::string(deleted ? deleted : "")}, {"size", Json::number(std::max<goffset>(0, g_file_info_get_size(info.get())))},
        {"directory", Json::boolean(g_file_info_get_file_type(info.get()) == G_FILE_TYPE_DIRECTORY)}}));
    }
    if (!g_file_enumerator_close(enumerator.get(), cancel, error.out())) error.raise();
    std::stable_sort(entries.begin(), entries.end(), [](const Json &a, const Json &b) {
        return a.get("deleted").text() != b.get("deleted").text() ? a.get("deleted").text() > b.get("deleted").text()
            : a.get("name").text() < b.get("name").text();
    });
    return Json::object({{"entries", Json::array(entries)}, {"truncated", Json::boolean(truncated)}});
}
void restore_trash_file(GFile *source, std::string_view original, GCancellable *cancel) {
    const auto target = absolute_path(original);
    if (target == target.root_path() || target.filename().empty()) invalid("The original location is unavailable.");
    if (!std::filesystem::is_directory(target.parent_path())) throw Error("failed", "The original folder no longer exists.");
    GObjectPtr<GFile> destination(g_file_new_for_path(target.c_str()));
    GioError error;
    if (!g_file_move(source, destination.get(), G_FILE_COPY_NOFOLLOW_SYMLINKS, cancel, nullptr, nullptr, error.out())) error.raise();
}
std::optional<Json> dispatch_trash(const Request &request, ServiceContext &context) {
    if (request.method != "files.trash_list" && request.method != "files.trash_restore") return std::nullopt;
    GObjectPtr<GFile> root(g_file_new_for_uri("trash:///"));
    const auto listing = trash_listing(root.get(), context.cancel);
    if (request.method == "files.trash_list") return listing;
    const auto uri = request.params.get("uri").text();
    const auto entries = listing.get("entries");
    for (size_t i = 0; i < entries.size(); ++i) {
        const auto entry = entries.at(i);
        if (entry.get("uri").text() != uri) continue;
        GObjectPtr<GFile> source(g_file_new_for_uri(std::string(uri).c_str()));
        restore_trash_file(source.get(), entry.get("original").text(), context.cancel);
        return entry.get("original");
    }
    throw Error("not_found", "This item is no longer in Trash. Refresh and try again.");
}
}
