use crate::{Device, OperationError, OperationErrorKind};

trait VolumeApi {
    type Lock;
    fn paths(&self, volume: &str) -> Result<Vec<String>, OperationError>;
    fn volume_at(&self, path: &str) -> Result<String, OperationError>;
    fn lock(&self, volume: &str) -> Result<Self::Lock, OperationError>;
    fn dismount(&self, lock: &Self::Lock) -> Result<(), OperationError>;
    fn remove_path(&self, path: &str) -> Result<(), OperationError>;
    fn add_path(&self, path: &str, volume: &str) -> Result<(), OperationError>;
    fn available(&self, letter: u8) -> Result<bool, OperationError>;
    fn set_label(&self, volume: &str, label: &str) -> Result<(), OperationError>;
}

fn verified_volume<'a>(
    api: &impl VolumeApi,
    device: &'a Device,
) -> Result<&'a str, OperationError> {
    let volume = device
        .stable_id
        .as_deref()
        .filter(|p| crate::operations::valid_volume_path(p))
        .ok_or_else(|| {
            OperationError::new(
                OperationErrorKind::IdentityChanged,
                "A verified volume GUID path is required",
            )
        })?;
    if api.paths(volume)? != device.mount_points {
        return Err(OperationError::new(
            OperationErrorKind::IdentityChanged,
            "Volume access paths changed. Refresh and try again.",
        ));
    }
    for path in &device.mount_points {
        if !api.volume_at(path)?.eq_ignore_ascii_case(volume) {
            return Err(OperationError::new(
                OperationErrorKind::IdentityChanged,
                "The drive letter now belongs to a different volume",
            ));
        }
    }
    Ok(volume)
}

fn rename_with(api: &impl VolumeApi, device: &Device, label: &str) -> Result<(), OperationError> {
    let volume = verified_volume(api, device)?;
    api.set_label(volume, label)
}

