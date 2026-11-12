#include "protocol/validation.hpp"
#include "core/error.hpp"
#include <algorithm>
#include <limits>

namespace meridian {
const Json &protocol_schema() {
    // sd-json references are not atomic; each worker owns its schema tree.
    thread_local const Json schema = Json::parse(
#include "schema.inc"
    );
    return schema;
}
void validate_type(const Json &value, std::string_view type) {
    if (type.starts_with("Option<") && type.ends_with('>')) {
        if (!value.is_null()) validate_type(value, type.substr(7, type.size() - 8));
        return;
    }
    if (!value.exists()) invalid("Missing required parameter.");
    if (type == "String") { if (!value.is_string()) invalid(); return; }
    if (type == "bool") { if (!value.is_bool()) invalid(); return; }
    if (type == "f64") { if (!value.is_number()) invalid(); return; }
    if (type == "u8" || type == "u16" || type == "u32") {
        const uint64_t max = type == "u8" ? 255 : type == "u16" ? 65535 : UINT32_MAX;
        if (!value.is_unsigned() || value.unsigned_number() > max) invalid();
        return;
    }
    if (type == "i32") {
        if (!value.is_integer() || value.integer() < INT32_MIN || value.integer() > INT32_MAX) invalid();
        return;
    }
    if (type.starts_with("Vec<") && type.ends_with('>')) {
        if (!value.is_array()) invalid();
        for (size_t i = 0; i < value.size(); ++i) validate_type(value.at(i), type.substr(4, type.size() - 5));
        return;
    }
    if (type == "std::collections::HashMap<String,DesktopPosition>") {
        if (!value.is_object()) invalid();
        for (size_t i = 1; i < value.size(); i += 2) validate_type(value.at(i), "DesktopPosition");
        return;
    }
    const auto definition = protocol_schema().get("types").get(std::string(type).c_str());
    if (!definition.exists()) throw Error("failed", "Native protocol schema missing.");
    if (definition.get("kind").text() == "enum") {
        if (!value.is_string()) invalid();
        const auto values = definition.get("values");
        for (size_t i = 0; i < values.size(); ++i) if (values.at(i).text() == value.text()) return;
        invalid("Unknown enum value.");
    }
    if (!value.is_object()) invalid();
    const auto fields = definition.get("fields");
    for (size_t i = 0; i < fields.size(); ++i) {
        const auto field = fields.at(i);
        const auto item = value.get(std::string(field.get("name").text()).c_str());
        if (!item.exists() && field.get("optional").boolean()) continue;
        validate_type(item, field.get("type").text());
    }
    if (definition.get("closed").boolean()) {
        for (size_t i = 0; i < value.size(); i += 2) {
            bool found = false;
            for (size_t j = 0; j < fields.size(); ++j)
                if (fields.at(j).get("name").text() == value.at(i).text()) { found = true; break; }
            if (!found) invalid("Unknown parameter field.");
        }
    }
}
Request parse_request(std::string_view input, std::string_view surface) {
    const auto root = Json::parse(input);
    if (!root.is_object() || !root.get("method").is_string()) invalid();
    for (size_t i = 0; i < root.size(); i += 2)
        if (root.at(i).text() != "method" && root.at(i).text() != "params") invalid("Unknown request field.");
    Request request{std::string(root.get("method").text()), root.get("params")};
    const auto rule = protocol_schema().get("methods").get(request.method.c_str());
    if (!rule.exists()) invalid("Unknown request method.");
    const auto grants = protocol_schema().get("surfaces").get(std::string(surface).c_str());
    if (!grants.exists()) throw Error("permission_denied", "Unknown client surface.");
    const auto required = rule.get("capability").text();
    bool allowed = required == "None";
    for (size_t i = 0; i < grants.size(); ++i) if (grants.at(i).text() == required) allowed = true;
    if (!allowed) throw Error("permission_denied", "Surface lacks the required capability.");
    const auto type = rule.get("type").text();
    if (type == "unit") {
        if (request.params.exists() && !request.params.is_null()) invalid("This request has no parameters.");
    } else validate_type(request.params, type);
    if (request.method == "search.query" && request.params.get("query").text().size() > 4096)
        invalid("Search text exceeds 4096 bytes.");
    if ((request.method == "settings.set_volume" || request.method == "settings.set_brightness")
        && request.params.get("percent").unsigned_number() > 100) invalid("Percent must be 0–100.");
    return request;
}
Json surface_info(std::string_view surface) {
    const auto grants = protocol_schema().get("surfaces").get(std::string(surface).c_str());
    if (!grants.exists()) throw Error("permission_denied", "Unknown client surface.");
    // Convert the Rust variant names used internally to wire kebab-case names.
    std::vector<Json> capabilities;
    for (size_t i = 0; i < grants.size(); ++i) {
        std::string wire;
        const auto grant = grants.at(i);
        for (char c : grant.text()) {
            if (c >= 'A' && c <= 'Z') { if (!wire.empty()) wire += '-'; wire += char(c + ('a' - 'A')); }
            else wire += c;
        }
        capabilities.push_back(Json::string(wire));
    }
    return Json::object({{"surface", Json::string(surface)}, {"capabilities", Json::array(capabilities)}, {"dev", Json::boolean(false)}});
}
}
