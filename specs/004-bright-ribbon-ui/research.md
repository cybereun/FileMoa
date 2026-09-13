# Research: Bright Ribbon Workspace

## Decision 1 — Warm bright palette with semantic tokens

- **Decision**: Use warm ivory for the app canvas, white for primary surfaces, warm gray for secondary surfaces and borders, and terracotta for primary emphasis. Keep success, warning, danger, and focus colors distinct from the accent.
- **Rationale**: The supplied style reference uses a calm neutral base with a single earthy highlight. CSS tokens make the palette consistent across tabs, cards, provider settings, update dialogs, and status messages and make future theme work reversible.
- **Alternatives considered**: Reusing the dark palette would not satisfy the requested bright mode. Adding a third-party theme library would increase bundle and maintenance cost for a presentation-only change.

## Decision 2 — Three-level ribbon hierarchy

- **Decision**: Group the ribbon into (1) action buttons, (2) organization controls, and (3) a full-width quick-folder row followed by the selected path.
- **Rationale**: Explicit groups prevent flexbox from shrinking unrelated controls into a narrow column. The quick-folder row can remain readable independently of the variable width of options and labels.
- **Alternatives considered**: A single flex row caused the reported vertical squeezing. A CSS grid with fixed columns would truncate English labels and long paths at smaller widths.

## Decision 3 — Horizontal overflow for quick folders

- **Decision**: Quick-folder buttons are non-shrinking inline items in a horizontally scrollable row; the row label is also non-shrinking.
- **Rationale**: Users keep one-click access to every known folder without forcing a tall ribbon. Horizontal scrolling is predictable when many folders or long localized labels are present and degrades safely on narrow windows.
- **Alternatives considered**: Wrapping into multiple rows increases vertical layout shift and makes the ribbon harder to scan. Ellipsizing button labels without a tooltip hides the destination name.

## Decision 4 — Preserve existing behavior and dependencies

- **Decision**: Change only the ribbon wrapper markup and CSS; reuse current React state, handlers, Lucide icons, and localization labels.
- **Rationale**: This isolates risk from file organization, update, AI, and safety workflows and keeps the app's bundle small.
- **Alternatives considered**: Rebuilding the ribbon as a new component library is unnecessary for the current scope and would make regression testing broader.

## Decision 5 — Validation strategy

- **Decision**: Run existing Vitest tests and the TypeScript/Vite production build, then manually inspect 1280px, 940px, and 520px widths in Korean and English without invoking file operations.
- **Rationale**: Automated checks catch compile/regression issues; width/language checks cover the layout failure shown in the supplied screenshot.
- **Alternatives considered**: Adding a browser test harness would be valuable later, but the project has no browser-test dependency and this focused iteration can be validated with the existing toolchain plus a manual checklist.
