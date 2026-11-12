#pragma once
#include "services/services.hpp"

namespace meridian {
bool valid_timezone(std::string_view value);
Json power_status_at(const std::filesystem::path &root);
std::optional<Json> dispatch_system(const Request &request, ServiceContext &context);
}
