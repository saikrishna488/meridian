#include "protocol/validation.hpp"
#include "core/error.hpp"
#include <stdexcept>
#include <string>

using namespace meridian;
void require(bool ok) { if (!ok) throw std::runtime_error("validation contract failed"); }
void rejects(std::string_view input, std::string_view surface = "preferences", std::string_view code = "invalid_request") {
    try { parse_request(input, surface); }
    catch (const Error &error) { require(error.code == code); return; }
    throw std::runtime_error("invalid request accepted");
}
int main() {
    require(parse_request(R"({"method":"settings.set_volume","params":{"percent":100}})", "preferences").method == "settings.set_volume");
    for (const auto input : {
        R"({"method":"settings.set_volume","params":{"percent":101}})",
        R"({"method":"settings.set_volume","params":{"percent":-1}})",
        R"({"method":"settings.set_volume","params":{"percent":256}})",
        R"({"method":"settings.set_volume","params":{"percent":true}})",
        R"({"method":"settings.set_volume","params":{"percent":1.0}})",
        R"({"method":"settings.set_volume","params":{"percent":1e0}})",
        R"({"method":"settings.set_volume","params":{"percent":10,"extra":false}})",
        R"({"method":"settings.set_volume","params":{}})",
        R"({"method":"settings.set_volume","params":null})",
        R"({"method":"shell.hello","params":{}})",
        R"({"method":"shell.hello","grants":["Login"]})",
        R"({"method":"shell.hello","method":"session.shutdown"})",
        R"({"method":"shell.hello"} trailing)",
        R"({"method":"unknown"})",
        R"({"method":"settings.capability","params":{"feature":"unknown"}})",
        R"({"method":"settings.input_set","params":{"repeat_rate":0}})"
    }) rejects(input);
    rejects(R"({"method":"terminal.start"})", "preferences", "permission_denied");
    rejects(R"({"method":"settings.users"})", "terminal", "permission_denied");
    rejects(R"({"method":"locker.unlock","params":{"password":"secret"}})", "greeter", "permission_denied");
    // Regression: block-bodied match arms must retain their own capability.
    rejects(R"({"method":"apps.uninstall_plan","params":{"id":"test.desktop"}})", "options", "permission_denied");
    require(parse_request(R"({"method":"apps.uninstall_plan","params":{"id":"test.desktop"}})", "preferences").method == "apps.uninstall_plan");
    require(parse_request(R"({"method":"greeter.power","params":{"action":"power-off"}})", "greeter").method == "greeter.power");
    rejects(R"({"method":"greeter.power","params":{"action":"shutdown"}})", "greeter");
    require(parse_request(R"({"method":"desktop.taskbar_set","params":{}})", "preferences").method == "desktop.taskbar_set");
    require(parse_request(R"({"method":"settings.primary_display_set","params":{}})", "preferences").method == "settings.primary_display_set");
    const auto huge = Json::object({{"method", Json::string("search.query")}, {"params", Json::object({
        {"query", Json::string(std::string(4097, 'x'))}, {"serial", Json::unsigned_number(1)}})}});
    rejects(huge.dump());
    rejects(std::string(2 * 1024 * 1024 + 1, ' '));
    rejects(R"({"method":"shell.hello"})", "invented", "permission_denied");
    const auto surfaces = protocol_schema().get("surfaces");
    const auto methods = protocol_schema().get("methods");
    require(methods.size() / 2 == 155);
    require(surfaces.size() / 2 == 17);
    require(surface_info("terminal").get("capabilities").at(0).text() == "terminal");
    // Every method has exactly the same gate for every surface as its generated rule.
    for (size_t i = 0; i < methods.size(); i += 2) {
        const auto method = methods.at(i);
        const auto capability = methods.at(i + 1).get("capability");
        for (size_t j = 0; j < surfaces.size(); j += 2) {
            const auto surface = surfaces.at(j);
            const auto grants = surfaces.at(j + 1);
            bool allowed = capability.text() == "None";
            for (size_t k = 0; k < grants.size(); ++k) allowed |= grants.at(k).text() == capability.text();
            const auto request = Json::object({{"method", method}}).dump();
            try { parse_request(request, surface.text()); require(allowed); }
            catch (const Error &error) { require(allowed ? error.code == "invalid_request" : error.code == "permission_denied"); }
        }
    }
}
