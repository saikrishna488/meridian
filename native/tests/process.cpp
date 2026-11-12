#include "core/process.hpp"
#include <array>
#include <cstdio>
#include <filesystem>
#include <stdexcept>
#include <thread>
#include <unistd.h>

using namespace meridian;
void require(bool ok) { if (!ok) throw std::runtime_error("process contract failed"); }
template<class Action> void rejects(Action action, const char *code) {
    try { action(); } catch (const Error &error) {
        require(error.code == code && std::string(error.what()).find("password-secret") == std::string::npos);
        return;
    }
    throw std::runtime_error("failed process accepted");
}
int main(int argc, char **argv) {
    if (argc > 1) {
        const std::string_view mode(argv[1]);
        if (mode == "literal") { std::printf("%s", argv[2]); return 0; }
        if (mode == "failure") { std::fprintf(stderr, "password-secret: permission denied"); return 1; }
        if (mode == "sleep") { std::this_thread::sleep_for(std::chrono::seconds(3)); return 0; }
        if (mode == "flood") { const std::string data(1200000, 'x'); std::fwrite(data.data(), 1, data.size(), stdout); return 0; }
        if (mode == "echo") {
            std::array<char, 1024> data;
            while (const auto count = read(STDIN_FILENO, data.data(), data.size())) {
                if (count < 0) return 1;
                std::fwrite(data.data(), 1, static_cast<size_t>(count), stdout);
            }
            return 0;
        }
        return 2;
    }
    const auto executable = std::filesystem::canonical("/proc/self/exe").string();
    require(run_checked({executable, "literal", "$(touch /tmp/no);$HOME"}) == "$(touch /tmp/no);$HOME");
    const auto input = std::string(65536, 's');
    require(run_checked({executable, "echo"}, nullptr, std::chrono::seconds(2), input) == input);
    rejects([&] { run_checked({executable, "failure"}); }, "permission_denied");
    rejects([&] { run_checked({executable, "flood"}); }, "failed");
    rejects([&] { run_checked({"/meridian-missing-program"}); }, "unavailable");
    const auto start = std::chrono::steady_clock::now();
    rejects([&] { run_checked({executable, "sleep"}, nullptr, std::chrono::milliseconds(100)); }, "unavailable");
    require(std::chrono::steady_clock::now() - start < std::chrono::seconds(2));
    GObjectPtr<GCancellable> cancel(g_cancellable_new());
    std::thread cancellation([&] { std::this_thread::sleep_for(std::chrono::milliseconds(50)); g_cancellable_cancel(cancel.get()); });
    rejects([&] { run_checked({executable, "sleep"}, cancel.get()); }, "unavailable");
    cancellation.join();
}
