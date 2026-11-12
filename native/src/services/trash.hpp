#pragma once
#include "services/services.hpp"

namespace meridian {
Json trash_listing(GFile *root, GCancellable *cancel);
void restore_trash_file(GFile *source, std::string_view original, GCancellable *cancel);
std::optional<Json> dispatch_trash(const Request &, ServiceContext &);
}
