use diskvio_core::{Disk, list_disks};
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

/// Returns a Rust-owned UTF-8 JSON string. The caller must release it with
/// `diskvio_string_free`. Returns null if a response cannot be encoded.
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

/// Releases a non-null pointer returned by `diskvio_list_disks_json` exactly once.
/// A null pointer is accepted.
///
/// # Safety
/// `pointer` must be null or an allocation returned by `diskvio_list_disks_json`
/// that has not already been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn diskvio_string_free(pointer: *mut c_char) {
    if !pointer.is_null() {
        let _ = catch_unwind(|| {
            // SAFETY: The caller contract guarantees ownership and provenance.
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
    fn serializes_error_and_accepts_null_free() {
        let response = response_from_result(Err("diskutil failed".to_owned()));
        let json = serde_json::to_value(response).unwrap();
        assert_eq!(json["status"], "error");
        assert_eq!(json["message"], "diskutil failed");
        unsafe { diskvio_string_free(ptr::null_mut()) };
    }
}
