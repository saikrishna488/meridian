#pragma once
#include "services/services.hpp"

namespace meridian {
using StartupRoots = std::vector<std::filesystem::path>;
Json startup_list(const StartupRoots &roots, std::string_view desktops, GCancellable *cancel = nullptr);
Json startup_set(const StartupRoots &roots, std::string_view desktops, std::string_view id, bool enabled, GCancellable *cancel = nullptr);
Json startup_add(const StartupRoots &roots, std::string_view desktops, std::string_view id,
                 const std::filesystem::path &source, GCancellable *cancel = nullptr);
std::optional<Json> dispatch_startup(const Request &request, ServiceContext &context);
}
