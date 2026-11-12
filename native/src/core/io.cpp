#include "core/io.hpp"
#include "core/error.hpp"
#include <array>
#include <cerrno>
#include <cstring>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

namespace meridian {
namespace {
class Fd {
    int fd_;
public:
    explicit Fd(int fd) : fd_(fd) {}
    ~Fd() { if (fd_ >= 0) close(fd_); }
    Fd(const Fd &) = delete;
    int get() const { return fd_; }
};
[[noreturn]] void io_error() { throw Error("failed", std::strerror(errno)); }
void write_all(int fd, std::string_view data) {
    while (!data.empty()) {
        const auto written = write(fd, data.data(), data.size());
        if (written < 0) { if (errno == EINTR) continue; io_error(); }
        if (!written) throw Error("failed", "Short configuration write.");
        data.remove_prefix(static_cast<size_t>(written));
    }
}
}
std::filesystem::path absolute_path(std::string_view input) {
    std::filesystem::path path(input);
    if (!path.is_absolute() || input.find('\0') != std::string_view::npos) invalid("Choose an absolute file path.");
    for (const auto &part : path) if (part == "..") invalid("Parent path components are not allowed.");
    return path.lexically_normal();
}
void valid_filename(std::string_view name) {
    if (name.empty() || name == "." || name == ".." || name.find_first_of("/\0", 0, 2) != std::string_view::npos)
        invalid("Enter a name without slashes or NUL bytes.");
}
std::optional<std::string> read_bounded(const std::filesystem::path &path, size_t limit) {
    Fd file(open(path.c_str(), O_RDONLY | O_CLOEXEC | O_NONBLOCK));
    if (file.get() < 0) { if (errno == ENOENT) return std::nullopt; io_error(); }
    struct stat metadata{};
    if (fstat(file.get(), &metadata) < 0) io_error();
    if (!S_ISREG(metadata.st_mode)) throw Error("failed", "Expected a regular file.");
    if (metadata.st_size < 0 || static_cast<uint64_t>(metadata.st_size) > limit)
        throw Error("failed", "File exceeds the read limit.");
    std::string result;
    std::array<char, 8192> buffer;
    while (true) {
        const auto count = read(file.get(), buffer.data(), std::min(buffer.size(), limit + 1 - result.size()));
        if (count < 0) { if (errno == EINTR) continue; io_error(); }
        if (!count) return result;
        result.append(buffer.data(), static_cast<size_t>(count));
        if (result.size() > limit) throw Error("failed", "File exceeds the read limit.");
    }
}
void write_atomic(const std::filesystem::path &path, std::string_view data, bool exclusive) {
    std::filesystem::create_directories(path.parent_path());
    std::string temporary = (path.parent_path() / ("." + path.filename().string() + ".XXXXXX")).string();
    Fd file(mkostemp(temporary.data(), O_CLOEXEC));
    if (file.get() < 0) io_error();
    struct Remove { std::string &path; ~Remove() { unlink(path.c_str()); } } remove{temporary};
    if (fchmod(file.get(), 0600) < 0) io_error();
    write_all(file.get(), data);
    if (fsync(file.get()) < 0) io_error();
    if (exclusive) {
        if (link(temporary.c_str(), path.c_str()) < 0) io_error();
        if (unlink(temporary.c_str()) < 0) io_error();
    } else if (rename(temporary.c_str(), path.c_str()) < 0) io_error();
    Fd directory(open(path.parent_path().c_str(), O_RDONLY | O_CLOEXEC | O_DIRECTORY));
    if (directory.get() < 0 || fsync(directory.get()) < 0) io_error();
}
ConfigStore::ConfigStore(std::filesystem::path root) : root_(absolute_path(root.string())) {}
std::filesystem::path ConfigStore::path(const char *name) const {
    valid_filename(name);
    return root_ / name;
}
Json ConfigStore::load(const char *name, const Json &fallback, bool versioned) {
    std::lock_guard lock(mutex_);
    const auto data = read_bounded(path(name), config_limit);
    if (!data) return fallback;
    Json value;
    try { value = Json::parse(*data, config_limit); }
    catch (const Error &) { return fallback; }
    if (versioned && value.is_object() && value.get("schema_version").exists()) {
        const auto version = value.get("schema_version");
        if (!version.is_unsigned() || version.unsigned_number() != 1 || !value.get("data").exists()) return fallback;
        return value.get("data");
    }
    return value;
}
void ConfigStore::save(const char *name, const Json &value, bool versioned) {
    std::lock_guard lock(mutex_);
    const auto file = path(name);
    const auto data = read_bounded(file, config_limit);
    if (data) {
        bool corrupt = false;
        Json previous;
        try { previous = Json::parse(*data, config_limit); }
        catch (const Error &) { corrupt = true; }
        if (!corrupt && versioned && previous.is_object() && previous.get("schema_version").exists()) {
            const auto version = previous.get("schema_version");
            if (!version.is_unsigned() || version.unsigned_number() != 1)
                throw Error("failed", "Unsupported configuration version; upgrade before saving.");
            corrupt = !previous.get("data").exists();
        }
        if (corrupt) {
            std::string backup = file.string() + ".corrupt-XXXXXX";
            Fd preserved(mkostemp(backup.data(), O_CLOEXEC));
            if (preserved.get() < 0) io_error();
            if (fchmod(preserved.get(), 0600) < 0) io_error();
            write_all(preserved.get(), *data);
            if (fsync(preserved.get()) < 0) io_error();
            Fd directory(open(file.parent_path().c_str(), O_RDONLY | O_CLOEXEC | O_DIRECTORY));
            if (directory.get() < 0 || fsync(directory.get()) < 0) io_error();
        }
    }
    const auto stored = versioned ? Json::object({{"schema_version", Json::unsigned_number(1)}, {"data", value}}) : value;
    const auto encoded = stored.dump();
    if (encoded.size() > config_limit) throw Error("failed", "Configuration exceeds 4 MiB.");
    write_atomic(file, encoded);
}
}
