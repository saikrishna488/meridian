#pragma once
#include "core/gio.hpp"
#include "core/io.hpp"
#include "protocol/validation.hpp"
#include <optional>

namespace meridian {
class Transfers;
struct ServiceContext {
    ConfigStore &config;
    GCancellable *cancel;
    Transfers *transfers = nullptr;
};
std::optional<Json> dispatch_files(const Request &request, ServiceContext &context);
std::optional<Json> dispatch_stores(const Request &request, ServiceContext &context);
std::optional<Json> dispatch_applications(const Request &request, ServiceContext &context);
std::optional<Json> dispatch_startup(const Request &request, ServiceContext &context);
std::optional<Json> dispatch_system(const Request &request, ServiceContext &context);
Json dispatch_request(const Request &request, ServiceContext &context, std::string_view surface);
}
