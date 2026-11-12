#ifndef MERIDIAN_BACKEND_H
#define MERIDIAN_BACKEND_H
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct meridian_backend meridian_backend;
typedef struct meridian_client meridian_client;
typedef struct meridian_task meridian_task;
/* Creates a worker-backed service runtime. config_directory is an absolute
 * per-user store directory, or NULL for $XDG_CONFIG_HOME/meridian. */
meridian_backend *meridian_backend_create(const char *config_directory);
/* Cancels outstanding jobs and joins workers. Clients can be destroyed later. */
void meridian_backend_destroy(meridian_backend *backend);
/* Trusted host only: surface names follow SurfaceKind's kebab-case wire names.
 * Do not expose client creation or surface selection to untrusted request data. */
meridian_client *meridian_client_create(meridian_backend *backend, const char *surface);
void meridian_client_destroy(meridian_client *client);
/* Copies input. At most 16 outstanding jobs per client and 128 globally.
 * Invalid/over-budget requests produce normal JSON error replies. NULL indicates
 * invalid handles or allocation failure. Calls are thread-safe while handles
 * remain alive; handle destruction must not race with calls on that handle. */
meridian_task *meridian_client_submit(meridian_client *client, const char *json, size_t length);
/* Returns 1 when complete, 0 on timeout/invalid handle. Timeout capped at 60s. */
int meridian_task_wait(meridian_task *task, unsigned timeout_ms);
/* NULL until complete. Borrowed UTF-8 JSON lives until task_destroy. */
const char *meridian_task_reply(meridian_task *task);
/* Best effort; completed side effects are not rolled back. */
void meridian_task_cancel(meridian_task *task);
void meridian_task_destroy(meridian_task *task);
#ifdef __cplusplus
}
#endif
#endif
