# Implementation Plan: Bright Ribbon Workspace

**Branch**: `004-bright-ribbon-ui` | **Date**: 2026-09-13 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/004-bright-ribbon-ui/spec.md`

## Summary

Refine the existing FileMoa ribbon so action buttons, organization options, quick folders, and the selected path have clear ownership and predictable responsive behavior. Apply a bright, warm visual system inspired by the supplied style reference across every existing screen and modal while preserving all file-safety and organization behavior. The implementation reuses the current React/Lucide UI and changes only the ribbon markup and stylesheet; no new runtime dependency or file-operation path is required.

## Technical Context

**Language/Version**: TypeScript 5.9, React 19, CSS, Rust/Tauri 2 (unchanged backend)

**Primary Dependencies**: Vite 7, `lucide-react`, `@tauri-apps/api`, existing Tauri plugins

**Storage**: Existing localStorage and Tauri store usage; no new storage

**Testing**: Vitest utility tests (`npm test -- --run`), TypeScript/Vite production build (`npm run build`)

**Target Platform**: Windows desktop via Tauri NSIS; browser-sized development preview for visual checks

**Project Type**: Desktop app (React frontend + Tauri shell)

**Performance Goals**: Keep ribbon interactions immediate and avoid layout shifts while folders/options render; preserve existing startup and file-operation performance

**Constraints**: Bright theme must remain readable in Korean and English; quick folders must not shrink into vertical text; no file mutation during visual validation; no additional dependencies or network calls

**Scale/Scope**: One existing desktop window, three tabs, settings/update/AI modals, and up to the existing set of known folders

## Constitution Check

*GATE: Must pass before implementation and after design.*

- Safety Before Automation: PASS — this change does not alter preview, approval, move, or Undo logic.
- Protect System/Project Boundaries: PASS — only presentation files and feature documentation are changed.
- Deterministic, Explainable Organization: PASS — no classification or destination rules are changed.
- Local-First Privacy: PASS — no new data transfer or persistence is introduced.
- Testable Quality: PASS — frontend tests and production build are required before completion.
- Reversible Releases: PASS — the change is isolated to a feature directory, `App.tsx`, and `styles.css`; release upload remains user-controlled.

## Project Structure

### Documentation (this feature)

```text
specs/004-bright-ribbon-ui/
├── spec.md          # User stories and acceptance criteria
├── plan.md          # This implementation plan
├── research.md      # Decisions and visual/layout research
├── data-model.md    # UI entities and invariants
├── quickstart.md    # Manual and automated validation guide
└── tasks.md         # Dependency-ordered implementation checklist
```

### Source Code (repository root)

```text
src/
├── App.tsx          # Ribbon structure and existing interaction wiring
├── styles.css       # Bright theme tokens and responsive layout rules
└── utils.ts         # Existing pure helpers and tests (unchanged)
src-tauri/           # Existing Tauri commands and safety-sensitive backend (unchanged)
```

**Structure Decision**: Keep the current single-package Tauri desktop layout. The feature is presentation-only, so introducing component folders or a design-system package would add indirection without improving this focused change. Existing `App.tsx` handlers remain the source of truth for folder selection and file operations.

## Phase 0 — Research Summary

See [research.md](./research.md). The key decisions are to use a CSS-token palette, an explicit three-row ribbon hierarchy, and a non-shrinking horizontally scrollable quick-folder strip.

## Phase 1 — Design Summary

See [data-model.md](./data-model.md), [quickstart.md](./quickstart.md), and the UI contract in `contracts/ribbon-ui.md`. The design keeps all existing labels and event handlers, adds semantic wrapper elements for layout, and defines keyboard/focus and narrow-window invariants.

## Post-Design Constitution Check

- No constitution gate was weakened. The selected design changes layout and colors only; all safety-critical actions remain behind their existing preview/approval state.
- Validation includes a production compile and test run, plus manual no-mutation checks at wide and narrow widths in both languages.
