#include "meridian.h"
#include "meridian_policy.h"
#include "meridian_backend.h"
int main(void) {
    meridian_catalog_destroy(NULL);
    meridian_backend_destroy(NULL);
    meridian_client_destroy(NULL);
    meridian_task_destroy(NULL);
    if (meridian_task_wait(NULL, 0) || meridian_task_reply(NULL)) return 1;
    if (meridian_request_authorize(NULL, NULL, 0) != MERIDIAN_INVALID_ARGUMENT) return 1;
    if (meridian_catalog_search(NULL, "", NULL, 0) != 0) return 1;
    return meridian_catalog_count(NULL) != 0 || meridian_catalog_id(NULL, 0) != NULL;
}
