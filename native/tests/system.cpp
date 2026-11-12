#include "services/system.hpp"
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <unistd.h>

using namespace meridian;
void require(bool ok) { if (!ok) throw std::runtime_error("system contract failed"); }
int main(int argc, char **argv) {
    const auto program = std::filesystem::path(argv[0]).filename().string();
    if (program == "timedatectl") {
        const auto *root = std::getenv("MERIDIAN_TEST_SYSTEM_ROOT");
        std::ofstream log(std::filesystem::path(root) / "actions", std::ios::app);
        for (int i = 1; i < argc; ++i) log << argv[i] << '\n';
        if (argc > 1 && std::string_view(argv[1]) == "show") std::cout << "Timezone=UTC\nNTP=yes\nNTPSynchronized=no\n";
        return 0;
    }
    if (program == "lpstat") { std::cout << "printer Office is idle\nprinter Desk disabled\n"; return 0; }
    if (program == "scanimage") { std::cout << "device `test:scanner' is a scanner\n"; return 0; }
    char pattern[] = "/tmp/meridian-system-XXXXXX";
    const auto *directory = mkdtemp(pattern);
    require(directory != nullptr);
    const std::filesystem::path root(directory);
    struct Cleanup { std::filesystem::path path; ~Cleanup() { std::filesystem::remove_all(path); } } cleanup{root};
    const auto sysfs = root / "power";
    for (const auto *name : {"BAT0", "AC", "mouse"}) std::filesystem::create_directories(sysfs / name);
    auto write = [&](const char *name, const char *key, const char *value) { std::ofstream(sysfs / name / key) << value; };
    write("BAT0", "type", "Battery"); write("BAT0", "capacity", "72"); write("BAT0", "status", "Discharging");
    write("BAT0", "energy_full", "4000"); write("BAT0", "energy_full_design", "5000");
    write("AC", "type", "Mains"); write("AC", "online", "0");
    write("mouse", "type", "Battery"); write("mouse", "scope", "Device"); write("mouse", "capacity", "50");
    auto state = power_status_at(sysfs);
    require(!state.get("on_ac").boolean());
    require(state.get("batteries").size() == 1);
    require(state.get("batteries").at(0).get("percent").unsigned_number() == 72);
    require(state.get("batteries").at(0).get("health").unsigned_number() == 80);
    std::filesystem::remove(sysfs / "BAT0/capacity");
    require(power_status_at(sysfs).get("batteries").at(0).get("percent").is_null());
    write("BAT0", "present", "0");
    require(power_status_at(sysfs).get("batteries").size() == 0);
    require(power_status_at(root / "missing").get("on_ac").is_null());
    require(!valid_timezone("../etc/passwd") && !valid_timezone("foo//bar") && !valid_timezone("/etc/passwd"));
    const auto bin = root / "bin";
    std::filesystem::create_directory(bin);
    const auto executable = std::filesystem::canonical("/proc/self/exe");
    for (const auto *name : {"timedatectl", "lpstat", "scanimage"}) std::filesystem::create_symlink(executable, bin / name);
    setenv("PATH", bin.c_str(), 1);
    setenv("MERIDIAN_TEST_SYSTEM_ROOT", directory, 1);
    ConfigStore config(root / "config");
    ServiceContext context{config, nullptr};
    const auto time = dispatch_system(Request{"settings.time", Json{}}, context).value();
    require(time.get("timezone").text() == "UTC" && time.get("ntp").boolean() && !time.get("synchronized").boolean());
    const auto change = Request{"settings.time_set", Json::object({{"ntp", Json::boolean(false)}})};
    dispatch_system(change, context);
    require(read_bounded(root / "actions", 4096).value().find("set-ntp\nfalse\n") != std::string::npos);
    const auto before = read_bounded(root / "actions", 4096).value();
    bool rejected = false;
    try { dispatch_system(Request{"settings.time_set", Json::object({{"timezone", Json::string("../etc/passwd")}})}, context); }
    catch (const Error &error) { rejected = error.code == "invalid_request"; }
    require(rejected && read_bounded(root / "actions", 4096).value() == before);
    const auto printers = dispatch_system(Request{"settings.printers", Json{}}, context).value();
    require(printers.get("cups_available").boolean() && printers.get("sane_available").boolean());
    require(printers.get("printers").size() == 2 && printers.get("scanners").size() == 1);
    const auto info = dispatch_system(Request{"settings.system_info", Json{}}, context).value();
    require(!info.get("kernel").text().empty() && info.get("version").text() == "0.1.1");
}
