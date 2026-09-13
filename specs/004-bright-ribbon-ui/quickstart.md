# Quickstart Validation: Bright Ribbon Workspace

## Prerequisites

- Windows or a development environment with Node.js and the repository dependencies installed.
- Run commands from `F:\\Codex-F\\smartfile`.

## Automated checks

```powershell
npm test -- --run
npm run build
```

Expected result: all existing Vitest tests pass and the TypeScript/Vite production build completes without errors.

Validation record (2026-09-13): `npm test -- --run` passed 6/6 tests; `npm run build` completed successfully; `cargo test --manifest-path src-tauri/Cargo.toml` passed 13/13 Rust tests. The development preview was checked at 1280px, 940px, and 520px; at 520px the four action buttons form a balanced 2×2 grid, while options wrap without overlap. The browser preview cannot invoke Tauri commands, so its expected startup `invoke` error was not treated as an app-runtime result.

## Manual visual scenarios

1. Start the frontend with `npm run dev` (or launch the Tauri app through the existing development workflow).
2. At approximately 1280px wide, open the **정리 / Organize** tab. Confirm action buttons, options, six quick-folder buttons, and the selected-path field are readable and do not form a vertical squeezed column.
3. Click one quick-folder button. Confirm the selected path updates and stop before preview/move; this check must not move or delete files.
4. Switch the language in settings and repeat the wide-window check in English and Korean.
5. Resize to approximately 940px and 520px. Confirm action buttons wrap, options remain reachable, and the quick-folder row scrolls horizontally without overlapping controls.
6. Open **관리 / Manage**, **안전 / Safety**, AI settings, and the update dialog. Confirm the warm bright palette, readable text, focus rings, status badges, and modal buttons.
7. Close the app without running a file operation. Verify that no test folder contents changed.

## Traceability

- Ribbon layout and keyboard/overflow invariants: [contracts/ribbon-ui.md](contracts/ribbon-ui.md)
- Entity responsibilities and state invariants: [data-model.md](data-model.md)
- User-facing acceptance criteria: [spec.md](spec.md)
