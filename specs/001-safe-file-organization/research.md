# Research: Safe File Organization MVP

## Tauri 2 with React and Rust

**Decision**: Use Tauri 2, React, and Rust.

**Rationale**: Tauri uses Windows WebView instead of bundling Chromium, producing a
smaller installer than Electron while Rust provides safe, fast filesystem operations.

**Alternatives considered**: Electron was rejected for installer size; a native-only
UI was rejected because a bilingual modern interface is needed.

## NSIS per-user setup executable

**Decision**: Bundle an NSIS setup EXE.

**Rationale**: Tauri supports NSIS setup executables; user-scoped installs normally
avoid elevation and suit update flows.

**Alternatives considered**: MSI is enterprise-oriented; MSIX adds sideloading
constraints.

## Static GitHub Releases updater

**Decision**: Publish Tauri signed update artifacts and `latest.json` via GitHub
Actions.

**Rationale**: Tauri supports static update JSON and GitHub release assets. The
Windows passive installer mode displays progress with minimal intervention.

## Updater signing versus Authenticode

**Decision**: Require updater artifact signatures; defer Authenticode.

**Rationale**: The owner has no code-signing certificate. Updater signatures protect
the update channel, but unsigned initial installers can still cause SmartScreen.

## Duplicate hashing

**Decision**: Group by size before full SHA-256 hashing.

**Rationale**: It confirms equality while avoiding hashes for files that cannot match.
