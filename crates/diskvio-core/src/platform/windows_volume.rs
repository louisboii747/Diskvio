use crate::{Device, OperationError, OperationErrorKind};

trait VolumeApi {
    type Lock;
    fn paths(&self, volume: &str) -> Result<Vec<String>, OperationError>;
    fn volume_at(&self, path: &str) -> Result<String, OperationError>;
    fn lock(&self, volume: &str) -> Result<Self::Lock, OperationError>;
    fn dismount(&self, lock: &Self::Lock) -> Result<(), OperationError>;
    fn remove_path(&self, path: &str) -> Result<(), OperationError>;
}

fn unmount_with(api: &impl VolumeApi, device: &Device) -> Result<(), OperationError> {
    let volume = device
        .stable_id
        .as_deref()
        .filter(|path| crate::operations::valid_volume_path(path))
        .ok_or_else(|| {
            OperationError::new(
                OperationErrorKind::IdentityChanged,
                "A verified volume GUID path is required",
            )
        })?;
    let path = device
        .mount_points
        .first()
        .filter(|path| device.mount_points.len() == 1 && crate::operations::valid_drive_path(path))
        .ok_or_else(|| {
            OperationError::new(
                OperationErrorKind::UnsupportedOperation,
                "Only a single drive-letter mount can be unmounted",
            )
        })?;
    let unchanged = || -> Result<(), OperationError> {
        if api.paths(volume)? != device.mount_points
            || !api.volume_at(path)?.eq_ignore_ascii_case(volume)
        {
            return Err(OperationError::new(
                OperationErrorKind::IdentityChanged,
                "Volume access paths changed before unmounting. Refresh and try again.",
            ));
        }
        Ok(())
    };
    unchanged()?;
    let lock = api.lock(volume)?;
    unchanged()?;
    api.dismount(&lock)?;
    api.remove_path(path)
}

#[cfg(target_os = "windows")]
mod win32 {
    use super::*;
    use std::{ffi::c_void, ptr};

    type Handle = *mut c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateFileW(
            name: *const u16,
            access: u32,
            share: u32,
            security: *const c_void,
            creation: u32,
            flags: u32,
            template: Handle,
        ) -> Handle;
        fn CloseHandle(handle: Handle) -> i32;
        fn DeviceIoControl(
            handle: Handle,
            code: u32,
            input: *const c_void,
            input_size: u32,
            output: *mut c_void,
            output_size: u32,
            returned: *mut u32,
            overlapped: *mut c_void,
        ) -> i32;
        fn GetVolumePathNamesForVolumeNameW(
            volume: *const u16,
            paths: *mut u16,
            length: u32,
            required: *mut u32,
        ) -> i32;
        fn GetVolumeNameForVolumeMountPointW(
            path: *const u16,
            volume: *mut u16,
            length: u32,
        ) -> i32;
        fn SetVolumeMountPointW(path: *const u16, volume: *const u16) -> i32;
        fn DeleteVolumeMountPointW(path: *const u16) -> i32;
        fn GetLogicalDrives() -> u32;
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn last_error(operation: &str, locking: bool) -> OperationError {
        let error = std::io::Error::last_os_error();
        let native = error.raw_os_error();
        let code = match native {
            Some(5 | 32 | 33) if locking => OperationErrorKind::Busy,
            Some(5 | 1314) => OperationErrorKind::PermissionDenied,
            Some(32 | 33 | 170) => OperationErrorKind::Busy,
            Some(2 | 3 | 1167) => OperationErrorKind::MissingTarget,
            _ => OperationErrorKind::Io,
        };
        let advice = if locking {
            " Close open files and check access permissions before trying again."
        } else {
            ""
        };
        OperationError {
            code,
            message: format!("{operation}: {error}.{advice}"),
            platform_code: native.map(i64::from),
        }
    }

    pub(super) struct LockedVolume(Handle);
    impl Drop for LockedVolume {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }

