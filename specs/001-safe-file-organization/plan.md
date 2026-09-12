# Implementation Plan: Safe File Organization MVP

**Branch**: `001-safe-file-organization` | **Date**: 2026-09-12 | **Spec**: [spec.md](spec.md)

## Summary

Build FileMoa as a local Windows desktop app. It scans one user-selected folder,
creates an explainable type-based organization plan, requires approval before moving
files, keeps local run history, provides Undo, and finds content-identical duplicate
candidates. GitHub Actions builds NSIS setup executables and publishes signed updater
metadata to GitHub Releases.

## Technical Context

**Language/Version**: Rust stable (edition 2021), TypeScript 5, Node.js 22 LTS

**Primary Dependencies**: Tauri 2, React 19, Vite, `serde`, `sha2`, `walkdir`,
`tauri-plugin-dialog`, `tauri-plugin-store`, `tauri-plugin-updater`

**Storage**: Local JSON files in the app data directory

**Testing**: Rust unit/integration tests; Vitest; `cargo tauri build --bundles nsis`

**Target Platform**: Windows 10/11 x64

**Project Type**: Desktop app

**Performance Goals**: Preview 1,000 local files in under 15 seconds; hash duplicate
candidates only inside equal-size groups

**Constraints**: Offline local-first processing; no destructive default action; no
elevation; selected-root-only writes; Korean/English UI; compact NSIS installer

**Scale/Scope**: One selected root per run; v1 excludes scheduling, AI, cloud
management, date sorting, and deletion tools

## Constitution Check

| Gate | Status | Evidence |
|---|---|---|
| Preview and approval precede writes | PASS | Plan and execute are separate commands. |
| System/project boundaries protected | PASS | Filters precede classification. |
| Actions explainable and collision safe | PASS | Action records contain rule, paths, and collision result. |
| Data stays local | PASS | Local JSON only; no file data in update request. |
| Behavior tested and releases verifiable | PASS | Rust tests and signed updater workflow planned. |

## Project Structure

```text
src/                         # React UI
├── components/
├── hooks/
├── i18n/
├── lib/
└── types.ts
src-tauri/src/
├── commands.rs
├── classifier.rs
├── duplicates.rs
├── executor.rs
├── planner.rs
├── protection.rs
├── storage.rs
└── lib.rs
src-tauri/tests/
tests/
.github/workflows/release.yml
assets/icon/
specs/001-safe-file-organization/
```

**Structure Decision**: Tauri keeps filesystem operations in Rust and presents a
lightweight, testable bilingual React interface.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Rust + TypeScript layers | Native filesystem safety and bilingual modern UI both matter. | A web-only app cannot safely access local files. |

## Post-Design Constitution Check

All gates remain PASS. Tauri updater signatures are separate from optional Windows
Authenticode signing. The update endpoint is static `latest.json` in GitHub Releases.
