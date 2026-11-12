#ifndef MERIDIAN_H
#define MERIDIAN_H
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Immutable snapshot. Borrowed strings live until catalog_destroy. No exceptions
 * cross this ABI. All calls on a catalog must run on its creating thread. */
typedef struct meridian_catalog meridian_catalog;
meridian_catalog *meridian_catalog_create(void);
void meridian_catalog_destroy(meridian_catalog *catalog);
size_t meridian_catalog_count(const meridian_catalog *catalog);
const char *meridian_catalog_id(const meridian_catalog *catalog, size_t index);
const char *meridian_catalog_name(const meridian_catalog *catalog, size_t index);
/* Serialized GIcon, or NULL if absent/out of bounds. Same snapshot lifetime. */
const char *meridian_catalog_icon(const meridian_catalog *catalog, size_t index);
const char *meridian_catalog_generic_name(const meridian_catalog *catalog, size_t index);
const char *meridian_catalog_description(const meridian_catalog *catalog, size_t index);
const char *meridian_catalog_source(const meridian_catalog *catalog, size_t index);
const char *meridian_catalog_wm_class(const meridian_catalog *catalog, size_t index);
/* Ranked multi-token fuzzy search matching the existing app search provider.
 * Output/count conventions and query limits match catalog_search. */
size_t meridian_catalog_query(const meridian_catalog *catalog, const char *query,
                              size_t *indices, size_t capacity);
/* Case-folded Unicode substring search across name, ID, description, generic
 * name and desktop keywords. Returns total matches; writes at most capacity
 * snapshot indices. NULL output is allowed only with capacity zero.
 * Invalid UTF-8, queries over 4096 bytes and invalid arguments return zero.
 * Empty query matches all entries in catalog order. */
size_t meridian_catalog_search(const meridian_catalog *catalog, const char *query,
                               size_t *indices, size_t capacity);
/* Only IDs present in this snapshot may launch. GIO expands Exec without a shell.
 * Returns 1 on success, 0 on failure; optional error buffer is always terminated. */
int meridian_catalog_launch(meridian_catalog *catalog, const char *id,
                            char *error, size_t capacity);
#ifdef __cplusplus
}
#endif
#endif
