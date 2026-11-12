#ifndef MERIDIAN_POLICY_H
#define MERIDIAN_POLICY_H
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef enum {
    MERIDIAN_ALLOWED = 0,
    MERIDIAN_UNKNOWN_METHOD = 1,
    MERIDIAN_FORBIDDEN = 2,
    MERIDIAN_INVALID_ARGUMENT = 3
} meridian_authorization;
/* Static borrowed strings. NULL means unknown method; "None" means ungated.
 * This registry describes policy, not implementation availability. */
size_t meridian_request_count(void);
const char *meridian_request_method(size_t index);
const char *meridian_request_capability(const char *method);
/* Grants are Rust Capability variant names (e.g. "AppManagement"). They must
 * come from the trusted host, never from request parameters.
 * Authorization does not validate parameters or execute a service handler. */
meridian_authorization meridian_request_authorize(
    const char *method, const char *const *grants, size_t count);
#ifdef __cplusplus
}
#endif
#endif
