#include "services/system.hpp"
#include "core/process.hpp"
#include "core/text.hpp"
#include <algorithm>
#include <array>
#include <cstdio>
#include <sys/utsname.h>

namespace meridian {
namespace {
std::string text(const std::filesystem::path &path) {
    try { const auto data = read_bounded(path, 1024 * 1024); return data ? std::string(trim(*data)) : ""; }
    catch (const Error &) { return ""; }
}
std::string field(std::string_view source, std::string_view prefix) {
    for (auto line : lines(source)) if (line.starts_with(prefix)) return std::string(trim(line.substr(prefix.size())));
    return "Unavailable";
}
Json system_info() {
    const auto memory_text = text("/proc/meminfo");
    auto memory_field = field(memory_text, "MemTotal:");
    auto memory = parse_number<double>(trim(std::string_view(memory_field).substr(0, memory_field.find(' '))));
    char memory_value[64];
    if (memory) std::snprintf(memory_value, sizeof memory_value, "%.1f GiB", *memory / 1048576.0);
    const auto os_text = text("/etc/os-release");
    auto os = field(os_text, "PRETTY_NAME=");
    if (os.size() >= 2 && os.front() == '"' && os.back() == '"') os = os.substr(1, os.size() - 2);
    struct utsname identity{};
    uname(&identity);
    return Json::object({{"kernel", Json::string(text("/proc/sys/kernel/osrelease"))}, {"architecture", Json::string(identity.machine)},
        {"hostname", Json::string(text("/proc/sys/kernel/hostname"))}, {"version", Json::string(MERIDIAN_VERSION)},
        {"manufacturer", Json::string(text("/sys/class/dmi/id/sys_vendor"))}, {"model", Json::string(text("/sys/class/dmi/id/product_name"))},
        {"os", Json::string(os)}, {"processor", Json::string(field(text("/proc/cpuinfo"), "model name\t:"))},
        {"memory", Json::string(memory ? memory_value : "Unavailable")}});
}
Json time_state(GCancellable *cancel) {
    const auto output = run_checked({"timedatectl", "show", "--property=Timezone", "--property=NTP", "--property=NTPSynchronized"}, cancel);
    const auto zone = field(output, "Timezone=");
    return Json::object({{"timezone", Json::string(zone == "Unavailable" ? "" : zone)},
        {"ntp", Json::boolean(field(output, "NTP=") == "yes")}, {"synchronized", Json::boolean(field(output, "NTPSynchronized=") == "yes")}});
}
Json printer_inventory(GCancellable *cancel) {
    std::vector<Json> printers, scanners;
    bool cups = false, sane = false;
    try {
        const auto output = run_checked({"lpstat", "-p"}, cancel);
        cups = true;
        for (auto line : lines(output)) if (line.starts_with("printer ")) {
            line.remove_prefix(8);
            printers.push_back(Json::string(line.substr(0, line.find_first_of(" \t"))));
            if (printers.size() == 128) break;
        }
    } catch (const Error &) { check_cancelled(cancel); }
    try {
        const auto output = run_checked({"scanimage", "-L"}, cancel);
        sane = true;
        for (auto line : lines(output)) if (line.starts_with("device ")) {
            scanners.push_back(Json::string(line));
            if (scanners.size() == 128) break;
        }
    } catch (const Error &) { check_cancelled(cancel); }
    return Json::object({{"printers", Json::array(printers)}, {"scanners", Json::array(scanners)},
        {"cups_available", Json::boolean(cups)}, {"sane_available", Json::boolean(sane)}});
}
}
bool valid_timezone(std::string_view value) {
    if (value.empty() || value.size() > 100 || value.starts_with('/') || value.find("..") != std::string_view::npos
        || value.find("//") != std::string_view::npos) return false;
    for (unsigned char c : value) if (!g_ascii_isalnum(c) && std::string_view("_-+/").find(c) == std::string_view::npos) return false;
    const std::filesystem::path root("/usr/share/zoneinfo");
    std::error_code error;
    const auto found = std::filesystem::canonical(root / value, error);
    if (error) return false;
    const auto relative = found.lexically_relative(root);
    return !relative.empty() && *relative.begin() != ".." && std::filesystem::is_regular_file(found, error);
}
Json power_status_at(const std::filesystem::path &root) {
    std::vector<Json> batteries;
    std::vector<bool> external;
    std::error_code error;
    std::filesystem::directory_iterator directories(root, error);
    if (!error) for (const auto &entry : directories) {
        const auto path = entry.path();
        const auto read = [&](const char *key) { return text(path / key); };
        const auto number = [&](const std::string &key) -> std::optional<double> {
            const auto value = parse_number<double>(text(path / key));
            return value && *value >= 0 ? value : std::nullopt;
        };
        const auto ratio = [](std::optional<double> now, std::optional<double> full) -> Json {
            if (!now || !full || *full <= 0) return Json{};
            return Json::unsigned_number(static_cast<uint64_t>(std::clamp(std::round(*now * 100 / *full), 0.0, 100.0)));
        };
        const auto type = read("type");
        if (type == "Battery" && read("present") != "0" && read("scope") != "Device") {
            Json percent, health;
            const auto capacity = number("capacity");
            if (capacity && *capacity <= 100) percent = Json::unsigned_number(static_cast<uint64_t>(std::round(*capacity)));
            for (const char *prefix : {"energy", "charge"}) {
                const std::string key(prefix);
                if (percent.is_null()) percent = ratio(number(key + "_now"), number(key + "_full"));
                if (health.is_null()) health = ratio(number(key + "_full"), number(key + "_full_design"));
            }
            auto status = read("status");
            batteries.push_back(Json::object({{"name", Json::string(path.filename().string())}, {"percent", percent},
                {"status", Json::string(status.empty() ? "Unknown" : status)}, {"health", health}}));
        } else if (type == "Mains" || type == "USB" || type == "USB_C" || type == "USB_PD" || type == "USB_PD_DRP" || type == "Wireless") {
            const auto online = read("online");
            if (online == "0" || online == "1") external.push_back(online == "1");
        }
    }
    std::sort(batteries.begin(), batteries.end(), [](const Json &a, const Json &b) { return a.get("name").text() < b.get("name").text(); });
    return Json::object({{"on_ac", external.empty() ? Json{} : Json::boolean(std::any_of(external.begin(), external.end(), [](bool value) { return value; }))},
        {"batteries", Json::array(batteries)}});
}
std::optional<Json> dispatch_system(const Request &request, ServiceContext &context) {
    if (request.method == "system.info" || request.method == "settings.system_info") return system_info();
    if (request.method == "power.status") return power_status_at("/sys/class/power_supply");
    if (request.method == "settings.printers") return printer_inventory(context.cancel);
    if (request.method == "settings.time") return time_state(context.cancel);
    if (request.method == "settings.time_set") {
        const auto zone = request.params.get("timezone"), ntp = request.params.get("ntp");
        if (zone.is_null() && ntp.is_null()) invalid("Select a time preference to change.");
        if (!zone.is_null() && !valid_timezone(zone.text())) invalid("Unknown or invalid time zone.");
        if (!zone.is_null()) run_checked({"timedatectl", "set-timezone", std::string(zone.text())}, context.cancel);
        if (!ntp.is_null()) run_checked({"timedatectl", "set-ntp", ntp.boolean() ? "true" : "false"}, context.cancel);
        return time_state(context.cancel);
    }
    return std::nullopt;
}
}
