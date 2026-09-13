# Ribbon UI Contract

This UI contract defines observable behavior for the existing desktop ribbon. It is not a network API.

## Structure

- The ribbon exposes action buttons for the active tab.
- The organize tab exposes options independently from the action group.
- When known folders exist, a labeled quick-folder row exposes one button per folder.
- The selected path is displayed as a separate, ellipsized read-only value.

## Interaction

- Every enabled action and quick-folder button is reachable by mouse click and keyboard focus.
- A disabled action remains disabled under the existing `busy`, `plan`, and count guards.
- Activating a quick-folder button invokes the existing folder-selection callback with its stable id/path.
- Hover and focus provide visible feedback; disabled controls are visibly muted.
- Long paths may be visually ellipsized but retain their full value in the existing path state/title.

## Responsive behavior

- At wide desktop sizes, actions and options share the top ribbon row while quick folders stay in a single readable horizontal row.
- At narrower sizes, action and option groups may wrap or stack; quick-folder buttons remain non-shrinking and the row may scroll horizontally.
- Korean and English labels must not overlap or turn into vertically stacked characters.

## Safety boundary

Rendering or selecting a ribbon control never performs a file move, delete, or undo by itself. Existing preview/approval guards remain authoritative.