fn set_drive_letter_with(
    api: &impl VolumeApi,
    device: &Device,
    letter: &str,
) -> Result<(), OperationError> {
    if letter.len() != 1 || !(b'D'..=b'Z').contains(&letter.as_bytes()[0]) {
        return Err(OperationError::new(
            OperationErrorKind::InvalidRequest,
            "Choose a drive letter from D through Z",
        ));
    }
    let volume = verified_volume(api, device)?;
    if device.mount_points.len() > 1
        || device
            .mount_points
            .iter()
            .any(|p| !crate::operations::valid_drive_path(p))
    {
        return Err(OperationError::new(
            OperationErrorKind::UnsupportedOperation,
            "Only single drive-letter mounts are supported",
        ));
    }
    if !api.available(letter.as_bytes()[0])? {
        return Err(OperationError::new(
            OperationErrorKind::InvalidState,
            "The requested drive letter is already in use",
        ));
    }
    let lock = api.lock(volume)?;
    verified_volume(api, device)?;
    if !api.available(letter.as_bytes()[0])? {
        return Err(OperationError::new(
            OperationErrorKind::InvalidState,
            "The requested drive letter became unavailable",
        ));
    }
    api.dismount(&lock)?;
    let old = device.mount_points.first();
    if let Some(old) = old {
        api.remove_path(old)?;
    }
    let new = format!("{letter}:\\");
    if let Err(mut error) = api.add_path(&new, volume) {
        if let Some(old) = old {
            match api.add_path(old, volume) {
                Ok(()) => error.message.push_str(" The original drive letter was restored."),
                Err(restore) => error.message.push_str(&format!(" The original drive letter could not be restored: {}. Refresh to inspect the volume; its GUID identity remains unchanged.", restore.message)),
            }
        }
        return Err(error);
    }
    Ok(())
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
        fn SetVolumeLabelW(volume: *const u16, label: *const u16) -> i32;
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

        fn add_path(&self, path: &str, volume: &str) -> Result<(), OperationError> {
            if unsafe { SetVolumeMountPointW(wide(path).as_ptr(), wide(volume).as_ptr()) } == 0 {
                return Err(last_error("Could not assign the drive letter", false));
            }
            Ok(())
        }
        fn available(&self, letter: u8) -> Result<bool, OperationError> {
            let drives = unsafe { GetLogicalDrives() };
            if drives == 0 {
                return Err(last_error("Could not query drive letters", false));
            }
            Ok(drives & (1 << (letter - b'A')) == 0)
        }
        fn set_label(&self, volume: &str, label: &str) -> Result<(), OperationError> {
            if unsafe { SetVolumeLabelW(wide(volume).as_ptr(), wide(label).as_ptr()) } == 0 {
                return Err(last_error("Could not rename the volume", false));
            }
            Ok(())
        }

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
        fn rename(&self, device: &Device, label: &str) -> Result<(), OperationError> {
            rename_with(&NativeApi, device, label)
        }
        fn set_drive_letter(&self, device: &Device, letter: &str) -> Result<(), OperationError> {
            for path in &device.mount_points {
                for process_path in [std::env::current_exe(), std::env::current_dir()]
                    .into_iter()
                    .flatten()
                {
                    if process_path
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .starts_with(&path.to_ascii_lowercase())
                    {
                        return Err(OperationError::new(
                            OperationErrorKind::ProtectedDevice,
                            "Diskvio is running from or using this volume as its working directory. Run it from another disk before changing the drive letter.",
                        ));
                    }
                }
            }
            set_drive_letter_with(&NativeApi, device, letter)
        }
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
        fn add_path(&self, _: &str, _: &str) -> Result<(), OperationError> {
            self.calls.borrow_mut().push("add");
            Ok(())
        }
        fn available(&self, _: u8) -> Result<bool, OperationError> {
            Ok(true)
        }
        fn set_label(&self, _: &str, _: &str) -> Result<(), OperationError> {
            self.calls.borrow_mut().push("label");
            Ok(())
        }
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
    struct LetterApi {
        paths: RefCell<Vec<String>>,
        calls: RefCell<Vec<String>>,
        busy: bool,
        occupied: bool,
        fail_new: bool,
        fail_restore: bool,
        changed_on_lock: bool,
    }
    impl Default for LetterApi {
        fn default() -> Self {
            Self {
                paths: RefCell::new(vec!["E:\\".into()]),
                calls: RefCell::default(),
                busy: false,
                occupied: false,
                fail_new: false,
                fail_restore: false,
                changed_on_lock: false,
            }
        }
    }
    impl VolumeApi for LetterApi {
        type Lock = ();
        fn paths(&self, _: &str) -> Result<Vec<String>, OperationError> {
            Ok(self.paths.borrow().clone())
        }
        fn volume_at(&self, _: &str) -> Result<String, OperationError> {
            Ok(VOLUME.into())
        }
        fn available(&self, _: u8) -> Result<bool, OperationError> {
            Ok(!self.occupied)
        }
        fn lock(&self, _: &str) -> Result<(), OperationError> {
            self.calls.borrow_mut().push("lock".into());
            if self.changed_on_lock {
                self.paths.borrow_mut().clear();
            }
            if self.busy {
                Err(OperationError::new(OperationErrorKind::Busy, "Open files"))
            } else {
                Ok(())
            }
        }
        fn dismount(&self, _: &()) -> Result<(), OperationError> {
            self.calls.borrow_mut().push("dismount".into());
            Ok(())
        }
        fn remove_path(&self, path: &str) -> Result<(), OperationError> {
            self.calls.borrow_mut().push(format!("remove {path}"));
            self.paths.borrow_mut().clear();
            Ok(())
        }
        fn add_path(&self, path: &str, _: &str) -> Result<(), OperationError> {
            self.calls.borrow_mut().push(format!("add {path}"));
            if (path.starts_with('F') && self.fail_new)
                || (path.starts_with('E') && self.fail_restore)
            {
                return Err(OperationError::new(
                    OperationErrorKind::PermissionDenied,
                    "Assignment failed",
                ));
            }
            self.paths.borrow_mut().push(path.into());
            Ok(())
        }
        fn set_label(&self, _: &str, label: &str) -> Result<(), OperationError> {
            self.calls.borrow_mut().push(format!("label {label}"));
            Ok(())
        }
    }
    #[test]
    fn drive_letter_change_locks_and_dismounts_before_removing_old_path() {
        let api = LetterApi::default();
        set_drive_letter_with(&api, &device(), "F").unwrap();
        assert_eq!(
            *api.calls.borrow(),
            ["lock", "dismount", "remove E:\\", "add F:\\"]
        );
        assert_eq!(*api.paths.borrow(), ["F:\\"]);
    }
    #[test]
    fn failed_assignment_restores_original_letter_and_reports_rollback_failure() {
        for fail_restore in [false, true] {
            let api = LetterApi {
                fail_new: true,
                fail_restore,
                ..Default::default()
            };
            let error = set_drive_letter_with(&api, &device(), "F").unwrap_err();
            assert_eq!(error.code, OperationErrorKind::PermissionDenied);
            assert_eq!(
                *api.calls.borrow(),
                ["lock", "dismount", "remove E:\\", "add F:\\", "add E:\\"]
            );
            assert_eq!(api.paths.borrow().is_empty(), fail_restore);
            assert!(error.message.contains(if fail_restore {
                "could not be restored"
            } else {
                "was restored"
            }));
        }
    }
    #[test]
    fn busy_occupied_or_changed_volumes_cannot_have_letters_removed() {
        for api in [
            LetterApi {
                busy: true,
                ..Default::default()
            },
            LetterApi {
                occupied: true,
                ..Default::default()
            },
            LetterApi {
                changed_on_lock: true,
                ..Default::default()
            },
        ] {
            assert!(set_drive_letter_with(&api, &device(), "F").is_err());
            assert!(
                !api.calls
                    .borrow()
                    .iter()
                    .any(|call| call.starts_with("remove") || call.starts_with("add"))
            );
        }
    }
    #[test]
    fn unmounted_volume_can_receive_explicit_letter_and_label_uses_guid_identity() {
        let api = LetterApi::default();
        api.paths.borrow_mut().clear();
        let unmounted = Device {
            mount_points: vec![],
            ..device()
        };
        set_drive_letter_with(&api, &unmounted, "F").unwrap();
        assert_eq!(*api.calls.borrow(), ["lock", "dismount", "add F:\\"]);
        let api = LetterApi::default();
        rename_with(&api, &device(), "Données").unwrap();
        assert_eq!(*api.calls.borrow(), ["label Données"]);
        api.paths.borrow_mut().clear();
        assert_eq!(
            rename_with(&api, &device(), "Bad target").unwrap_err().code,
            OperationErrorKind::IdentityChanged
        );
        assert_eq!(api.calls.borrow().len(), 1);
    }
}
