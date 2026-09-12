# Tasks: Safe File Organization MVP

## Phase 1: Setup

- [ ] T001 Create Tauri React project manifest and scripts in F:\Codex-F\smartfile\package.json
- [ ] T002 Configure Rust dependencies in F:\Codex-F\smartfile\src-tauri\Cargo.toml
- [ ] T003 [P] Configure NSIS bundle and updater endpoint in F:\Codex-F\smartfile\src-tauri\tauri.conf.json
- [ ] T004 [P] Create bilingual message dictionaries in F:\Codex-F\smartfile\src\i18n\messages.ts

## Phase 2: Foundation

- [ ] T005 Create shared filesystem models in F:\Codex-F\smartfile\src-tauri\src\models.rs
- [ ] T006 [P] Implement protected-path and project-marker filters in F:\Codex-F\smartfile\src-tauri\src\protection.rs
- [ ] T007 [P] Implement local settings and history store in F:\Codex-F\smartfile\src-tauri\src\storage.rs
- [ ] T008 Create Tauri command boundary in F:\Codex-F\smartfile\src-tauri\src\commands.rs

## Phase 3: User Story 1 - Preview and organize (P1)

- [ ] T009 [P] [US1] Write classifier and planner tests in F:\Codex-F\smartfile\src-tauri\src\planner.rs
- [ ] T010 [US1] Implement default and custom category classification in F:\Codex-F\smartfile\src-tauri\src\classifier.rs
- [ ] T011 [US1] Implement dry-run plan and collision naming in F:\Codex-F\smartfile\src-tauri\src\planner.rs
- [ ] T012 [US1] Implement approved move execution in F:\Codex-F\smartfile\src-tauri\src\executor.rs
- [ ] T013 [US1] Implement folder selection and preview UI in F:\Codex-F\smartfile\src\App.tsx

## Phase 4: User Story 2 - Custom categories (P2)

- [ ] T014 [US2] Implement custom category persistence in F:\Codex-F\smartfile\src-tauri\src\storage.rs
- [ ] T015 [US2] Implement category editor UI in F:\Codex-F\smartfile\src\components\CategoryEditor.tsx

## Phase 5: User Story 3 - Undo and history (P3)

- [ ] T016 [P] [US3] Write Undo integration tests in F:\Codex-F\smartfile\src-tauri\tests\undo.rs
- [ ] T017 [US3] Implement Undo preview and collision-safe restoration in F:\Codex-F\smartfile\src-tauri\src\executor.rs
- [ ] T018 [US3] Implement history and Undo UI in F:\Codex-F\smartfile\src\components\HistoryPanel.tsx

## Phase 6: User Story 4 - Duplicate candidates (P4)

- [ ] T019 [P] [US4] Write duplicate grouping tests in F:\Codex-F\smartfile\src-tauri\src\duplicates.rs
- [ ] T020 [US4] Implement size-grouped SHA-256 duplicate scan in F:\Codex-F\smartfile\src-tauri\src\duplicates.rs
- [ ] T021 [US4] Implement duplicate review UI in F:\Codex-F\smartfile\src\components\DuplicatePanel.tsx

## Phase 7: Release and verification

- [ ] T022 Configure tag-driven GitHub Release build in F:\Codex-F\smartfile\.github\workflows\release.yml
- [ ] T023 Add README installation, updater-key, and signing instructions in F:\Codex-F\smartfile\README.md
- [ ] T024 Run tests and NSIS release build from F:\Codex-F\smartfile\package.json
