#pragma once
#include "core/json.hpp"
#include <string>

namespace meridian {
struct Request { std::string method; Json params; };
const Json &protocol_schema();
void validate_type(const Json &value, std::string_view type);
Request parse_request(std::string_view input, std::string_view surface);
Json surface_info(std::string_view surface);
}
