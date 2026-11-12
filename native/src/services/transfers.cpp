#include "services/transfers.hpp"
#include <array>
#include <atomic>
#include <cerrno>
#include <cstring>
#include <fcntl.h>
#include <limits>
#include <map>
#include <sys/stat.h>
#include <sys/statvfs.h>
#include <thread>
#include <unistd.h>

namespace meridian {
namespace {
namespace fs = std::filesystem;
[[noreturn]] void io_error() { throw Error("failed", std::strerror(errno)); }
struct Fd {
    int value;
    explicit Fd(int value) : value(value) { if (value < 0) io_error(); }
    ~Fd() { close(value); }
    Fd(const Fd &) = delete;
};
void sync_directory(const fs::path &path) {
    Fd fd(open(path.c_str(), O_RDONLY | O_DIRECTORY | O_CLOEXEC));
    if (fsync(fd.value) < 0) io_error();
}
void rename_exclusive(const fs::path &from, const fs::path &to) {
    if (renameat2(AT_FDCWD, from.c_str(), AT_FDCWD, to.c_str(), RENAME_NOREPLACE) < 0) io_error();
}
struct Control {
    GObjectPtr<GCancellable> cancel{g_cancellable_new()};
    std::atomic<uint64_t> bytes{0}, total{0}, entries{0};
    void check() const { check_cancelled(cancel.get()); }
};
struct Stage {
    fs::path directory;
    bool preserve = false;
    Stage(const fs::path &parent, const fs::path &original, const char *operation) {
        auto pattern = (parent / ".meridian-transfer-XXXXXX").string();
        if (!mkdtemp(pattern.data())) io_error();
        directory = pattern;
        try {
            write_atomic(directory / "recovery.json", Json::object({{"version", Json::integer(1)},
                {"operation", Json::string(operation)}, {"original", Json::string(original.string())},
                {"data", Json::string("data")},
                {"recovery", Json::string("Restore or verify held move data before deleting this directory.")}}).dump(), true);
            sync_directory(directory);
        } catch (...) { std::error_code ignored; fs::remove_all(directory, ignored); throw; }
    }
    ~Stage() {
        std::error_code ignored;
        if (preserve && fs::symlink_status(data(), ignored).type() != fs::file_type::not_found) return;
        fs::remove_all(directory, ignored);
    }
    fs::path data() const { return directory / "data"; }
};
struct stat metadata(const fs::path &path) {
    struct stat result{};
    if (lstat(path.c_str(), &result) < 0) io_error();
    return result;
}
bool same(const struct stat &a, const struct stat &b) {
    return a.st_dev == b.st_dev && a.st_ino == b.st_ino && a.st_size == b.st_size
        && a.st_mtim.tv_sec == b.st_mtim.tv_sec && a.st_mtim.tv_nsec == b.st_mtim.tv_nsec
        && a.st_ctim.tv_sec == b.st_ctim.tv_sec && a.st_ctim.tv_nsec == b.st_ctim.tv_nsec;
}
void bound(Control &control, unsigned depth, size_t &entries) {
    control.check();
    if (depth > 128 || ++entries > 200000) throw Error("failed", "Transfer exceeds the entry or folder depth limit.");
}
uint64_t measure(const fs::path &path, Control &control, unsigned depth, size_t &entries) {
    bound(control, depth, entries);
    const auto info = metadata(path);
    if (S_ISLNK(info.st_mode)) return 0;
    if (S_ISREG(info.st_mode)) return info.st_size;
    if (!S_ISDIR(info.st_mode)) throw Error("failed", "Special files cannot be copied.");
    uint64_t total = 0;
    for (const auto &entry : fs::directory_iterator(path)) {
        const auto bytes = measure(entry.path(), control, depth + 1, entries);
        if (bytes > std::numeric_limits<uint64_t>::max() - total) throw Error("failed", "Transfer is too large.");
        total += bytes;
    }
    return total;
}
void copy_tree(const fs::path &from, const fs::path &to, Control &control, unsigned depth, size_t &entries) {
    bound(control, depth, entries);
    const auto before = metadata(from);
    if (S_ISLNK(before.st_mode)) fs::create_symlink(fs::read_symlink(from), to);
    else if (S_ISDIR(before.st_mode)) {
        if (mkdir(to.c_str(), 0700) < 0) io_error();
        for (const auto &entry : fs::directory_iterator(from)) copy_tree(entry.path(), to / entry.path().filename(), control, depth + 1, entries);
        if (!same(before, metadata(from))) throw Error("failed", "Source folder changed during transfer.");
        if (chmod(to.c_str(), before.st_mode & 0777) < 0) io_error();
        sync_directory(to);
    } else if (S_ISREG(before.st_mode)) {
        Fd input(open(from.c_str(), O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC));
        struct stat opened{};
        if (fstat(input.value, &opened) < 0) io_error();
        if (!same(before, opened)) throw Error("failed", "Source changed during transfer.");
        Fd output(open(to.c_str(), O_WRONLY | O_CREAT | O_EXCL | O_CLOEXEC, 0600));
        std::array<char, 65536> buffer;
        uint64_t copied = 0;
        while (true) {
            control.check();
            const auto count = read(input.value, buffer.data(), buffer.size());
            if (count < 0) { if (errno == EINTR) continue; io_error(); }
            if (!count) break;
            copied += count;
            if (copied > static_cast<uint64_t>(before.st_size)) throw Error("failed", "Source grew during transfer.");
            for (ssize_t offset = 0; offset < count;) {
                control.check();
                const auto written = write(output.value, buffer.data() + offset, count - offset);
                if (written < 0) { if (errno == EINTR) continue; io_error(); }
                if (!written) throw Error("failed", "Transfer write made no progress.");
                offset += written;
                control.bytes += written;
            }
        }
        if (fstat(input.value, &opened) < 0) io_error();
        if (!same(before, opened) || !same(before, metadata(from))) throw Error("failed", "Source changed during transfer.");
        if (fchmod(output.value, before.st_mode & 0777) < 0 || fsync(output.value) < 0) io_error();
    } else throw Error("failed", "Special files cannot be copied.");
    ++control.entries;
}
void run(const fs::path &source, const fs::path &destination, bool moving, Control &control) {
    control.check();
    const auto info = metadata(source);
    const auto canonical = S_ISLNK(info.st_mode) ? source : fs::canonical(source);
    if (canonical == "/" || (moving && canonical == g_get_home_dir())) invalid("This location cannot be transferred.");
    const auto folder = fs::canonical(destination);
    if (!fs::is_directory(folder)) invalid("Destination must be a directory.");
    if (S_ISDIR(info.st_mode)) {
        const auto relative = folder.lexically_relative(canonical);
        if (relative.empty() || *relative.begin() != "..") invalid("Choose a destination outside the source folder.");
    }
    auto name = source.filename();
    if (name.empty()) name = source.parent_path().filename();
    valid_filename(name.string());
    const auto target = folder / name;
    if (moving) {
        if (renameat2(AT_FDCWD, source.c_str(), AT_FDCWD, target.c_str(), RENAME_NOREPLACE) == 0) {
            try { sync_directory(folder); sync_directory(source.parent_path()); }
            catch (...) { throw Error("failed", "Move completed, but durability could not be confirmed. Verify the destination before retrying."); }
            return;
        }
        if (errno != EXDEV) io_error();
    }
    std::error_code status_error;
    if (fs::symlink_status(target, status_error).type() != fs::file_type::not_found)
        throw Error("failed", "An item already exists at the destination.");
    size_t entries = 0;
    const auto total = measure(source, control, 0, entries);
    control.total = total;
    struct statvfs space{};
    if (statvfs(folder.c_str(), &space) < 0) io_error();
    if (space.f_frsize && total / space.f_frsize + (total % space.f_frsize != 0) > space.f_bavail)
        throw Error("failed", "Not enough storage space at the destination.");
    Stage stage(folder, source, "copy");
    std::unique_ptr<Stage> held;
    if (moving) {
        held = std::make_unique<Stage>(source.parent_path(), source, "held-move");
        held->preserve = true;
        rename_exclusive(source, held->data());
    }
    try {
        if (held) { sync_directory(held->directory); sync_directory(source.parent_path()); }
        entries = 0;
        copy_tree(held ? held->data() : source, stage.data(), control, 0, entries);
        control.check();
        rename_exclusive(stage.data(), target);
        sync_directory(folder);
        if (held) held->preserve = false;
    } catch (...) {
        if (held) {
            try { rename_exclusive(held->data(), source); sync_directory(source.parent_path()); }
            catch (...) { throw Error("failed", "Move failed; original data is preserved at " + held->directory.string() + ". Inspect recovery.json before retrying."); }
        }
        throw;
    }
}
struct Choice {
    fs::path source, destination;
    bool moving;
    explicit Choice(const Request &request) : source(absolute_path(request.params.get("path").text())),
        destination(absolute_path(request.params.get("destination").text())), moving(request.params.get("move_file").boolean()) {}
};
}
struct Transfers::State {
    struct Job {
        Control control;
        std::mutex mutex;
        bool done = false;
        std::string error;
        std::thread worker;
        ~Job() { g_cancellable_cancel(control.cancel.get()); if (worker.joinable()) worker.join(); }
        Json status(uint32_t id) {
            std::lock_guard lock(mutex);
            return Json::object({{"id", Json::unsigned_number(id)}, {"bytes", Json::number(control.bytes.load())},
                {"total", Json::number(control.total.load())}, {"entries", Json::number(control.entries.load())},
                {"done", Json::boolean(done)}, {"error", error.empty() ? Json{} : Json::string(error)}});
        }
    };
    std::mutex mutex;
    std::map<uint32_t, std::unique_ptr<Job>> jobs;
    uint32_t next = 0;
    ~State() {
        for (auto &[id, job] : jobs) { (void)id; g_cancellable_cancel(job->control.cancel.get()); }
    }
};
Transfers::Transfers() : state_(std::make_unique<State>()) {}
Transfers::~Transfers() = default;
std::optional<Json> Transfers::dispatch(const Request &request, GCancellable *cancel) {
    const auto &method = request.method;
    if (method == "files.transfer") {
        Choice choice(request);
        Control control;
        const auto forward = +[](GCancellable *, gpointer data) { g_cancellable_cancel(static_cast<GCancellable *>(data)); };
        const auto connection = cancel ? g_cancellable_connect(cancel, G_CALLBACK(forward), control.cancel.get(), nullptr) : 0;
        struct Disconnect { GCancellable *cancel; gulong id; ~Disconnect() { if (id) g_cancellable_disconnect(cancel, id); } } disconnect{cancel, connection};
        run(choice.source, choice.destination, choice.moving, control);
        return Json{};
    }
    if (method != "files.transfer_start" && method != "files.transfer_status" && method != "files.transfer_cancel" && method != "files.transfers") return std::nullopt;
    std::lock_guard lock(state_->mutex);
    if (method == "files.transfer_start") {
        Choice choice(request);
        size_t active = 0;
        for (auto it = state_->jobs.begin(); it != state_->jobs.end();) {
            bool done;
            { std::lock_guard job_lock(it->second->mutex); done = it->second->done; }
            if (!done) ++active;
            if (done && state_->jobs.size() >= 32) it = state_->jobs.erase(it); else ++it;
        }
        if (active >= 2) throw Error("unavailable", "Two transfers are already running.");
        if (state_->next == UINT32_MAX) throw Error("unavailable", "Transfer identifier limit reached.");
        const auto id = ++state_->next;
        auto job = std::make_unique<State::Job>();
        auto *pointer = job.get();
        pointer->worker = std::thread([pointer, choice] {
            std::string error;
            try { run(choice.source, choice.destination, choice.moving, pointer->control); }
            catch (const std::exception &failure) { error = failure.what(); }
            catch (...) { error = "Transfer worker failed. Inspect recovery folders before retrying."; }
            std::lock_guard lock(pointer->mutex);
            pointer->error = std::move(error);
            pointer->done = true;
        });
        state_->jobs.emplace(id, std::move(job));
        return pointer->status(id);
    }
    if (method == "files.transfers") {
        std::vector<Json> items;
        for (auto &[id, job] : state_->jobs) items.push_back(job->status(id));
        return Json::array(items);
    }
    const auto id = request.params.get("id").unsigned_number();
    const auto found = state_->jobs.find(id);
    if (found == state_->jobs.end()) throw Error("not_found", "This transfer is no longer available.");
    if (method == "files.transfer_cancel") { g_cancellable_cancel(found->second->control.cancel.get()); return Json{}; }
    return found->second->status(id);
}
}
