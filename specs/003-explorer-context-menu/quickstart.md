# Quickstart: Shell Context Menu Validation

## Prerequisites

- Windows 10 or Windows 11.
- A local checkout at `F:\Codex-F\smartfile`.
- NSIS build prerequisites installed for Tauri.

## Automated checks

```powershell
Set-Location F:\Codex-F\smartfile
npm test -- --run
npm run build
cargo fmt --manifest-path src-tauri\Cargo.toml -- --check
$env:CARGO_INCREMENTAL='0'; $env:RUSTFLAGS='-C debuginfo=0'
cargo test --manifest-path src-tauri\Cargo.toml -j1 --lib
```

## Installer smoke test

Run `npm run build:nsis` only when a release build is explicitly requested. Install the generated current-user setup, then:

1. Right-click a folder containing spaces and Korean characters; choose `FileMoa로 정리 / Organize with FileMoa`.
2. Right-click the empty area of that folder and repeat; the same folder should be selected.
3. Right-click the desktop background and repeat; the desktop directory should be selected.
4. Right-click a drive root and repeat; the drive should be selected.
5. Press FileMoa's preview button. No files should move before explicit approval.
6. Uninstall FileMoa and confirm all four FileMoa menu entries are gone while unrelated entries remain.

On Windows 11, use the standard context menu's `Show more options` entry if the legacy verb is not shown in the compact menu.
