# Feature Specification: Safe File Organization MVP

**Feature Branch**: `001-safe-file-organization`
**Created**: 2026-09-12
**Status**: Ready for planning
**Input**: User description: "Build FileMoa, a bilingual Windows app that safely
organizes user-selected folders, previews every change, supports Undo, and updates
from GitHub Releases."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Safely preview and organize a chosen folder (Priority: P1)

A person selects one folder, chooses whether subfolders are included, and reviews a
type-based organization plan before approving it. FileMoa creates category folders
inside the selected folder and moves only eligible files with collision-safe names.

**Why this priority**: This is the essential promise of FileMoa: organize scattered
files without unexpected changes.

**Independent Test**: In a sample folder containing supported files, an unsupported
file, a protected project subfolder, and a naming collision, create a preview and
approve it; verify only eligible files move to the stated destinations.

**Acceptance Scenarios**:

1. **Given** a user-selected folder with PDF, HWPX, PNG, MP4, ZIP, EXE, and unknown
   files, **When** the user creates a plan, **Then** the preview groups each eligible
   file into its Korean or English category and puts unknown files in Other.
2. **Given** a plan with a destination filename that already exists, **When** the
   user executes it, **Then** FileMoa retains both files by appending a number to the
   incoming filename.
3. **Given** a nested folder that is recognized as a development project, **When**
   subfolder scanning is enabled, **Then** its contents are excluded and the preview
   explains why.

---

### User Story 2 - Customize organization rules (Priority: P2)

A person creates, edits, enables, or disables a named category with one or more
file extensions. The rule appears in future preview plans and takes precedence over
the built-in extension grouping.

**Why this priority**: People organize files around their own work, not only generic
file types.

**Independent Test**: Add an enabled "Receipts" category for PDF files, create a
plan for a matching file, and verify it is proposed under Receipts rather than the
built-in document category.

**Acceptance Scenarios**:

1. **Given** an enabled custom category, **When** an eligible file has a matching
   extension, **Then** its planned destination uses that category.
2. **Given** a disabled custom category, **When** a plan is generated, **Then** it
   does not affect file classification.

---

### User Story 3 - Recover and audit an organization run (Priority: P3)

A person sees past runs and can undo the latest completed run. FileMoa restores
files to their original locations when safe and records any restoration conflict or
failure for review.

**Why this priority**: Recovery is the foundation of user trust for file automation.

**Independent Test**: Execute a plan, select Undo from history, and verify each
still-unmodified destination file returns to its original path.

**Acceptance Scenarios**:

1. **Given** a completed run, **When** the user chooses Undo, **Then** FileMoa shows
   a restore preview before moving files back.
2. **Given** an original path now has a conflicting filename, **When** Undo runs,
   **Then** FileMoa does not overwrite it and reports the conflict.

---

### User Story 4 - Find duplicate candidates (Priority: P4)

A person scans selected folders for files with identical contents and reviews each
duplicate group. FileMoa never deletes or moves a duplicate without an explicit
choice.

**Why this priority**: Duplicate visibility adds value while retaining the safety
model of the MVP.

**Independent Test**: Put two identical files with different names in a selected
folder, scan for duplicates, and verify they appear in one group with the same size.

**Acceptance Scenarios**:

1. **Given** files of different sizes, **When** duplicate scanning runs, **Then**
   they are not content-hashed against each other.
2. **Given** byte-identical files, **When** scanning completes, **Then** they are
   shown together with their full paths and total reclaimable size.

## Edge Cases

- A file is locked, read-only, deleted, moved, or permission-denied after preview.
- A selected path is a protected system location, a cloud-only file, or a disconnected
  removable/network drive.
- A file path is too long or has an unsupported destination filename.
- The target folder already contains FileMoa category folders from an earlier run.
- The folder contains `.git`, `package.json`, `pyproject.toml`, or another project
  marker at its root or below it.
- A browser download is incomplete (`.crdownload`, `.part`, or `.tmp`).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST let users select exactly one folder for a new
  organization run and choose either current-folder-only or include-subfolders.
- **FR-002**: The system MUST reject protected Windows and application locations
  before analysis and explain the exclusion in the interface.
- **FR-003**: The system MUST classify eligible files using the supplied guideline's
  default categories, including HWP and HWPX, and route unmatched files to Other.
- **FR-004**: The system MUST create category folders inside the selected folder by
  default and MUST not alter a file until the user approves a dry-run preview.
- **FR-005**: The preview MUST show planned moves, destination folders, collisions,
  exclusions, and failures separately, including the matched rule for each move.
- **FR-006**: The system MUST detect development project markers and exclude the
  detected project directory and its descendants from automatic organization.
- **FR-007**: The system MUST skip hidden system files and incomplete downloads by
  default and MUST not force cloud-only files to download.
- **FR-008**: The system MUST resolve destination name collisions by adding an
  incrementing number and MUST keep overwrite disabled by default.
- **FR-009**: The system MUST persist user-defined extension categories locally and
  apply enabled custom categories before built-in extension rules.
- **FR-010**: The system MUST save local run history with actions, exclusions,
  failures, timestamps, and Undo eligibility.
- **FR-011**: The system MUST provide an Undo preview and restore eligible files to
  their original paths without overwriting existing files.
- **FR-012**: The system MUST find duplicate candidates by grouping file size before
  comparing complete content hashes and MUST require explicit user action for any
  later removal or relocation.
- **FR-013**: The system MUST provide Korean and English UI selection and persist
  the selected language locally.
- **FR-014**: The system MUST check GitHub Releases for a newer signed update when
  explicitly requested and present a user-controlled update action.
- **FR-015**: The system MUST record actionable errors without exposing file content
  or sending file metadata to a remote service.

### Key Entities

- **Organization Rule**: A built-in or user-defined enabled category and its matching
  conditions, display names, precedence, and destination folder name.
- **Organization Plan**: A dry-run snapshot for one selected root and scan mode,
  containing proposed actions, exclusions, collisions, and errors.
- **Planned Action**: One explained potential file move with source, destination,
  matched rule, collision resolution, and execution result.
- **Organization Run**: An approved plan's execution record and its Undo state.
- **Custom Category**: A locally stored category name, localized labels, extensions,
  enablement, and display order.
- **Duplicate Group**: Files confirmed to have identical full-content hashes, their
  common size, and their review state.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can choose a folder, generate a preview, and understand every
  proposed change in under three minutes for a folder of up to 1,000 files.
- **SC-002**: A 1,000-file local folder receives a type-based preview in under
  15 seconds on a typical consumer Windows computer, excluding unreadable files.
- **SC-003**: 100% of files moved by a completed unmodified run are restorable by
  Undo without overwriting a newer file.
- **SC-004**: In the supported default extension set, at least 95% of sample files
  are assigned to the intended category or explicitly shown as Other.
- **SC-005**: Korean and English users can complete the primary preview-and-approve
  flow with no untranslated primary actions.

## Assumptions

- v1 is a local Windows desktop application; dates, AI classification, scheduling,
  real-time monitoring, deletion workflows, and cloud providers are deferred.
- The default Safe mode only moves files after approval; it never deletes files.
- A selected folder is an ordinary user-writable local folder; inaccessible files
  are skipped and logged instead of prompting for elevation.
- Standard category folder names follow the active UI language; existing folders are
  reused without renaming them.
- GitHub Releases uses signed updater metadata. Windows Authenticode signing is
  deferred until the repository owner supplies a certificate.
