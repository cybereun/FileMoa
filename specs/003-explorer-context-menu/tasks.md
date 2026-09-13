# Tasks: Windows 셸 컨텍스트 메뉴 실행

**Input**: Design documents from `specs/003-explorer-context-menu/`

## Phase 1: Specification and installer boundary

- [X] T001 Write user scenarios, requirements, edge cases, and success criteria in `spec.md`.
- [X] T002 Record registry/NSIS decisions and shell contract in `research.md`, `data-model.md`, and `contracts/context-menu.md`.

## Phase 2: Shell registration

- [X] T003 [US1] Add current-user folder, background, desktop-background, and drive verbs in `src-tauri/nsis/hooks.nsh`.
- [X] T004 [US3] Configure `bundle.windows.nsis.installerHooks` in `src-tauri/tauri.conf.json`.
- [X] T005 [US3] Add uninstall cleanup that removes only FileMoa-owned registry keys.

## Phase 3: Shell launch handoff

- [X] T006 [US1] Add a safe startup-path command in `src-tauri/src/lib.rs`.
- [X] T007 [US1] Consume the startup path in `src/App.tsx` and show it as the selected folder.
- [X] T008 [US2] Preserve existing protected-path, warning, preview, approval, and Undo behavior for shell-launched folders.

## Phase 4: Verification and documentation

- [X] T009 [P] Add Rust tests for quoted/unicode startup argument parsing and invalid paths.
- [X] T010 [P] Add frontend tests for startup-path state and no-auto-move behavior.
- [X] T011 Document installer smoke tests in `quickstart.md` and update the root README.
- [X] T012 Run `npm test`, `npm run build`, `cargo fmt`, and `cargo test`; defer NSIS build until explicitly requested.

## Dependencies

- T003–T005 depend on T001–T002.
- T006–T008 depend on T003–T005 because the launch argument must match the installed verb.
- T009–T012 depend on implementation completion; NSIS installation is a manual Windows check.
