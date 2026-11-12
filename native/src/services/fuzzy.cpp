#include "services/fuzzy.hpp"
#include <algorithm>
#include <limits>

namespace meridian::search {
Prepared::Prepared(std::string_view text) {
    if (text.empty()) return;
    gunichar previous = 0;
    for (const char *p = text.data(), *end = p + text.size(); p < end; p = g_utf8_next_char(p)) {
        const auto c = g_utf8_get_char(p);
        word_start.push_back(chars.empty() || (!g_unichar_isalnum(previous) && g_unichar_isalnum(c))
            || (g_unichar_islower(previous) && g_unichar_isupper(c)));
        chars.push_back(g_unichar_tolower(c));
        previous = c;
    }
}
std::vector<std::vector<gunichar>> tokens(std::string_view query) {
    std::vector<std::vector<gunichar>> result;
    bool space = true;
    for (const char *p = query.data(), *end = p + query.size(); p < end; p = g_utf8_next_char(p)) {
        const auto c = g_utf8_get_char(p);
        if (g_unichar_isspace(c)) { space = true; continue; }
        if (space) result.emplace_back();
        result.back().push_back(g_unichar_tolower(c));
        space = false;
    }
    return result;
}
namespace {
bool matches_after(const std::vector<gunichar> &token, size_t index, const Prepared &text, size_t position) {
    for (; index < token.size(); ++index) {
        const auto it = std::find(text.chars.begin() + position, text.chars.end(), token[index]);
        if (it == text.chars.end()) return false;
        position = static_cast<size_t>(it - text.chars.begin()) + 1;
    }
    return true;
}
std::optional<int> align(const std::vector<gunichar> &token, const Prepared &text, size_t start) {
    int score = -static_cast<int>(std::min(start, size_t{12})), gap_penalty = 0;
    std::optional<size_t> previous;
    size_t position = start;
    for (size_t i = 0; i < token.size(); ++i) {
        auto it = std::find(text.chars.begin() + position, text.chars.end(), token[i]);
        if (it == text.chars.end()) return std::nullopt;
        size_t chosen = static_cast<size_t>(it - text.chars.begin());
        if (previous && chosen != *previous + 1 && !text.word_start[chosen]) {
            for (size_t j = chosen; j < text.chars.size(); ++j) {
                if (text.chars[j] == token[i] && text.word_start[j] && matches_after(token, i + 1, text, j + 1)) {
                    chosen = j; break;
                }
            }
        }
        score += 16;
        if (text.word_start[chosen]) score += 24;
        if (chosen == 0) score += 32;
        if (previous) {
            if (chosen == *previous + 1) score += 20;
            else gap_penalty = std::min(30, gap_penalty + static_cast<int>(std::min(chosen - *previous - 1, size_t{10})) * 3);
        }
        previous = chosen;
        position = chosen + 1;
    }
    if (position - start == token.size() && text.word_start[start]
        && (position == text.chars.size() || !g_unichar_isalnum(text.chars[position]))) score += 24;
    return score - gap_penalty;
}
}
std::optional<int> fuzzy(const std::vector<gunichar> &token, const Prepared &candidate) {
    if (token.empty() || token.size() > candidate.chars.size()) return std::nullopt;
    std::optional<int> best;
    for (size_t i = 0; i < candidate.chars.size(); ++i) {
        if (candidate.chars[i] != token.front()) continue;
        const auto score = align(token, candidate, i);
        if (score && (!best || *score > *best)) best = score;
    }
    return best;
}
std::optional<int> word_prefix(const std::vector<gunichar> &token, const Prepared &candidate) {
    if (token.empty() || token.size() > candidate.chars.size()) return std::nullopt;
    for (size_t i = 0; i <= candidate.chars.size() - token.size(); ++i)
        if (candidate.word_start[i] && std::equal(token.begin(), token.end(), candidate.chars.begin() + i))
            return static_cast<int>(token.size()) * 16 + 24;
    return std::nullopt;
}
std::optional<int> Fields::score(const std::vector<std::vector<gunichar>> &query) const {
    int total = 0;
    for (const auto &token : query) {
        auto best = fuzzy(token, name);
        for (const auto &alias : aliases) if (const auto score = fuzzy(token, alias)) {
            const int weighted = *score * 70 / 100;
            if (!best || weighted > *best) best = weighted;
        }
        if (const auto score = word_prefix(token, description)) {
            const int weighted = *score * 40 / 100;
            if (!best || weighted > *best) best = weighted;
        }
        if (!best) return std::nullopt;
        total += *best;
    }
    return total;
}
}
