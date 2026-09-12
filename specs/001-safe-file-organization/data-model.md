# Data Model: Safe File Organization MVP

## OrganizationRule

| Field | Description | Validation |
|---|---|---|
| id | Stable identifier | Unique, nonempty |
| labels | Korean and English names | Both nonempty |
| folderName | Destination folder segment | Safe relative path |
| extensions | Lowercase extension list | Dotless, unique |
| precedence | Evaluation order | Custom before built-in |
| enabled | Participation flag | Boolean |

## PlannedAction

Source path, collision-resolved destination path, matching rule, status, explanation,
and collision index. Status is proposed, excluded, skipped, failed, executed, or undone.

## OrganizationPlan

Local identifier, selected root, subfolder choice, creation time, actions, exclusions,
and analysis errors. It is immutable after approval.

## OrganizationRun

Approved plan identifier, execution time, final action results, and Undo state. Undo
states are available, partially completed, completed, or unavailable.

## CustomCategory

User-owned `OrganizationRule` with local display order and mutable enablement.

## DuplicateGroup

Full SHA-256, common byte size, member file paths, and reclaimable bytes. No automatic
deletion action exists in v1.

## State Transitions

`Plan created → Previewed → Approved → Executed → Undo available → Undo previewed
→ Undone`. Failed actions remain explicitly recorded.
