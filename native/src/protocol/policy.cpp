#include "meridian_policy.h"
#include <algorithm>
#include <iterator>
#include <string_view>

namespace {
struct Rule { const char *method; const char *capability; };
constexpr Rule rules[] = {
#include "request_registry.inc"
};
const Rule *find(const char *method) noexcept {
    if (!method) return nullptr;
    const std::string_view key(method);
    const auto *rule = std::lower_bound(std::begin(rules), std::end(rules), key,
        [](const Rule &entry, std::string_view name) { return entry.method < name; });
    return rule != std::end(rules) && key == rule->method ? rule : nullptr;
}
}
extern "C" {
size_t meridian_request_count(void) { return std::size(rules); }
const char *meridian_request_method(size_t index) {
    return index < std::size(rules) ? rules[index].method : nullptr;
}
const char *meridian_request_capability(const char *method) {
    const auto *rule = find(method);
    return rule ? rule->capability : nullptr;
}
meridian_authorization meridian_request_authorize(
    const char *method, const char *const *grants, size_t count) {
    if (!method || count > 64 || (count && !grants)) return MERIDIAN_INVALID_ARGUMENT;
    for (size_t i = 0; i < count; ++i)
        if (!grants[i]) return MERIDIAN_INVALID_ARGUMENT;
    const auto *rule = find(method);
    if (!rule) return MERIDIAN_UNKNOWN_METHOD;
    const std::string_view required(rule->capability);
    if (required == "None") return MERIDIAN_ALLOWED;
    for (size_t i = 0; i < count; ++i)
        if (required == grants[i]) return MERIDIAN_ALLOWED;
    return MERIDIAN_FORBIDDEN;
}
}
