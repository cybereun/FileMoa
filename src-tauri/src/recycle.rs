//! Explicit, recoverable removal helpers.
//!
//! The UI must pass an explicit list of files. Windows' shell operation is used
//! so the files go to the Recycle Bin rather than being permanently deleted.
//!
//! Windows deletes permanently, without a prompt, when an item cannot be
//! recycled (network and removable drives, files larger than the bin). To keep
//! "Recycle Bin" honest this module (1) refuses drives that have no bin,
//! (2) asks the shell to warn before any permanent delete, and (3) verifies
//! afterwards which items are really gone and reports the rest.

use serde::Serialize;
#[cfg(windows)]
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecycleSkip {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecycleOutcome {
    pub recycled: Vec<String>,
    pub skipped: Vec<RecycleSkip>,
}

/// Only fixed local drives have a Recycle Bin that reliably keeps deleted
/// items. Values are the Win32 `GetDriveType` results.
#[cfg(any(windows, test))]
fn drive_type_blocker(drive_type: u32) -> Option<&'static str> {
    match drive_type {
        3 => None,
        2 => Some("이동식 드라이브는 휴지통이 없어 영구 삭제됩니다 / removable drive: items would be deleted permanently"),
        4 => Some("네트워크 드라이브는 휴지통이 없어 영구 삭제됩니다 / network drive: items would be deleted permanently"),
        5 => Some("광학 드라이브의 항목은 삭제할 수 없습니다 / optical drive"),
        6 => Some("RAM 디스크는 휴지통이 없어 영구 삭제됩니다 / RAM disk: items would be deleted permanently"),
        _ => Some("드라이브 종류를 확인할 수 없어 건너뜀 / drive type could not be determined"),
    }
}

/// Decide, after the shell call, what really happened. `exists` is injected
/// so the logic is testable without touching the file system.
#[cfg(any(windows, test))]
fn settle(
    accepted: Vec<String>,
    mut skipped: Vec<RecycleSkip>,
    aborted: bool,
    result: i32,
    exists: impl Fn(&str) -> bool,
) -> RecycleOutcome {
    let mut recycled = Vec::new();
    for path in accepted {
        if exists(&path) {
            let reason = if aborted {
                "취소되어 삭제되지 않았습니다 / not deleted: the operation was cancelled"
                    .to_string()
            } else if result != 0 {
                format!("휴지통으로 이동하지 못했습니다 (Windows 오류 {result}) / could not be moved to the Recycle Bin")
            } else {
                "휴지통으로 이동하지 못했습니다 / could not be moved to the Recycle Bin".to_string()
            };
            skipped.push(RecycleSkip { path, reason });
        } else {
            recycled.push(path);
        }
    }
    RecycleOutcome { recycled, skipped }
}

#[cfg(windows)]
mod win {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumePathNameW};

    pub fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain([0]).collect()
    }

    /// Mount point or drive root that holds `path` (handles mounted folders).
    pub fn volume_root(path: &Path) -> Option<Vec<u16>> {
        let input = wide(path);
        let mut buffer = vec![0_u16; 1024];
        let ok =
            unsafe { GetVolumePathNameW(input.as_ptr(), buffer.as_mut_ptr(), buffer.len() as u32) };
        if ok == 0 {
            return None;
        }
        let length = buffer.iter().position(|&unit| unit == 0)?;
        buffer.truncate(length + 1);
        Some(buffer)
    }

    pub fn drive_type(path: &Path) -> u32 {
        match volume_root(path) {
            Some(root) => unsafe { GetDriveTypeW(root.as_ptr()) },
            None => 0,
        }
    }
}

