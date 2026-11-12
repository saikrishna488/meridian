#include "core/json.hpp"
#include "core/error.hpp"
#include <cmath>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <unordered_set>

namespace meridian {
namespace {
void checked(int result) { if (result < 0) throw Error("failed", "JSON allocation or encoding failed."); }
void check_tree(const Json &value, unsigned depth) {
    if (depth > 64) invalid("JSON nesting exceeds 64 levels.");
    if (value.is_array()) {
        for (size_t i = 0; i < value.size(); ++i) check_tree(value.at(i), depth + 1);
    } else if (value.is_object()) {
        std::unordered_set<std::string_view> keys;
        for (size_t i = 0; i < value.size(); i += 2) {
            if (!keys.insert(value.at(i).text()).second) invalid("Duplicate JSON field.");
            check_tree(value.at(i + 1), depth + 1);
        }
    }
}
}
Json Json::parse(std::string_view input, size_t limit) {
    if (input.size() > limit || input.find('\0') != std::string_view::npos) invalid("Invalid JSON size or encoding.");
    std::string terminated(input);
    struct Wipe {
        std::string &value;
        ~Wipe() { if (!value.empty()) explicit_bzero(value.data(), value.size()); }
    } wipe{terminated};
    Json result;
    if (sd_json_parse(terminated.c_str(), SD_JSON_PARSE_SENSITIVE, &result.value_, nullptr, nullptr) < 0)
        invalid("Malformed JSON.");
    check_tree(result, 0);
    return result;
}
Json Json::string(std::string_view value) {
    Json result;
    checked(sd_json_variant_new_stringn(&result.value_, value.data(), value.size()));
    return result;
}
Json Json::boolean(bool value) { Json r; checked(sd_json_variant_new_boolean(&r.value_, value)); return r; }
Json Json::number(double value) {
    if (!std::isfinite(value)) throw Error("failed", "Non-finite JSON number.");
    Json r; checked(sd_json_variant_new_real(&r.value_, value)); return r;
}
Json Json::integer(int64_t value) { Json r; checked(sd_json_variant_new_integer(&r.value_, value)); return r; }
Json Json::unsigned_number(uint64_t value) { Json r; checked(sd_json_variant_new_unsigned(&r.value_, value)); return r; }
Json Json::array(const std::vector<Json> &values) {
    std::vector<sd_json_variant *> raw;
    raw.reserve(values.size());
    for (const auto &value : values) raw.push_back(value.value_);
    Json result; checked(sd_json_variant_new_array(&result.value_, raw.data(), raw.size())); return result;
}
Json Json::object(std::initializer_list<std::pair<std::string_view, Json>> fields) {
    std::vector<Json> items;
    for (const auto &[key, value] : fields) { items.push_back(string(key)); items.push_back(value); }
    std::vector<sd_json_variant *> raw;
    for (const auto &item : items) raw.push_back(item.value_);
    Json result; checked(sd_json_variant_new_object(&result.value_, raw.data(), raw.size())); return result;
}
std::string_view Json::text() const { if (!is_string()) invalid(); return sd_json_variant_string(value_); }
bool Json::boolean() const { if (!is_bool()) invalid(); return sd_json_variant_boolean(value_); }
double Json::number() const { if (!is_number()) invalid(); return sd_json_variant_real(value_); }
uint64_t Json::unsigned_number() const { if (!is_unsigned()) invalid(); return sd_json_variant_unsigned(value_); }
int64_t Json::integer() const { if (!is_integer()) invalid(); return sd_json_variant_integer(value_); }
std::string Json::dump() const {
    if (!value_) return "null";
    char *raw = nullptr;
    checked(sd_json_variant_format(value_, static_cast<sd_json_format_flags_t>(0), &raw));
    std::unique_ptr<char, decltype(&std::free)> owned(raw, std::free);
    return raw;
}
void Json::set(const char *key, const Json &value) {
    checked(sd_json_variant_set_field(&value_, key, value.value_));
}
}
