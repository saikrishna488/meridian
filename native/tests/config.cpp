#include "core/io.hpp"
#include "core/error.hpp"
#include <fstream>
#include <stdexcept>
#include <sys/stat.h>
#include <unistd.h>

using namespace meridian;
void require(bool ok) { if (!ok) throw std::runtime_error("config contract failed"); }
int main() {
    char pattern[] = "/tmp/meridian-config-XXXXXX";
    const auto *root = mkdtemp(pattern);
    require(root != nullptr);
    struct Cleanup { std::filesystem::path path; ~Cleanup() { std::filesystem::remove_all(path); } } cleanup{root};
    ConfigStore store(root);
    const auto path = std::filesystem::path(root) / "test.json";
    const auto value = Json::array({Json::string("saved")});
    require(store.load("test.json", value).dump() == value.dump());
    std::ofstream(path) << "{interrupted";
    store.save("test.json", value, true);
    require(store.load("test.json", Json{}, true).dump() == value.dump());
    bool backup = false;
    for (const auto &entry : std::filesystem::directory_iterator(root)) {
        if (entry.path() == path) continue;
        backup = true;
        require(read_bounded(entry.path(), config_limit).value() == "{interrupted");
    }
    require(backup);
    struct stat metadata{};
    require(stat(path.c_str(), &metadata) == 0 && (metadata.st_mode & 0777) == 0600);
    const std::string future = R"({"schema_version":2,"data":[]})";
    std::ofstream(path) << future;
    bool rejected = false;
    try { store.save("test.json", value, true); } catch (const Error &) { rejected = true; }
    require(rejected && read_bounded(path, config_limit).value() == future);
    std::ofstream(path) << "[\"legacy\"]";
    require(store.load("test.json", Json{}, true).at(0).text() == "legacy");
    store.save("test.json", value, true);
    require(Json::parse(read_bounded(path, config_limit).value()).get("schema_version").unsigned_number() == 1);
    std::filesystem::resize_file(path, config_limit + 1);
    rejected = false;
    try { store.save("test.json", value); } catch (const Error &) { rejected = true; }
    require(rejected && std::filesystem::file_size(path) == config_limit + 1);
    rejected = false;
    try { store.save("../unsafe", value); } catch (const Error &) { rejected = true; }
    require(rejected);
}
