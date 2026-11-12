#include "meridian_backend.h"
#include "core/json.hpp"
#include <cstring>
#include <filesystem>
#include <fstream>
#include <memory>
#include <stdexcept>
#include <sys/stat.h>
#include <unistd.h>

using namespace meridian;
void require(bool ok) { if (!ok) throw std::runtime_error("backend contract failed"); }
using Backend = std::unique_ptr<meridian_backend, decltype(&meridian_backend_destroy)>;
using Client = std::unique_ptr<meridian_client, decltype(&meridian_client_destroy)>;
using Task = std::unique_ptr<meridian_task, decltype(&meridian_task_destroy)>;
Json call(meridian_client *client, const std::string &input) {
    Task task(meridian_client_submit(client, input.data(), input.size()), meridian_task_destroy);
    require(task != nullptr && meridian_task_wait(task.get(), 5000));
    const auto result = Json::parse(meridian_task_reply(task.get()));
    return result;
}
Json request(meridian_client *client, const char *method, const Json &params = Json{}) {
    return call(client, Json::object({{"method", Json::string(method)}, {"params", params}}).dump());
}
int main() {
    char pattern[] = "/tmp/meridian-backend-XXXXXX";
    const auto *directory = mkdtemp(pattern);
    require(directory != nullptr);
    const std::filesystem::path root(directory);
    struct Cleanup { std::filesystem::path path; ~Cleanup() { std::filesystem::remove_all(path); } } cleanup{root};
    Backend backend(meridian_backend_create((root / "config").c_str()), meridian_backend_destroy);
    require(backend != nullptr);
    Client settings(meridian_client_create(backend.get(), "preferences"), meridian_client_destroy);
    Client finder(meridian_client_create(backend.get(), "finder"), meridian_client_destroy);
    Client calendar(meridian_client_create(backend.get(), "calendar"), meridian_client_destroy);
    require(settings && finder && calendar);
    require(!meridian_client_create(backend.get(), "invented"));
    require(request(finder.get(), "shell.hello").get("result").get("surface").text() == "finder");
    require(request(finder.get(), "settings.users").get("error").get("code").text() == "permission_denied");
    const auto idle = Json::object({{"lock_after", Json::unsigned_number(600)}, {"blank_after", Json::unsigned_number(660)}});
    require(request(settings.get(), "settings.idle_set", idle).get("ok").boolean());
    require(request(settings.get(), "settings.idle").get("result").dump() == idle.dump());
    const auto search = Json::object({{"settings", Json::boolean(false)}, {"calculator", Json::boolean(true)}});
    require(request(settings.get(), "settings.search_set", search).get("ok").boolean());
    const auto events = Json::array({Json::object({{"id", Json::string("one")}, {"title", Json::string("Meeting")},
        {"date", Json::string("2026-11-11")}, {"time", Json::string("10:00")}})});
    require(request(calendar.get(), "calendar.save", events).get("ok").boolean());
    require(request(calendar.get(), "calendar.list").get("result").dump() == events.dump());
    const auto files = root / "files";
    std::filesystem::create_directory(files);
    const auto path = [&](const std::filesystem::path &p) { return Json::object({{"path", Json::string(p.string())}}); };
    auto named = [&](const std::filesystem::path &p, const char *name) {
        return Json::object({{"path", Json::string(p.string())}, {"name", Json::string(name)}});
    };
    require(request(finder.get(), "files.mkdir", named(files, "folder")).get("ok").boolean());
    require(!request(finder.get(), "files.mkdir", named(files, "../escape")).get("ok").boolean());
    std::ofstream(files / "note.txt") << "hello";
    std::ofstream(files / ".hidden") << "secret";
    std::filesystem::create_symlink("absent", files / "broken");
    require(request(finder.get(), "files.properties", path(files / "broken")).get("result").get("link_target").text() == "absent");
    const auto listing = request(finder.get(), "files.list", Json::object({{"path", Json::string(files.string())}, {"hidden", Json::boolean(false)}}));
    require(listing.get("result").get("entries").size() == 3);
    require(listing.get("result").get("entries").at(0).get("directory").boolean());
    require(!request(finder.get(), "files.rename", named(files / "note.txt", ".hidden")).get("ok").boolean());
    require(request(finder.get(), "files.rename", named(files / "note.txt", "renamed.txt")).get("ok").boolean());
    require(request(finder.get(), "files.preview", path(files / "renamed.txt")).get("result").get("content").text() == "hello");
    // Exercise transfer dispatch through the public C ABI and capability gate.
    const auto transfer = Json::object({{"path", Json::string((files / "renamed.txt").string())},
        {"destination", Json::string((files / "folder").string())}, {"move_file", Json::boolean(false)}});
    require(request(finder.get(), "files.transfer", transfer).get("ok").boolean());
    require(std::filesystem::file_size(files / "folder/renamed.txt") == 5);
    require(!request(settings.get(), "files.transfer", transfer).get("ok").boolean());
    require(request(finder.get(), "files.transfers").get("result").size() == 0);
    require(mkfifo((files / "pipe").c_str(), 0600) == 0);
    require(request(finder.get(), "files.preview", path(files / "pipe")).get("result").get("kind").text() == "unsupported");
    std::ofstream(files / "big.txt") << std::string(70000, 'x');
    const auto preview = request(finder.get(), "files.preview", path(files / "big.txt")).get("result");
    require(preview.get("content").text().size() == 65536 && preview.get("truncated").boolean());
    require(request(finder.get(), "files.bookmark_set", path(files)).get("result").size() == 1);
    require(request(finder.get(), "files.bookmark_set", path(files)).get("result").size() == 0);
    require(!request(finder.get(), "files.properties", path("/tmp/../etc/passwd")).get("ok").boolean());
    // Backend state survives reopening and remains compatible with stored JSON.
    settings.reset(); finder.reset(); calendar.reset(); backend.reset();
    backend.reset(meridian_backend_create((root / "config").c_str()));
    settings.reset(meridian_client_create(backend.get(), "preferences"));
    require(request(settings.get(), "settings.search").get("result").dump() == search.dump());
    require(request(settings.get(), "settings.idle").get("result").dump() == idle.dump());
    // Admission is bounded even when callers retain completed task handles.
    std::vector<Task> tasks;
    const std::string hello = R"({"method":"shell.hello"})";
    for (unsigned i = 0; i < 300; ++i) tasks.emplace_back(meridian_client_submit(settings.get(), hello.data(), hello.size()), meridian_task_destroy);
    unsigned limited = 0;
    for (const auto &task : tasks) {
        require(task && meridian_task_wait(task.get(), 5000));
        const auto reply = Json::parse(meridian_task_reply(task.get()));
        if (!reply.get("ok").boolean()) { require(reply.get("error").get("code").text() == "unavailable"); ++limited; }
    }
    require(limited > 0);
    // Closing backend first leaves client/task handles safe and replies explicit.
    backend.reset();
    require(request(settings.get(), "shell.hello").get("error").get("code").text() == "unavailable");
    require(!meridian_task_reply(nullptr));
    require(!meridian_client_submit(nullptr, hello.data(), hello.size()));
}
