# Implementation Plan: Windows 셸 컨텍스트 메뉴 실행

**Branch**: `003-explorer-context-menu` | **Date**: 2026-09-13 | **Spec**: [spec.md](spec.md)

## Summary

현재 사용자용 NSIS 설치 후 Windows 셸의 폴더·배경·드라이브 메뉴에 FileMoa를 등록한다. 메뉴는 고정된 설치 실행 파일에 명시적인 `--organize` 인자와 단일 경로를 전달하고, FileMoa는 시작 시 인자를 읽어 정리 화면에 폴더를 표시한다. 제거 훅은 FileMoa가 만든 HKCU 항목만 삭제한다.

## Technical Context

**Language/Version**: Rust (Tauri 2) and TypeScript/React 19

**Primary Dependencies**: Tauri NSIS installer hooks, existing Tauri dialog/updater plugins

**Storage**: Windows per-user registry (`HKCU\Software\Classes`) for shell registration; no new user data store

**Testing**: `cargo test --manifest-path src-tauri/Cargo.toml -j1 --lib`, `npm test -- --run`, `npm run build`, NSIS inspection and Windows manual smoke test

**Target Platform**: Windows 10/11 x64 and ARM64, current-user installation

**Project Type**: Tauri desktop application

**Performance Goals**: Context-menu launch adds no blocking work before the existing app window appears; argument parsing is constant-time

**Constraints**: No administrator elevation, no shell command interpolation, preserve existing preview/approval safety gates, no release upload in this implementation turn

**Scale/Scope**: One path per shell invocation; folder, background, and drive verbs; existing in-app multi-folder flow remains unchanged

## Constitution Check

- Local-first and privacy: PASS. Only a local path argument crosses the shell boundary; no remote service is involved.
- Safety and reversibility: PASS. Shell launch never moves files; existing protected-path, preview, approval, and Undo checks remain authoritative.
- Deterministic behavior: PASS. Registry verbs use explicit executable and argument forms, and invalid paths become visible errors.
- Testable releases: PASS. Hook registration, removal, argument parsing, and existing test suites have deterministic checks.

## Project Structure

```text
src-tauri/
├── nsis/hooks.nsh          # HKCU shell registration/removal macros
├── tauri.conf.json         # NSIS installer hook configuration
└── src/lib.rs              # startup_path command and invoke registration
src/App.tsx                 # consume startup path and show selected folder/error
specs/003-explorer-context-menu/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/context-menu.md
└── tasks.md
```

**Structure Decision**: Keep shell integration at the installer/command boundary and reuse the existing React folder-selection state. No shell extension DLL or new background process is required.

## Complexity Tracking

No constitution violations. A registry-based verb is intentionally chosen over a COM shell extension because this feature needs a launch action, not an in-process Explorer handler.
