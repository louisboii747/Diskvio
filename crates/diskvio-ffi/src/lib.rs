use diskvio_core::{Disk, disk_inventory, list_disks};
use serde::Serialize;
use std::{ffi::CString, os::raw::c_char, panic::catch_unwind, ptr};

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum Response {
    Ok { disks: Vec<Disk> },
    Error { message: String },
}

fn response_from_result(result: Result<Vec<Disk>, String>) -> Response {
    match result {
        Ok(disks) => Response::Ok { disks },
        Err(message) => Response::Error { message },
    }
}

fn encode_response(response: &Response) -> Option<*mut c_char> {
    let json = serde_json::to_string(response).ok()?;
    Some(CString::new(json).ok()?.into_raw())
}

#[unsafe(no_mangle)]
pub extern "C" fn diskvio_list_disks_json() -> *mut c_char {
    let result = catch_unwind(|| {
        let response = response_from_result(list_disks().map_err(|error| error.to_string()));
        encode_response(&response)
    });

    match result {
        Ok(Some(pointer)) => pointer,
        _ => catch_unwind(|| {
            encode_response(&Response::Error {
                message: "Internal disk discovery error".to_owned(),
            })
        })
        .ok()
        .flatten()
        .unwrap_or(ptr::null_mut()),
    }
}

/// Returns owned UTF-8 topology JSON. Free non-null results exactly once with
/// `diskvio_string_free`; null indicates encoding failure.
#[unsafe(no_mangle)]
pub extern "C" fn diskvio_inventory_json() -> *mut c_char {
    catch_unwind(|| {
        let response = match disk_inventory() {
            Ok(inventory) => serde_json::json!({"status": "ok", "inventory": inventory}),
            Err(error) => serde_json::json!({"status": "error", "message": error.to_string()}),
        };
        CString::new(response.to_string())
            .ok()
            .map(CString::into_raw)
    })
    .ok()
    .flatten()
    .unwrap_or(ptr::null_mut())
}

/// Returns owned operation-response JSON, freed with `diskvio_string_free`.
///
/// # Safety
/// `request` must be null or point to `length` readable bytes kept alive for this
/// call. Rust borrows the buffer without retaining or modifying it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn diskvio_operation_json(request: *const u8, length: usize) -> *mut c_char {
    let response = catch_unwind(|| {
        let result = if request.is_null() || length == 0 || length > 16_384 {
            Err("Invalid operation request buffer".to_owned())
        } else {
            // SAFETY: The caller guarantees readable bytes; the checked length
            // is nonzero, bounded to 16 KiB, and below isize::MAX.
            let bytes = unsafe { std::slice::from_raw_parts(request, length) };
            serde_json::from_slice::<diskvio_core::OperationRequest>(bytes)
                .map_err(|error| format!("Invalid operation request: {error}"))
                .and_then(|request| diskvio_core::perform_operation(&request))
        };
        match result {
            Ok(outcome) => serde_json::json!({"status": "ok", "operation": outcome}),
            Err(message) => serde_json::json!({"status": "error", "message": message}),
        }
    })
    .unwrap_or_else(
        |_| serde_json::json!({"status": "error", "message": "Internal operation error"}),
    );
    CString::new(response.to_string())
        .ok()
        .map(CString::into_raw)
        .unwrap_or(ptr::null_mut())
}

/// Frees a Rust-owned Diskvio JSON response. Null is accepted.
///
/// # Safety
/// `pointer` must be null or a pointer returned by a Diskvio JSON function that
/// has not already been freed. Release each allocation exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn diskvio_string_free(pointer: *mut c_char) {
    if !pointer.is_null() {
        let _ = catch_unwind(|| {
            // SAFETY: The caller transfers the original allocation's ownership.
            unsafe { drop(CString::from_raw(pointer)) };
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn serializes_success_and_frees_ffi_allocation() {
        let response = response_from_result(Ok(vec![Disk {
            number: 0,
            name: "Example SSD".to_owned(),
            size_bytes: 500_000_000_000,
            bus_type: "PCI-Express".to_owned(),
            partition_style: "GUID_partition_scheme".to_owned(),
        }]));
        let pointer = encode_response(&response).unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(unsafe { CStr::from_ptr(pointer) }.to_bytes()).unwrap();

        assert_eq!(json["status"], "ok");
        assert_eq!(json["disks"][0]["name"], "Example SSD");
        assert_eq!(json["disks"][0]["size_bytes"], 500_000_000_000_u64);
        unsafe { diskvio_string_free(pointer) };
    }

    #[test]
    fn rejects_invalid_operation_buffers_without_system_operations() {
        for (pointer, length) in [(ptr::null(), 0), (ptr::null(), 16_385)] {
            let response = unsafe { diskvio_operation_json(pointer, length) };
            assert!(!response.is_null());
            let json: serde_json::Value =
                serde_json::from_slice(unsafe { CStr::from_ptr(response) }.to_bytes()).unwrap();
            assert_eq!(json["status"], "error");
            unsafe { diskvio_string_free(response) };
        }
        let invalid = b"{\"action\":\"format\",\"identifier\":\"disk0\"}";
        let response = unsafe { diskvio_operation_json(invalid.as_ptr(), invalid.len()) };
        let json: serde_json::Value =
            serde_json::from_slice(unsafe { CStr::from_ptr(response) }.to_bytes()).unwrap();
        assert_eq!(json["status"], "error");
        unsafe { diskvio_string_free(response) };
    }

    #[test]
    fn serializes_error_and_accepts_null_free() {
        let response = response_from_result(Err("diskutil failed".to_owned()));
        let json = serde_json::to_value(response).unwrap();
        assert_eq!(json["status"], "error");
        assert_eq!(json["message"], "diskutil failed");
        unsafe { diskvio_string_free(ptr::null_mut()) };
    }
}
