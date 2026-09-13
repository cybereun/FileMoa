//! Explicit, recoverable removal helpers.
//!
//! The UI must pass an explicit list of files. Windows' shell operation is used
//! so the files go to the Recycle Bin rather than being permanently deleted.

use std::path::Path;

#[cfg(windows)]
pub fn recycle_files(paths: Vec<String>) -> Result<Vec<String>, String> {
    use std::ptr::null;
    use windows_sys::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE,
        SHFILEOPSTRUCTW,
    };

    if paths.is_empty() {
        return Err("휴지통으로 보낼 파일을 하나 이상 선택하세요.".into());
    }
    let mut accepted = Vec::with_capacity(paths.len());
    let mut from = Vec::new();
    for value in paths {
        let path = Path::new(&value);
        if !path.is_file() && !path.is_dir() {
            continue;
        }
        if crate::planner::is_protected_path(path) {
            return Err(format!(
                "보호된 경로는 휴지통으로 보낼 수 없습니다: {}",
                path.display()
            ));
        }
        let wide = value.encode_utf16().collect::<Vec<_>>();
        from.extend(wide);
        from.push(0);
        accepted.push(value);
    }
    if accepted.is_empty() {
        return Err("선택한 파일을 찾을 수 없습니다.".into());
    }
    // SHFileOperation requires a double-NUL-terminated list of source paths.
    from.push(0);
    let mut operation = SHFILEOPSTRUCTW {
        hwnd: std::ptr::null_mut(),
        wFunc: FO_DELETE,
        pFrom: from.as_ptr(),
        pTo: null(),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT) as u16,
        fAnyOperationsAborted: 0,
        hNameMappings: std::ptr::null_mut(),
        lpszProgressTitle: null(),
    };
    let result = unsafe { SHFileOperationW(&mut operation) };
    if result != 0 {
        return Err(format!(
            "휴지통 이동에 실패했습니다 (Windows 오류 {result})."
        ));
    }
    Ok(accepted)
}

#[cfg(not(windows))]
pub fn recycle_files(_: Vec<String>) -> Result<Vec<String>, String> {
    Err("휴지통 연동은 Windows에서만 지원됩니다.".into())
}
