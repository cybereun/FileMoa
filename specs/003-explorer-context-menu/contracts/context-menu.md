# Shell Context Menu Contract

| Scope | Registry verb | Path token | Expected behavior |
|---|---|---|---|
| Folder | `HKCU\Software\Classes\Directory\shell\FileMoa` | `%1` | Open FileMoa with the clicked directory selected |
| Folder background | `HKCU\Software\Classes\Directory\Background\shell\FileMoa` | `%V` | Open FileMoa with the current directory selected |
| Desktop background | `HKCU\Software\Classes\DesktopBackground\shell\FileMoa` | `%V` | Open FileMoa with the desktop directory selected |
| Drive | `HKCU\Software\Classes\Drive\shell\FileMoa` | `%1` | Open FileMoa with the drive root selected |

The command must include a quoted installed executable path and the literal
`--organize` marker. The application must not execute any operation until the
existing preview and explicit approval flow is used.
