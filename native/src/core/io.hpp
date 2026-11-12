#pragma once
#include "core/json.hpp"
#include <filesystem>
#include <mutex>
#include <optional>

namespace meridian {
constexpr size_t config_limit = 4 * 1024 * 1024;
std::filesystem::path absolute_path(std::string_view path);
void valid_filename(std::string_view name);
std::optional<std::string> read_bounded(const std::filesystem::path &path, size_t limit);
void write_atomic(const std::filesystem::path &path, std::string_view data, bool exclusive = false);
class ConfigStore {
    std::filesystem::path root_;
    std::recursive_mutex mutex_;
    std::filesystem::path path(const char *name) const;
public:
    explicit ConfigStore(std::filesystem::path root);
    std::recursive_mutex &mutex() { return mutex_; }
    const std::filesystem::path &root() const { return root_; }
    Json load(const char *name, const Json &fallback, bool versioned = false);
    void save(const char *name, const Json &value, bool versioned = false);
};
}
