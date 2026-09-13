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
byte size, modified timestamp, and collision flag. Status is proposed, skipped, failed,
executed, undone, or an explicit Undo conflict/missing result.

## OrganizationPlan

Selected root(s), scan mode, language, creation time, actions, exclusions, and analysis
errors. A multi-root plan keeps each action inside its originating root. It is immutable
after approval.

## OrganizationRun

Approved plan identifier, execution time, final action results, and Undo state. Undo
states are available, partially completed, completed, or unavailable.

## CustomCategory

User-owned `OrganizationRule` with local display order and mutable enablement.

## DuplicateGroup

Full SHA-256, common byte size, member file paths, and reclaimable bytes. Removal is an
explicit, confirmed Recycle Bin operation; permanent deletion is never used.

## AnalysisCandidate

Large files and empty folders are read-only candidates. They are never changed until
the user selects and confirms a Recycle Bin action.

## State Transitions

`Plan created → Previewed → Approved → Executed → Undo available → Undo previewed
→ Undone`. Failed actions remain explicitly recorded.
