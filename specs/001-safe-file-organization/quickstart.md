# Quickstart: Validate FileMoa MVP

## Prerequisites

- Windows 10 or 11 x64
- Node.js 22 LTS, Rust stable, and Visual Studio C++ Build Tools

## Local checks

1. Run `npm install`.
2. Run `cargo test --manifest-path src-tauri/Cargo.toml`.
3. Run `npm test`.
4. Start with `npm run tauri dev`.

## End-to-end scenario

1. Create a disposable folder with `sample.pdf`, `photo.png`, `archive.zip`,
   `setup.exe`, `unknown.xyz`, and a nested directory containing `package.json`.
2. Choose the folder in FileMoa and select a scan mode.
3. Generate a preview. Confirm categories and that the project directory is excluded.
4. Approve the plan and confirm category folders appear inside the selected folder.
5. Open History, preview Undo, execute it, and verify files return to their paths.
6. Add a custom category and confirm it overrides the built-in rule.
7. Add two identical files and confirm duplicate scan groups them without deletion.

## Release check

Run `npm run tauri build -- --bundles nsis`. Confirm setup and updater artifacts are
created under `src-tauri/target/release/bundle/nsis/`.
