# Research: Windows 셸 컨텍스트 메뉴 실행

## Decision 1: Use an NSIS installer hook for registration

- **Decision**: Register and remove the shell verbs from Tauri's NSIS post-install and pre-uninstall hooks.
- **Rationale**: The installer knows the final `$INSTDIR`, so the command always points at the installed executable. Tauri documents `installerHooks` and the `NSIS_HOOK_POSTINSTALL`/`NSIS_HOOK_PREUNINSTALL` lifecycle macros.
- **Alternatives considered**: A first-run self-registration routine would require registry writes at app startup and would leave a gap before first launch; a custom installer template would be more maintenance than a hook.
- **Reference**: [Tauri Windows installer hooks](https://v2.tauri.app/distribute/windows-installer/)

## Decision 2: Use per-user registry verbs

- **Decision**: Write under `HKCU\Software\Classes\Directory\shell`, `Directory\Background\shell`, and `Drive\shell`.
- **Rationale**: Current-user registration needs no administrator elevation and follows Windows shell verb locations for folders, folder backgrounds, and drives.
- **Alternatives considered**: HKLM would require elevation and affect all users; a COM `IExplorerCommand` extension would add an in-process component that is unnecessary for a simple launch verb.
- **Reference**: [Microsoft shell extension registration](https://learn.microsoft.com/en-us/windows/win32/shell/reg-shell-exts)

## Decision 3: Pass one quoted path argument and validate in Rust

- **Decision**: The command uses an explicit `--organize` marker and a quoted `%1` (folder/drive) or `%V` (background) value. Rust accepts only the following argument and checks that it is a directory before exposing it to the UI.
- **Rationale**: Quoting preserves spaces and Unicode; an explicit marker avoids confusing arbitrary process arguments with a folder path. Validation keeps a stale or malicious registry value from triggering file changes.
- **Alternatives considered**: Parsing the last argument alone is ambiguous when update/runtime flags are present; embedding a PowerShell command would increase injection risk.

## Decision 4: Keep Windows 11 compatibility honest

- **Decision**: Use the standard legacy registry verbs. Document that Windows 11 may place them under “Show more options”.
- **Rationale**: A COM-based modern menu extension is a separate product scope. The standard verb is reliable and discoverable without a native Explorer DLL.
- **Reference**: [Microsoft shell context menu locations](https://learn.microsoft.com/en-us/windows/win32/shell/reg-shell-exts)
