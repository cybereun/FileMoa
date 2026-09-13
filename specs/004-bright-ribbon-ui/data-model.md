# UI Data Model: Bright Ribbon Workspace

This feature does not introduce persisted domain data. The following entities describe layout responsibilities and invariants over existing state.

## RibbonLayout

Represents the visible organization controls for the active tab.

| Field | Type | Invariant |
| --- | --- | --- |
| activeTab | `organize \| manage \| safety` | Selects the existing action set; no handler changes. |
| actions | existing ribbon action descriptors | Each action retains its existing disabled and busy rules. |
| options | existing organize state | Options remain keyboard reachable and do not overlap actions. |
| quickFolders | `QuickFolderItem[]` | May be empty; items never shrink below readable button width. |
| selectedPath | string | Displayed with ellipsis only after the full value remains available via title/path state. |

## QuickFolderItem

Represents one existing known-folder shortcut.

| Field | Type | Invariant |
| --- | --- | --- |
| id | string | Stable identifier from the existing known-folder list. |
| path | string | Actual folder path used by the existing selection handler. |
| label | localized string | Korean/English label remains on one readable line; title exposes the path. |
| select | existing callback | Calls the current `chooseKnownFolder` flow and does not mutate files. |

## ThemePalette

Semantic visual roles used by all existing screens.

| Role | Meaning | Required contrast behavior |
| --- | --- | --- |
| canvas | App background | Warm ivory; dark ink text remains readable. |
| surface | Cards, panels, modals | White or near-white against canvas. |
| surfaceMuted | Inputs, path, secondary controls | Warm gray distinct from surface. |
| accent | Primary action, active tab, focus | Terracotta; never the only status signal. |
| success/warning/danger | Result states | Distinct text and border/badge treatments on light surfaces. |

## State transitions

No new state transitions are added. Existing transitions remain:

`no folder → folder selected → plan preview → approved move → undo/history`

The theme and layout render each state without changing its transition guards.
