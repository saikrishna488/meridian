#include "core/process.hpp"
#include <algorithm>
#include <array>
#include <cerrno>
#include <cstring>
#include <fcntl.h>
#include <poll.h>
#include <spawn.h>
#include <pthread.h>
#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>

extern char **environ;
namespace meridian {
namespace {
class Fd {
    int value_ = -1;
public:
    Fd() = default;
    explicit Fd(int fd) : value_(fd) {}
    ~Fd() { reset(); }
    Fd(const Fd &) = delete;
    int get() const { return value_; }
    void reset(int fd = -1) { if (value_ >= 0) close(value_); value_ = fd; }
};
// Pipe writes must not terminate the host when a child exits early. Block only
// this thread, consume only a newly generated SIGPIPE, and restore its mask.
class SigpipeBlock {
    sigset_t previous_{}, blocked_{};
    bool pending_ = false;
public:
    SigpipeBlock() {
        sigemptyset(&blocked_); sigaddset(&blocked_, SIGPIPE);
        const int error = pthread_sigmask(SIG_BLOCK, &blocked_, &previous_);
        if (error) throw Error("failed", "Could not protect operation input.");
        sigset_t pending;
        sigpending(&pending);
        pending_ = sigismember(&pending, SIGPIPE) == 1;
    }
    ~SigpipeBlock() {
        if (!pending_) {
            const timespec immediate{};
            while (sigtimedwait(&blocked_, nullptr, &immediate) < 0 && errno == EINTR) {}
        }
        pthread_sigmask(SIG_SETMASK, &previous_, nullptr);
    }
};
void checked(int error) { if (error) throw Error("failed", "Could not prepare system operation."); }
void nonblocking(int fd) {
    const auto flags = fcntl(fd, F_GETFL);
    if (flags < 0 || fcntl(fd, F_SETFL, flags | O_NONBLOCK) < 0) throw Error("failed", "Could not configure operation streams.");
}
bool drain(int fd, std::string &output) {
    std::array<char, 8192> buffer;
    for (unsigned i = 0; i < 32; ++i) {
        const auto count = read(fd, buffer.data(), buffer.size());
        if (count < 0) {
            if (errno == EINTR) continue;
            if (errno == EAGAIN || errno == EWOULDBLOCK) return false;
            throw Error("failed", "Could not read operation output.");
        }
        if (!count) return true;
        if (output.size() + static_cast<size_t>(count) > 1024 * 1024) throw Error("failed", "System operation output exceeds 1 MiB.");
        output.append(buffer.data(), static_cast<size_t>(count));
    }
    return false;
}
}
ProcessResult run_process(const std::vector<std::string> &arguments, GCancellable *cancel,
    std::chrono::milliseconds timeout, std::string_view input) {
    if (arguments.empty() || arguments.size() > 256 || input.size() > 65536 || timeout.count() <= 0 || timeout > std::chrono::seconds(60))
        invalid("Invalid system operation.");
    for (const auto &argument : arguments) if (argument.find('\0') != std::string::npos) invalid("Invalid operation argument.");
    check_cancelled(cancel);
    int output_pipe[2], error_pipe[2], input_pair[2];
    if (pipe2(output_pipe, O_CLOEXEC) < 0) throw Error("failed", "Could not create operation stream.");
    Fd output_read(output_pipe[0]), output_write(output_pipe[1]);
    if (pipe2(error_pipe, O_CLOEXEC) < 0) throw Error("failed", "Could not create operation stream.");
    Fd error_read(error_pipe[0]), error_write(error_pipe[1]);
    if (pipe2(input_pair, O_CLOEXEC) < 0) throw Error("failed", "Could not create operation input.");
    Fd input_read(input_pair[0]), input_write(input_pair[1]);
    nonblocking(output_read.get()); nonblocking(error_read.get()); nonblocking(input_write.get());
    struct Actions {
        posix_spawn_file_actions_t value;
        Actions() { checked(posix_spawn_file_actions_init(&value)); }
        ~Actions() { posix_spawn_file_actions_destroy(&value); }
    } actions;
    struct Attributes {
        posix_spawnattr_t value;
        Attributes() { checked(posix_spawnattr_init(&value)); }
        ~Attributes() { posix_spawnattr_destroy(&value); }
    } attributes;
    checked(posix_spawn_file_actions_adddup2(&actions.value, input_read.get(), STDIN_FILENO));
    checked(posix_spawn_file_actions_adddup2(&actions.value, output_write.get(), STDOUT_FILENO));
    checked(posix_spawn_file_actions_adddup2(&actions.value, error_write.get(), STDERR_FILENO));
    checked(posix_spawnattr_setflags(&attributes.value, POSIX_SPAWN_SETPGROUP));
    checked(posix_spawnattr_setpgroup(&attributes.value, 0));
    std::vector<char *> argv;
    for (const auto &argument : arguments) argv.push_back(const_cast<char *>(argument.c_str()));
    argv.push_back(nullptr);
    std::vector<std::string> environment;
    for (size_t i = 0; environ[i]; ++i) if (!std::string_view(environ[i]).starts_with("LC_ALL=")) environment.emplace_back(environ[i]);
    environment.emplace_back("LC_ALL=C");
    std::vector<char *> envp;
    for (auto &variable : environment) envp.push_back(variable.data());
    envp.push_back(nullptr);
    pid_t pid = -1;
    const auto spawned = posix_spawnp(&pid, argv.front(), &actions.value, &attributes.value, argv.data(), envp.data());
    if (spawned) throw Error("unavailable", "Required system program is unavailable.");
    struct Child {
        pid_t pid;
        bool reaped = false;
        ~Child() {
            if (reaped) return;
            kill(-pid, SIGKILL);
            while (waitpid(pid, nullptr, 0) < 0 && errno == EINTR) {}
        }
    } child{pid};
    output_write.reset(); error_write.reset(); input_read.reset();
    if (input.empty()) input_write.reset();
    SigpipeBlock signal_guard;
    ProcessResult result;
    const auto deadline = std::chrono::steady_clock::now() + timeout;
    size_t written = 0;
    while (true) {
        check_cancelled(cancel);
        if (std::chrono::steady_clock::now() >= deadline) throw Error("unavailable", "System operation timed out.");
        if (input_write.get() >= 0) {
            const auto count = write(input_write.get(), input.data() + written, input.size() - written);
            if (count < 0 && errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR)
                throw Error("failed", "System operation could not receive input.");
            if (count > 0) written += static_cast<size_t>(count);
            if (written == input.size()) input_write.reset();
        }
        if (output_read.get() >= 0 && drain(output_read.get(), result.output)) output_read.reset();
        if (error_read.get() >= 0 && drain(error_read.get(), result.diagnostics)) error_read.reset();
        int status = 0;
        const auto reaped = waitpid(pid, &status, WNOHANG);
        if (reaped == pid) {
            child.reaped = true;
            result.status = WIFEXITED(status) ? WEXITSTATUS(status) : 128 + (WIFSIGNALED(status) ? WTERMSIG(status) : 0);
            if (output_read.get() >= 0) drain(output_read.get(), result.output);
            if (error_read.get() >= 0) drain(error_read.get(), result.diagnostics);
            return result;
        }
        if (reaped < 0 && errno != EINTR) throw Error("failed", "Could not wait for system operation.");
        std::array<pollfd, 3> fds{{{output_read.get(), POLLIN, 0}, {error_read.get(), POLLIN, 0}, {input_write.get(), POLLOUT, 0}}};
        const auto remaining = std::chrono::duration_cast<std::chrono::milliseconds>(deadline - std::chrono::steady_clock::now()).count();
        if (poll(fds.data(), fds.size(), static_cast<int>(std::clamp<int64_t>(remaining, 0, 25))) < 0 && errno != EINTR)
            throw Error("failed", "Could not monitor system operation.");
    }
}
std::string run_checked(const std::vector<std::string> &arguments, GCancellable *cancel,
    std::chrono::milliseconds timeout, std::string_view input) {
    auto result = run_process(arguments, cancel, timeout, input);
    if (!result.status) return result.output;
    std::transform(result.diagnostics.begin(), result.diagnostics.end(), result.diagnostics.begin(), [](unsigned char c) { return g_ascii_tolower(c); });
    if (result.diagnostics.find("permission") != std::string::npos || result.diagnostics.find("not authorized") != std::string::npos
        || result.diagnostics.find("not permitted") != std::string::npos) throw Error("permission_denied", "System authorization denied the operation.");
    if (result.diagnostics.find("not running") != std::string::npos || result.diagnostics.find("could not connect") != std::string::npos
        || result.diagnostics.find("serviceunknown") != std::string::npos) throw Error("unavailable", "System service unavailable.");
    throw Error("failed", "The system service could not complete this operation.");
}
}