    fn control(handle: Handle, code: u32, name: &str, locking: bool) -> Result<(), OperationError> {
        let mut returned = 0;
        if unsafe {
            DeviceIoControl(
                handle,
                code,
                ptr::null(),
                0,
                ptr::null_mut(),
                0,
                &mut returned,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err(last_error(name, locking));
        }
        Ok(())
    }

    pub(super) struct NativeApi;
    impl VolumeApi for NativeApi {
        type Lock = LockedVolume;

        fn paths(&self, volume: &str) -> Result<Vec<String>, OperationError> {
            let volume = wide(volume);
            let mut required = 0;
            let mut buffer = vec![0; 1024];
            let mut ok = unsafe {
                GetVolumePathNamesForVolumeNameW(
                    volume.as_ptr(),
                    buffer.as_mut_ptr(),
                    buffer.len() as u32,
                    &mut required,
                )
            };
            if ok == 0
                && std::io::Error::last_os_error().raw_os_error() == Some(234)
                && required <= 65_536
            {
                buffer.resize(required as usize, 0);
                ok = unsafe {
                    GetVolumePathNamesForVolumeNameW(
                        volume.as_ptr(),
                        buffer.as_mut_ptr(),
                        buffer.len() as u32,
                        &mut required,
                    )
                };
            }
            if ok == 0 {
                return Err(last_error("Could not verify volume access paths", false));
            }
            let mut paths: Vec<_> = buffer
                .split(|unit| *unit == 0)
                .take_while(|units| !units.is_empty())
                .map(String::from_utf16_lossy)
                .collect();
            paths.sort();
            Ok(paths)
        }

        fn volume_at(&self, path: &str) -> Result<String, OperationError> {
            let path = wide(path);
            let mut buffer = vec![0; 64];
            if unsafe {
                GetVolumeNameForVolumeMountPointW(
                    path.as_ptr(),
                    buffer.as_mut_ptr(),
                    buffer.len() as u32,
                )
            } == 0
            {
                return Err(last_error("Could not verify drive-letter identity", false));
            }
            let length = buffer
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(buffer.len());
            Ok(String::from_utf16_lossy(&buffer[..length]))
        }

        fn lock(&self, volume: &str) -> Result<Self::Lock, OperationError> {
            let volume = wide(volume.trim_end_matches('\\'));
            let handle = unsafe {
                CreateFileW(
                    volume.as_ptr(),
                    0xc000_0000,
                    3,
                    ptr::null(),
                    3,
                    0,
                    ptr::null_mut(),
                )
            };
            if handle as isize == -1 {
                return Err(last_error("Could not open the volume", false));
            }
            let handle = LockedVolume(handle);
            control(
                handle.0,
                0x0009_0018,
                "Could not exclusively lock the volume",
                true,
            )?;
            Ok(handle)
        }

        fn dismount(&self, lock: &Self::Lock) -> Result<(), OperationError> {
            control(
                lock.0,
                0x0009_0020,
                "Could not dismount the locked volume",
                false,
            )
        }

        fn remove_path(&self, path: &str) -> Result<(), OperationError> {
            let path = wide(path);
            if unsafe { DeleteVolumeMountPointW(path.as_ptr()) } == 0 {
                return Err(last_error("Could not remove the drive letter", false));
            }
            Ok(())
        }
    }

    #[derive(Default)]
    pub(crate) struct NativeVolumeManager;
    impl super::super::VolumeManager for NativeVolumeManager {
        fn mount(&self, device: &Device) -> Result<String, OperationError> {
            let volume = device
                .stable_id
                .as_deref()
                .filter(|path| crate::operations::valid_volume_path(path))
                .ok_or_else(|| {
                    OperationError::new(
                        OperationErrorKind::IdentityChanged,
                        "A verified volume GUID path is required",
                    )
                })?;
            if !NativeApi.paths(volume)?.is_empty() {
                return Err(OperationError::new(
                    OperationErrorKind::IdentityChanged,
                    "Volume access paths changed before mounting",
                ));
            }
            let drives = unsafe { GetLogicalDrives() };
            if drives == 0 {
                return Err(last_error("Could not find available drive letters", false));
            }
            let letter = (b'D'..=b'Z')
                .find(|letter| drives & (1 << (letter - b'A')) == 0)
                .ok_or_else(|| {
                    OperationError::new(
                        OperationErrorKind::UnsupportedOperation,
                        "No drive letter is available",
                    )
                })?;
            let path = format!("{}:\\", char::from(letter));
            if unsafe { SetVolumeMountPointW(wide(&path).as_ptr(), wide(volume).as_ptr()) } == 0 {
                return Err(last_error("Could not mount the volume", false));
            }
            Ok(path)
        }

        fn unmount(&self, device: &Device) -> Result<(), OperationError> {
            unmount_with(&NativeApi, device)
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) use win32::NativeVolumeManager;

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const VOLUME: &str = "\\\\?\\Volume{11111111-2222-3333-4444-555555555555}\\";

    struct FakeApi {
        calls: RefCell<Vec<&'static str>>,
        busy: bool,
        changed: bool,
    }
    impl VolumeApi for FakeApi {
        type Lock = ();
        fn paths(&self, _: &str) -> Result<Vec<String>, OperationError> {
            Ok(vec!["E:\\".into()])
        }
        fn volume_at(&self, _: &str) -> Result<String, OperationError> {
            Ok(if self.changed { "different" } else { VOLUME }.into())
        }
        fn lock(&self, _: &str) -> Result<(), OperationError> {
            self.calls.borrow_mut().push("lock");
            if self.busy {
                Err(OperationError::new(
                    OperationErrorKind::Busy,
                    "Files are open",
                ))
            } else {
                Ok(())
            }
        }
        fn dismount(&self, _: &()) -> Result<(), OperationError> {
            self.calls.borrow_mut().push("dismount");
            Ok(())
        }
        fn remove_path(&self, _: &str) -> Result<(), OperationError> {
            self.calls.borrow_mut().push("remove");
            Ok(())
        }
    }

    fn device() -> Device {
        Device {
            stable_id: Some(VOLUME.into()),
            mount_points: vec!["E:\\".into()],
            ..Default::default()
        }
    }

    #[test]
    fn busy_volumes_are_never_dismounted() {
        let api = FakeApi {
            calls: RefCell::default(),
            busy: true,
            changed: false,
        };
        assert_eq!(
            unmount_with(&api, &device()).unwrap_err().code,
            OperationErrorKind::Busy
        );
        assert_eq!(*api.calls.borrow(), ["lock"]);
    }

    #[test]
    fn changed_drive_letter_is_rejected_before_locking() {
        let api = FakeApi {
            calls: RefCell::default(),
            busy: false,
            changed: true,
        };
        assert_eq!(
            unmount_with(&api, &device()).unwrap_err().code,
            OperationErrorKind::IdentityChanged
        );
        assert!(api.calls.borrow().is_empty());
    }

    #[test]
    fn dismount_requires_a_successful_lock() {
        let api = FakeApi {
            calls: RefCell::default(),
            busy: false,
            changed: false,
        };
        unmount_with(&api, &device()).unwrap();
        assert_eq!(*api.calls.borrow(), ["lock", "dismount", "remove"]);
    }
}
