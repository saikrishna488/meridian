#pragma once
#include <glib.h>
#include <optional>
#include <string_view>
#include <vector>

namespace meridian::search {
struct Prepared {
    std::vector<gunichar> chars;
    std::vector<bool> word_start;
    explicit Prepared(std::string_view text = {});
};
std::vector<std::vector<gunichar>> tokens(std::string_view query);
std::optional<int> fuzzy(const std::vector<gunichar> &token, const Prepared &candidate);
std::optional<int> word_prefix(const std::vector<gunichar> &token, const Prepared &candidate);
struct Fields {
    Prepared name, description;
    std::vector<Prepared> aliases;
    std::optional<int> score(const std::vector<std::vector<gunichar>> &query) const;
};
}
