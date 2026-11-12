#include "services/transfers.hpp"
#include <chrono>
#include <fstream>
#include <stdexcept>
#include <sys/stat.h>
#include <thread>
#include <unistd.h>

using namespace meridian;
namespace fs = std::filesystem;
void require(bool ok) { if (!ok) throw std::runtime_error("transfer contract failed"); }
template<class Action> void rejects(Action action) {
    try { action(); } catch (const std::exception &) { return; }
    throw std::runtime_error("unsafe transfer accepted");
}
Json choice(const fs::path &from, const fs::path &to, bool moving = false) {
    return Json::object({{"path", Json::string(from.string())}, {"destination", Json::string(to.string())}, {"move_file", Json::boolean(moving)}});
}
Json call(Transfers &transfers, const char *method, const Json &params = {}, GCancellable *cancel = nullptr) {
    auto result = transfers.dispatch({method, params}, cancel);
    require(result.has_value());
    return *result;
}
Json wait(Transfers &transfers, const Json &job) {
    const auto id = Json::object({{"id", job.get("id")}});
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(10);
    while (std::chrono::steady_clock::now() < deadline) {
        auto status = call(transfers, "files.transfer_status", id);
        if (status.get("done").boolean()) return status;
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    throw std::runtime_error("transfer did not finish");
}
int main(int argc, char **argv) {
    char pattern[] = "/tmp/meridian-transfers-XXXXXX";
    require(mkdtemp(pattern));
    const fs::path root(pattern);
    struct Cleanup { fs::path root; ~Cleanup() { fs::remove_all(root); } } cleanup{root};
    fs::create_directories(root / "source/tree/sub");
    fs::create_directory(root / "destination");
    std::ofstream(root / "source/tree/sub/text") << "content";
    fs::create_symlink("missing", root / "source/tree/link");
    require(chmod((root / "source/tree/sub/text").c_str(), 0640) == 0);
    Transfers transfers;
    const auto copy = choice(root / "source/tree", root / "destination");
    const auto finished = wait(transfers, call(transfers, "files.transfer_start", copy));
    require(finished.get("error").is_null());
    require(finished.get("bytes").number() == 7 && finished.get("total").number() == 7);
    require(finished.get("entries").number() == 4);
    require(fs::read_symlink(root / "destination/tree/link") == "missing");
    struct stat info{};
    require(stat((root / "destination/tree/sub/text").c_str(), &info) == 0 && (info.st_mode & 0777) == 0640);
    rejects([&] { call(transfers, "files.transfer", copy); });
    require(fs::file_size(root / "destination/tree/sub/text") == 7);
    rejects([&] { call(transfers, "files.transfer", choice(root / "source/tree", root / "source/tree/sub")); });
    fs::create_symlink(root / "source/tree/sub", root / "inside");
    rejects([&] { call(transfers, "files.transfer", choice(root / "source/tree", root / "inside")); });
    fs::create_directory(root / "moved");
    call(transfers, "files.transfer", choice(root / "source/tree", root / "moved", true));
    require(!fs::exists(root / "source/tree") && fs::exists(root / "moved/tree/sub/text"));
    require(mkfifo((root / "source/pipe").c_str(), 0600) == 0);
    rejects([&] { call(transfers, "files.transfer", choice(root / "source/pipe", root / "destination")); });
    require(!fs::exists(root / "destination/pipe"));
    GObjectPtr<GCancellable> cancel(g_cancellable_new());
    g_cancellable_cancel(cancel.get());
    rejects([&] { call(transfers, "files.transfer", choice(root / "moved/tree", root / "source"), cancel.get()); });
    require(!fs::exists(root / "source/tree"));
    rejects([&] { call(transfers, "files.transfer_status", Json::object({{"id", Json::unsigned_number(999)}})); });
    const auto failed = wait(transfers, call(transfers, "files.transfer_start", copy));
    require(!failed.get("error").is_null());
    require(call(transfers, "files.transfers").size() == 2);
    // Cancellation and backend destruction must leave no published partial data.
    std::ofstream large(root / "source/large", std::ios::binary);
    large.seekp(64 * 1024 * 1024 - 1); large.put('x'); large.close();
    const auto job = call(transfers, "files.transfer_start", choice(root / "source/large", root / "destination"));
    call(transfers, "files.transfer_cancel", Json::object({{"id", job.get("id")}}));
    require(!wait(transfers, job).get("error").is_null());
    require(!fs::exists(root / "destination/large") && fs::exists(root / "source/large"));
    {
        Transfers closing;
        call(closing, "files.transfer_start", choice(root / "source/large", root / "destination"));
    }
    for (const auto &entry : fs::directory_iterator(root / "destination")) require(!entry.path().filename().string().starts_with(".meridian-transfer-"));
    if (argc == 2) {
        // Opt-in real EXDEV coverage; only this test's private fixtures move.
        auto other_pattern = (fs::path(argv[1]) / "meridian-transfer-test-XXXXXX").string();
        require(mkdtemp(other_pattern.data()));
        Cleanup other{other_pattern};
        struct stat first{}, second{};
        require(stat(root.c_str(), &first) == 0 && stat(other.root.c_str(), &second) == 0 && first.st_dev != second.st_dev);
        call(transfers, "files.transfer", choice(root / "moved/tree", other.root, true));
        require(!fs::exists(root / "moved/tree") && fs::file_size(other.root / "tree/sub/text") == 7);
        require(fs::read_symlink(other.root / "tree/link") == "missing");
        // Move back across the boundary, then ensure collisions preserve both sides.
        call(transfers, "files.transfer", choice(other.root / "tree", root / "source", true));
        require(!fs::exists(other.root / "tree"));
        fs::create_directory(other.root / "tree");
        std::ofstream(other.root / "tree/keep") << "keep";
        rejects([&] { call(transfers, "files.transfer", choice(root / "source/tree", other.root, true)); });
        require(fs::file_size(other.root / "tree/keep") == 4 && fs::exists(root / "source/tree/sub/text"));
        for (const auto &parent : {root / "source", root / "moved", other.root})
            for (const auto &entry : fs::directory_iterator(parent)) require(!entry.path().filename().string().starts_with(".meridian-transfer-"));
    }
}
