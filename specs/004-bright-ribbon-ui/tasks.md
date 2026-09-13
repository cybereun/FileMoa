# Tasks: Bright Ribbon Workspace

**Input**: Design documents from `specs/004-bright-ribbon-ui/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/ribbon-ui.md](contracts/ribbon-ui.md)

**Tests**: Existing Vitest and production build checks are included in the cross-cutting phase. No new test dependency is needed for this presentation-only feature.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the existing project and feature artifacts before changing UI code.

- [X] T001 Verify the existing React/Tauri project entry points and feature documents in `src/App.tsx`, `src/styles.css`, and `specs/004-bright-ribbon-ui/`
- [X] T002 Record the bright palette, ribbon grouping, and responsive invariants in `specs/004-bright-ribbon-ui/research.md` and `specs/004-bright-ribbon-ui/contracts/ribbon-ui.md`

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Establish a safe presentation-only boundary before story work.

- [X] T003 Inspect existing ribbon handlers and disabled guards in `src/App.tsx` and document that folder selection, preview, move, Undo, and update behavior remain unchanged
- [X] T004 [P] Map current CSS selectors used by organize, manage, safety, settings, AI, and update screens in `src/styles.css` for palette coverage

**Checkpoint**: Existing behavior boundaries and visual surface inventory are confirmed.

## Phase 3: User Story 1 - 빠른 폴더를 한눈에 선택하기 (Priority: P1) 🎯 MVP

**Goal**: Make quick folders a readable, non-shrinking horizontal strip while retaining one-click selection.

**Independent Test**: With two or more known folders, open the organize tab at a wide and narrow window size, confirm labels stay horizontal, click one button, and confirm the path updates without running a file operation.

- [X] T005 [US1] Group action buttons and organize options with semantic wrappers in `src/App.tsx` without changing existing callbacks or disabled expressions
- [X] T006 [US1] Render the quick-folder strip as a separate labeled row and keep each button's existing id, path tooltip, icon, and `chooseKnownFolder` handler in `src/App.tsx`
- [X] T007 [US1] Add non-shrinking horizontal quick-folder layout, readable labels, overflow scrolling, and path ellipsis rules in `src/styles.css`
- [X] T008 [US1] Add wide/narrow ribbon media rules in `src/styles.css` so action and option groups wrap or stack without squeezing quick-folder buttons

**Checkpoint**: User Story 1 is independently usable and the screenshot's vertical quick-folder defect is removed.

## Phase 4: User Story 2 - 밝고 편안한 작업 화면 사용하기 (Priority: P1)

**Goal**: Apply a coherent bright ivory, warm-gray, and terracotta theme to every existing screen and modal.

**Independent Test**: Visit organize, manage, safety, AI settings, and update modal in the default language and verify readable surfaces, controls, statuses, and focus states.

- [X] T009 [US2] Define semantic bright theme tokens and base typography/background colors in `src/styles.css`
- [X] T010 [US2] Restyle header, tabs, ribbon, workspace, tables, forms, cards, and empty states with the bright palette in `src/styles.css`
- [X] T011 [US2] Restyle provider/AI settings, privacy callout, registration links, and status messages for light-surface contrast in `src/styles.css`
- [X] T012 [US2] Restyle update/undo dialogs, release notes, progress states, action buttons, and semantic warning/error/success badges in `src/styles.css`
- [X] T013 [US2] Add visible hover, keyboard focus, disabled, and high-contrast safeguards for all interactive controls in `src/styles.css`

**Checkpoint**: User Story 2 is independently readable across all existing views without changing behavior.

## Phase 5: User Story 3 - 창 크기와 언어에 관계없이 리본 사용하기 (Priority: P2)

**Goal**: Preserve access to ribbon controls and selected path across Korean/English labels and compact desktop widths.

**Independent Test**: Switch between Korean and English at approximately 940px and 520px widths; confirm controls do not overlap, key actions remain reachable, and quick folders scroll.

- [X] T014 [US3] Verify all existing localized ribbon labels and settings menu actions fit the new wrapper structure in `src/App.tsx`
- [X] T015 [US3] Tune responsive breakpoints, minimum click targets, wrapping, and horizontal overflow for 940px/520px windows in `src/styles.css`
- [X] T016 [US3] Verify long Unicode folder labels/paths retain readable labels and full-path title access in `src/App.tsx` and `src/styles.css`

**Checkpoint**: User Stories 1–3 are independently functional in both supported languages and target widths.

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Validate the complete visual change and keep documentation aligned.

- [X] T017 [P] Update `specs/004-bright-ribbon-ui/quickstart.md` with any observed validation notes and final commands
- [X] T018 Run `npm test -- --run` and record the result for `specs/004-bright-ribbon-ui/quickstart.md`
- [X] T019 Run `npm run build` and record the result for `specs/004-bright-ribbon-ui/quickstart.md`
- [X] T020 Manually perform the no-file-mutation wide/narrow Korean/English checklist from `specs/004-bright-ribbon-ui/quickstart.md`
- [X] T021 Review the final diff for accidental changes to file-operation logic and ensure `specs/004-bright-ribbon-ui/spec.md` requirements are covered

## Dependencies & Execution Order

### Phase Dependencies

- Phase 1 has no dependencies.
- Phase 2 depends on Phase 1 and blocks user-story implementation.
- User Stories 1 and 2 both depend on Phase 2; their CSS work touches the same stylesheet and should be sequenced to avoid conflicts.
- User Story 3 depends on the ribbon structure from User Story 1 and the palette from User Story 2.
- Phase 6 depends on all selected stories.

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Phase 2; MVP and prerequisite for responsive verification.
- **User Story 2 (P1)**: Can start after Phase 2; independent behaviorally, but shares `src/styles.css` with US1.
- **User Story 3 (P2)**: Depends on US1 structure and US2 tokens; validates their combined result.

### Parallel Opportunities

- T004 can run in parallel with T003.
- T009–T012 cover separate visual regions conceptually, but should be applied in one stylesheet pass to keep token usage consistent.
- T017 and T018 can run in parallel after implementation; T019 follows code completion and T020 can run while the build executes.

### Requirement Traceability

- FR-001/FR-002 → T005–T008; FR-003 → T006; FR-004 → T009–T010; FR-005 → T010–T012; FR-006/FR-007 → T014–T016; FR-008 → T003 and T021; FR-009 → T013.
- SC-001/SC-002 → T005–T008 and T020; SC-003 → T009–T013 and T018–T019; SC-004 → T014–T016 and T020; SC-005 → T003 and T020–T021.

## Implementation Strategy

### MVP First (User Story 1)

1. Complete setup and foundational inspection.
2. Implement the semantic ribbon wrappers and horizontal quick-folder row.
3. Run the focused manual quick-folder check before broad theme work.

### Incremental Delivery

1. Add the bright theme across existing surfaces.
2. Verify responsive Korean/English behavior.
3. Run tests/build and the no-mutation checklist.

## Notes

- Every task has a checkbox, sequential ID, and exact repository path as required by the spec-kit task format.
- No release build or GitHub upload is included; those remain user-controlled for this iteration.
