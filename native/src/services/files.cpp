#include "services/services.hpp"
#include <algorithm>
#include <array>
#include <cerrno>
#include <cstring>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

namespace meridian {
namespace {
GObjectPtr<GFile> file_for(const std::filesystem::path &path) { return GObjectPtr<GFile>(g_file_new_for_path(path.c_str())); }
std::string fold(const std::string &text) {
    GMemory<char> lowered(g_utf8_strdown(text.c_str(), -1));
    return lowered.get();
}
bool utf8(const std::string &text) { return g_utf8_validate(text.c_str(), text.size(), nullptr); }
Json listing(const std::filesystem::path &requested, bool hidden, GCancellable *cancel) {
    const auto path = std::filesystem::canonical(requested);
    std::vector<Json> entries;
    bool truncated = false;
    for (const auto &entry : std::filesystem::directory_iterator(path)) {
        check_cancelled(cancel);
        const auto name = entry.path().filename().string();
        if ((!hidden && name.starts_with('.')) || !utf8(name) || !utf8(entry.path().string())) continue;
        struct stat info{}, link{};
        if (lstat(entry.path().c_str(), &link) < 0) throw Error("failed", std::strerror(errno));
        if (stat(entry.path().c_str(), &info) < 0) info = link;
        entries.push_back(Json::object({{"name", Json::string(name)}, {"path", Json::string(entry.path().string())},
            {"directory", Json::boolean(S_ISDIR(info.st_mode))}, {"symlink", Json::boolean(S_ISLNK(link.st_mode))},
            {"size", Json::number(info.st_size)}, {"modified", Json::number(info.st_mtim.tv_sec + info.st_mtim.tv_nsec / 1e9)}}));
        if (entries.size() == 5000) { truncated = true; break; }
    }
    std::stable_sort(entries.begin(), entries.end(), [](const Json &a, const Json &b) {
        const bool ad = a.get("directory").boolean(), bd = b.get("directory").boolean();
        return ad != bd ? ad : fold(std::string(a.get("name").text())) < fold(std::string(b.get("name").text()));
    });
    return Json::object({{"path", Json::string(path.string())},
        {"parent", path == path.root_path() ? Json{} : Json::string(path.parent_path().string())},
        {"entries", Json::array(entries)}, {"truncated", Json::boolean(truncated)}});
}
Json properties(const std::filesystem::path &path, GCancellable *cancel) {
    struct stat meta{};
    if (lstat(path.c_str(), &meta) < 0) throw Error("failed", std::strerror(errno));
    auto file = file_for(path);
    GioError error;
    GObjectPtr<GFileInfo> info(g_file_query_info(file.get(), "standard::content-type,owner::user,owner::group",
        G_FILE_QUERY_INFO_NOFOLLOW_SYMLINKS, cancel, error.out()));
    std::string kind = "File";
    if (S_ISLNK(meta.st_mode)) kind = "Symbolic link";
    else if (S_ISDIR(meta.st_mode)) kind = "Folder";
    else if (info) {
        if (const auto *type = g_file_info_get_content_type(info.get())) {
            GMemory<char> description(g_content_type_get_description(type));
            if (description) kind = description.get();
        }
    }
    auto attribute = [&](const char *key, unsigned fallback) {
        const char *value = info ? g_file_info_get_attribute_string(info.get(), key) : nullptr;
        return value ? std::string(value) : std::to_string(fallback);
    };
    std::string permissions;
    const std::array<char, 9> letters{'r','w','x','r','w','x','r','w','x'};
    for (size_t i = 0; i < letters.size(); ++i) permissions += (meta.st_mode & (0400 >> i)) ? letters[i] : '-';
    char octal[16];
    std::snprintf(octal, sizeof octal, " (%03o)", unsigned(meta.st_mode & 07777));
    permissions += octal;
    std::error_code failure;
    const auto target = S_ISLNK(meta.st_mode) ? std::filesystem::read_symlink(path, failure).string() : std::string{};
    auto name = path.filename().string();
    if (name.empty()) name = "/";
    return Json::object({{"name", Json::string(name)}, {"path", Json::string(path.string())}, {"kind", Json::string(kind)},
        {"size", S_ISREG(meta.st_mode) ? Json::number(meta.st_size) : Json{}},
        {"modified", meta.st_mtim.tv_sec >= 0 ? Json::number(meta.st_mtim.tv_sec + meta.st_mtim.tv_nsec / 1e9) : Json{}},
        {"created", Json{}}, {"owner", Json::string(attribute("owner::user", meta.st_uid))},
        {"group", Json::string(attribute("owner::group", meta.st_gid))}, {"permissions", Json::string(permissions)},
        {"link_target", S_ISLNK(meta.st_mode) && !failure ? Json::string(target) : Json{}}});
}
Json preview(const std::filesystem::path &path, GCancellable *cancel) {
    const int fd = open(path.c_str(), O_RDONLY | O_CLOEXEC | O_NONBLOCK | O_NOCTTY);
    if (fd < 0) throw Error("failed", std::strerror(errno));
    struct Close { int fd; ~Close() { close(fd); } } close_file{fd};
    struct stat meta{};
    if (fstat(fd, &meta) < 0) throw Error("failed", std::strerror(errno));
    const auto unsupported = [] {
        return Json::object({{"kind", Json::string("unsupported")}, {"content", Json::string("")}, {"truncated", Json::boolean(false)}});
    };
    if (!S_ISREG(meta.st_mode)) return unsupported();
    std::string content(64 * 1024 + 1, '\0');
    size_t count = 0;
    while (count < content.size()) {
        check_cancelled(cancel);
        const auto received = read(fd, content.data() + count, content.size() - count);
        if (received < 0) { if (errno == EINTR) continue; throw Error("failed", std::strerror(errno)); }
        if (!received) break;
        count += static_cast<size_t>(received);
    }
    content.resize(count);
    const bool truncated = content.size() > 64 * 1024;
    if (truncated) content.resize(64 * 1024);
    // Permit an incomplete UTF-8 character at the bounded end, but never
    // represent arbitrary binary bytes as text.
    const char *end = nullptr;
    if (!g_utf8_validate(content.data(), content.size(), &end)) {
        const auto offset = static_cast<size_t>(end - content.data());
        if (truncated && g_utf8_get_char_validated(end, content.size() - offset) == static_cast<gunichar>(-2)) content.resize(offset);
        else return unsupported();
    }
    for (const char *p = content.data(), *end = p + content.size(); p < end; p = g_utf8_next_char(p)) {
        const auto c = g_utf8_get_char(p);
        if (g_unichar_iscntrl(c) && c != '\t' && c != '\n' && c != '\r') return unsupported();
    }
    return Json::object({{"kind", Json::string("text")}, {"content", Json::string(content)}, {"truncated", Json::boolean(truncated)}});
}
Json locations() {
    std::vector<Json> result;
    auto add = [&](const char *name, const char *path) {
        if (path && utf8(path)) result.push_back(Json::object({{"name", Json::string(name)}, {"path", Json::string(path)}}));
    };
    add("Home", g_get_home_dir());
    for (const auto &[name, directory] : std::array<std::pair<const char *, GUserDirectory>, 6>{{
        {"Desktop", G_USER_DIRECTORY_DESKTOP}, {"Documents", G_USER_DIRECTORY_DOCUMENTS}, {"Downloads", G_USER_DIRECTORY_DOWNLOAD},
        {"Pictures", G_USER_DIRECTORY_PICTURES}, {"Music", G_USER_DIRECTORY_MUSIC}, {"Videos", G_USER_DIRECTORY_VIDEOS}}}) {
        const auto *path = g_get_user_special_dir(directory);
        if (path && std::filesystem::is_directory(path)) add(name, path);
    }
    add("Computer", "/");
    return Json::array(result);
}
}
std::optional<Json> dispatch_files(const Request &request, ServiceContext &context) {
    const auto &method = request.method;
    if (method == "files.locations") return locations();
    if (method != "files.list" && method != "files.properties" && method != "files.mkdir"
        && method != "files.rename" && method != "files.trash" && method != "files.preview" && method != "files.open") return std::nullopt;
    const auto path = absolute_path(request.params.get("path").text());
    check_cancelled(context.cancel);
    if (method == "files.list") return listing(path, request.params.get("hidden").boolean(), context.cancel);
    if (method == "files.properties") return properties(path, context.cancel);
    if (method == "files.preview") return preview(path, context.cancel);
    if (method == "files.mkdir") {
        const auto name = request.params.get("name").text();
        valid_filename(name);
        if (!std::filesystem::create_directory(path / name)) throw Error("failed", "A file with that name already exists.");
        return Json{};
    }
    auto file = file_for(path);
    GioError error;
    if (method == "files.rename") {
        const auto name = request.params.get("name").text();
        valid_filename(name);
        if (path == path.root_path()) invalid("Cannot rename the filesystem root.");
        auto target = file_for(path.parent_path() / name);
        if (!g_file_move(file.get(), target.get(), G_FILE_COPY_NOFOLLOW_SYMLINKS, context.cancel, nullptr, nullptr, error.out())) error.raise();
    } else if (method == "files.trash") {
        const auto canonical = std::filesystem::is_symlink(path) ? path : std::filesystem::canonical(path);
        if (canonical == "/" || canonical == g_get_home_dir()) invalid("This location cannot be moved to Trash.");
        if (!g_file_trash(file.get(), context.cancel, error.out())) error.raise();
    } else if (method == "files.open") {
        if (!std::filesystem::exists(path)) throw Error("not_found", "The file no longer exists.");
        GMemory<char> uri(g_file_get_uri(file.get()));
        if (!g_app_info_launch_default_for_uri(uri.get(), nullptr, error.out())) error.raise();
    }
    return Json{};
}
}
