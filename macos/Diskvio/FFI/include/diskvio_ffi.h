#ifndef DISKVIO_FFI_H
#define DISKVIO_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Returns UTF-8 JSON: {"status":"ok","disks":[...]} or
// {"status":"error","message":"..."}. Returns NULL if encoding fails.
// Release every non-NULL return value with diskvio_string_free.
char *diskvio_list_disks_json(void);

// Returns {"status":"ok","inventory":{...}} or the same error envelope.
// Ownership: release every non-NULL result exactly once with diskvio_string_free.
char *diskvio_inventory_json(void);

// Borrow a UTF-8 JSON buffer for this call only. length must be <= 16384;
// request must point to length readable bytes. NULL is reported as an error.
// Only mount, unmount and eject are accepted. Every operation is revalidated
// against fresh metadata in Rust. Release the result with diskvio_string_free.
char *diskvio_operation_json(const uint8_t *request, size_t length);

// Accepts NULL. Pass only a pointer returned by any Diskvio JSON function,
// and release that pointer exactly once.
void diskvio_string_free(char *pointer);

#ifdef __cplusplus
}
#endif

#endif
