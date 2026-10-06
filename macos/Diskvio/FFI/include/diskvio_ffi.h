#ifndef DISKVIO_FFI_H
#define DISKVIO_FFI_H

#ifdef __cplusplus
extern "C" {
#endif

// Returns UTF-8 JSON: {"status":"ok","disks":[...]} or
// {"status":"error","message":"..."}. Returns NULL if encoding fails.
// Release every non-NULL return value with diskvio_string_free.
char *diskvio_list_disks_json(void);

// Accepts NULL. Pass only a pointer returned by diskvio_list_disks_json,
// and release that pointer exactly once.
void diskvio_string_free(char *pointer);

#ifdef __cplusplus
}
#endif

#endif
