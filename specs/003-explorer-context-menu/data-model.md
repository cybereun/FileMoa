# Data Model: Windows 셸 컨텍스트 메뉴 실행

## ShellMenuRegistration

- `scope`: folder, folder background, or drive registry location.
- `verb`: stable FileMoa registry key name.
- `display_name`: bilingual menu label.
- `command_template`: installed executable plus `--organize` and one shell path token.
- `icon_path`: installed FileMoa executable used for the menu icon.
- `lifecycle`: registered after install, removed before uninstall.

## ShellLaunchRequest

- `raw_arguments`: process arguments received at startup.
- `path`: one candidate folder path after the `--organize` marker.
- `validation`: missing, not-a-directory, valid, or protected (the existing planner decides protected behavior).

## State transitions

```text
No arguments → normal empty start
--organize <directory> → selected-folder start
--organize <invalid> → selected-folder error/no writes
Installed → HKCU verbs present
Uninstalled → FileMoa HKCU verbs absent
```
