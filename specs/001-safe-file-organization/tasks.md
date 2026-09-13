# Tasks: Safe File Organization MVP

## Phase 1: Setup

- [X] T001 Create Tauri React project manifest and scripts in F:\Codex-F\smartfile\package.json
- [X] T002 Configure Rust dependencies in F:\Codex-F\smartfile\src-tauri\Cargo.toml
- [X] T003 [P] Configure NSIS bundle and updater endpoint in F:\Codex-F\smartfile\src-tauri\tauri.conf.json
- [X] T004 [P] Create bilingual message dictionaries in the `labels` map in `src/App.tsx`

## Phase 2: Foundation

- [X] T005 Create shared filesystem models in `src-tauri/src/planner.rs`
- [X] T006 [P] Implement protected-path and project-marker filters in `src-tauri/src/planner.rs`
- [X] T007 [P] Implement local settings and history store in `src/App.tsx` and `src-tauri/src/ai.rs`
- [X] T008 Create Tauri command boundary in `src-tauri/src/lib.rs`

## Phase 3: User Story 1 - Preview and organize (P1)

- [X] T009 [P] [US1] Write classifier and planner tests in `src-tauri/src/planner.rs`
- [X] T010 [US1] Implement default and custom category classification in `src-tauri/src/planner.rs`
- [X] T011 [US1] Implement dry-run plan and collision naming in `src-tauri/src/planner.rs`
- [X] T012 [US1] Implement approved move execution in `src-tauri/src/planner.rs`
- [X] T013 [US1] Implement folder selection and preview UI in `src/App.tsx`

## Phase 4: User Story 2 - Custom categories (P2)

- [X] T014 [US2] Implement custom category persistence in `src/App.tsx` local settings
- [X] T015 [US2] Implement category editor UI in `src/App.tsx`

## Phase 5: User Story 3 - Undo and history (P3)

- [X] T016 [P] [US3] Write Undo integration coverage in `src-tauri/src/planner.rs`
- [X] T017 [US3] Implement Undo preview and collision-safe restoration in `src-tauri/src/planner.rs`
- [X] T018 [US3] Implement history and Undo UI in `src/App.tsx`

## Phase 6: User Story 4 - Duplicate candidates (P4)

- [X] T019 [P] [US4] Write duplicate grouping tests in `src-tauri/src/planner.rs`
- [X] T020 [US4] Implement size-grouped SHA-256 duplicate scan in `src-tauri/src/planner.rs`
- [X] T021 [US4] Implement duplicate review UI in `src/App.tsx`

## Phase 7: Release and verification

- [X] T022 Configure tag-driven GitHub Release build in F:\Codex-F\smartfile\.github\workflows\release.yml
- [X] T023 Add README installation, updater-key, and signing instructions in F:\Codex-F\smartfile\README.md
- [ ] T024 Run a user-requested NSIS release build from F:\Codex-F\smartfile\package.json (deliberately deferred until release instruction)