#[cfg(windows)]
fn recycle_with_owner(
    paths: Vec<String>,
    owner: windows_sys::Win32::Foundation::HWND,
) -> Result<RecycleOutcome, String> {
    use std::ptr::null;
    use windows_sys::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT,
        FOF_WANTNUKEWARNING, FO_DELETE, SHFILEOPSTRUCTW,
    };

    if paths.is_empty() {
        return Err("휴지통으로 보낼 파일을 하나 이상 선택하세요.".into());
    }
    let mut accepted: Vec<String> = Vec::with_capacity(paths.len());
    let mut skipped = Vec::new();
    let mut from: Vec<u16> = Vec::new();
    for value in paths {
        if accepted.contains(&value) {
            continue;
        }
        let path = Path::new(&value);
        let mut skip = |reason: &str| {
            skipped.push(RecycleSkip {
                path: value.clone(),
                reason: reason.into(),
            });
        };
        if !path.is_absolute() {
            skip("전체 경로가 아니어서 건너뜀 / not an absolute path");
            continue;
        }
        if !path.is_file() && !path.is_dir() {
            skip("항목을 찾을 수 없어 건너뜀 / not found");
            continue;
        }
        if crate::planner::is_protected_path(path) {
            skip("보호된 경로는 휴지통으로 보낼 수 없습니다 / protected path");
            continue;
        }
        if let Some(reason) = drive_type_blocker(win::drive_type(path)) {
            skip(reason);
            continue;
        }
        from.extend(value.encode_utf16());
        from.push(0);
        accepted.push(value);
    }
    if accepted.is_empty() {
        return Ok(RecycleOutcome {
            recycled: Vec::new(),
            skipped,
        });
    }
    // SHFileOperation requires a double-NUL-terminated list of source paths.
    from.push(0);
    // FOF_WANTNUKEWARNING partially overrides FOF_NOCONFIRMATION: the shell
    // still asks before an item that cannot be recycled (for example one
    // larger than the bin) would be destroyed.
    let mut operation = SHFILEOPSTRUCTW {
        hwnd: owner,
        wFunc: FO_DELETE,
        pFrom: from.as_ptr(),
        pTo: null(),
        fFlags: (FOF_ALLOWUNDO
            | FOF_NOCONFIRMATION
            | FOF_WANTNUKEWARNING
            | FOF_NOERRORUI
            | FOF_SILENT) as u16,
        fAnyOperationsAborted: 0,
        hNameMappings: std::ptr::null_mut(),
        lpszProgressTitle: null(),
    };
    let result = unsafe { SHFileOperationW(&mut operation) };
    Ok(settle(
        accepted,
        skipped,
        operation.fAnyOperationsAborted != 0,
        result,
        |path| Path::new(path).exists(),
    ))
}

#[cfg(windows)]
pub fn recycle_files(
    paths: Vec<String>,
    owner: windows_sys::Win32::Foundation::HWND,
) -> Result<RecycleOutcome, String> {
    recycle_with_owner(paths, owner)
}

#[cfg(not(windows))]
pub fn recycle_files(_: Vec<String>, _: *mut std::ffi::c_void) -> Result<RecycleOutcome, String> {
    Err("휴지통 연동은 Windows에서만 지원됩니다.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_fixed_drives_are_recycled() {
        assert_eq!(drive_type_blocker(3), None);
        for blocked in [0, 1, 2, 4, 5, 6] {
            assert!(drive_type_blocker(blocked).is_some(), "type {blocked}");
        }
    }

    #[test]
    fn items_that_survive_the_shell_call_are_reported_not_counted() {
        let outcome = settle(
            vec!["C:\\a.bin".into(), "C:\\b.bin".into()],
            Vec::new(),
            false,
            0,
            |path| path == "C:\\b.bin",
        );
        assert_eq!(outcome.recycled, vec!["C:\\a.bin".to_string()]);
        assert_eq!(outcome.skipped.len(), 1);
        assert_eq!(outcome.skipped[0].path, "C:\\b.bin");
    }

    #[test]
    fn cancelled_operation_is_labelled_as_cancelled() {
        let outcome = settle(vec!["C:\\a.bin".into()], Vec::new(), true, 0, |_| true);
        assert!(outcome.recycled.is_empty());
        assert!(outcome.skipped[0].reason.contains("cancelled"));
    }

    #[test]
    fn shell_error_code_is_included_when_nothing_was_removed() {
        let outcome = settle(vec!["C:\\a.bin".into()], Vec::new(), false, 124, |_| true);
        assert!(outcome.skipped[0].reason.contains("124"));
    }

    #[cfg(windows)]
    #[test]
    fn relative_or_missing_inputs_are_skipped_without_touching_the_shell() {
        let outcome = recycle_with_owner(
            vec![
                "relative\\file.txt".into(),
                "C:\\definitely\\missing\\x.bin".into(),
            ],
            std::ptr::null_mut(),
        )
        .expect("outcome");
        assert!(outcome.recycled.is_empty());
        assert_eq!(outcome.skipped.len(), 2);
    }

    /// Sends one throwaway file to the real Recycle Bin. Run on demand:
    /// `cargo test --lib -- --ignored recycles_a_real_file`.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn recycles_a_real_file() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp");
        std::fs::create_dir_all(&base).expect("base");
        let directory = tempfile::Builder::new().tempdir_in(base).expect("dir");
        let file = directory.path().join("filemoa-recycle-test.txt");
        std::fs::write(&file, b"safe to delete").expect("file");
        let value = file.display().to_string();
        let outcome =
            recycle_with_owner(vec![value.clone()], std::ptr::null_mut()).expect("outcome");
        assert_eq!(outcome.recycled, vec![value], "{:?}", outcome.skipped);
        assert!(!file.exists());
    }
}
