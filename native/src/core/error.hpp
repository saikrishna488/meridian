#pragma once
#include <stdexcept>
#include <string>

namespace meridian {
class Error : public std::runtime_error {
public:
    const std::string code;
    Error(std::string code, const std::string &message)
        : std::runtime_error(message), code(std::move(code)) {}
};
[[noreturn]] inline void invalid(const char *message = "Invalid request.") {
    throw Error("invalid_request", message);
}
}
