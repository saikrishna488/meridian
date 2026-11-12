#include "meridian_policy.h"
#include <cstring>
#include <stdexcept>
#include <string_view>
void require(bool ok) { if (!ok) throw std::runtime_error("policy contract failed"); }
int main() {
    require(meridian_request_count() == 155);
    for (size_t i = 0; i < meridian_request_count(); ++i) {
        const char *method = meridian_request_method(i);
        const char *capability = meridian_request_capability(method);
        require(capability != nullptr);
        if (i) require(std::string_view(meridian_request_method(i - 1)) < method);
        const char *grants[] = {capability};
        require(meridian_request_authorize(method, grants, 1) == MERIDIAN_ALLOWED);
        const auto expected = std::strcmp(capability, "None") == 0 ? MERIDIAN_ALLOWED : MERIDIAN_FORBIDDEN;
        require(meridian_request_authorize(method, nullptr, 0) == expected);
        const char *wrong[] = {"unknown-capability"};
        require(meridian_request_authorize(method, wrong, 1) == expected);
    }
    require(!meridian_request_method(155));
    require(!meridian_request_capability("bogus"));
    require(meridian_request_authorize("bogus", nullptr, 0) == MERIDIAN_UNKNOWN_METHOD);
    require(meridian_request_authorize(nullptr, nullptr, 0) == MERIDIAN_INVALID_ARGUMENT);
    require(meridian_request_authorize("desktop.get", nullptr, 1) == MERIDIAN_INVALID_ARGUMENT);
    const char *invalid[] = {nullptr};
    require(meridian_request_authorize("desktop.get", invalid, 1) == MERIDIAN_INVALID_ARGUMENT);
    require(meridian_request_authorize("desktop.get", invalid, 65) == MERIDIAN_INVALID_ARGUMENT);
}
