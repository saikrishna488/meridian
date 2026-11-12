#pragma once
#include <charconv>
#include <cmath>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

namespace meridian {
inline std::string_view trim(std::string_view value) {
    const auto first = value.find_first_not_of(" \t\n\r\f\v");
    if (first == std::string_view::npos) return {};
    return value.substr(first, value.find_last_not_of(" \t\n\r\f\v") - first + 1);
}
inline std::vector<std::string_view> lines(std::string_view value) {
    std::vector<std::string_view> result;
    while (!value.empty()) {
        const auto end = value.find('\n');
        result.push_back(value.substr(0, end));
        if (end == std::string_view::npos) break;
        value.remove_prefix(end + 1);
    }
    return result;
}
template<class T> std::optional<T> parse_number(std::string_view text) {
    T value{};
    text = trim(text);
    if (text.empty()) return std::nullopt;
    const auto [end, error] = std::from_chars(text.data(), text.data() + text.size(), value);
    if (error != std::errc{} || end != text.data() + text.size()) return std::nullopt;
    if constexpr (std::is_floating_point_v<T>) if (!std::isfinite(value)) return std::nullopt;
    return value;
}
}
