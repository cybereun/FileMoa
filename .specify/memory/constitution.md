<!--
Sync Impact Report
- Version change: template → 1.0.0
- Modified principles: template placeholders → all five FileMoa principles
- Added sections: Safety Boundaries; Delivery and Quality Gates
- Removed sections: none
- Templates requiring updates: ✅ .specify/templates/plan-template.md; ✅ .specify/templates/spec-template.md; ✅ .specify/templates/tasks-template.md
- Follow-up TODOs: none
-->

# FileMoa Constitution

## Core Principles

### I. Safety Before Automation
FileMoa MUST show a complete dry-run preview before any bulk change, MUST require
explicit approval to execute it, MUST prefer moving over deleting, and MUST offer
an Undo record for every completed organization run. This protects user data and
makes automation trustworthy.

### II. Protect System and Project Boundaries
FileMoa MUST exclude protected Windows locations, application-data areas, detected
development projects, configured exclusions, locked paths, and sensitive file
patterns from automatic changes. It MUST report exclusions and failures without
silently bypassing them. This prevents system damage and broken source trees.

### III. Deterministic, Explainable Organization
Every planned file action MUST state its source, destination, matched rule, and
collision outcome. Rule priority is protection, user rule, project protection,
filename rule, date rule, extension rule, then Other. Duplicate detection MUST
confirm identical contents before offering a removal action. This lets users verify
and reproduce each result.

### IV. Local-First Privacy and Resilience
File analysis, organization plans, history, preferences, and Undo data MUST remain
on the local computer for v1. The application MUST work offline except for an
explicitly user-initiated update check. This avoids uploading personal file names,
paths, or content and keeps core work dependable.

### V. Testable Quality and Reversible Releases
Core classification, safety checks, collision naming, plan generation, execution,
and Undo MUST have automated tests. A release MUST build successfully before it is
published; updater metadata MUST be cryptographically signed and the updater MUST
verify it before installing. User-facing Korean and English text MUST be kept in
the same feature path. This preserves quality across local use and upgrades.

## Safety Boundaries

The default mode is Safe: move files only, do not delete, and minimize renaming.
Overwrite is disabled by default; collisions receive an incremented filename. The
application MUST not force-download cloud-only files, MUST ignore in-progress and
temporary downloads, and MUST preserve errors in the run log. Filesystem actions
occur only in folders that the user explicitly selected for that run.

## Delivery and Quality Gates

Features MUST begin with a written specification, plan, and dependency-ordered
task list. Implementation MUST pass the Constitution Check before coding and after
design. A change is complete only after its automated tests and a relevant local
build or run check pass. Releases use GitHub Releases and a GitHub Actions workflow;
Windows Authenticode signing is optional until the owner supplies a valid certificate.

## Governance

This constitution supersedes conflicting development conventions. Amendments MUST
document their rationale, update this file and affected templates or guidance, and
use semantic versioning: MAJOR for incompatible governance changes, MINOR for new
or materially expanded principles, and PATCH for clarifications. Every plan,
implementation review, and release review MUST verify compliance with these
principles. The product guideline at `windows_file_organizer_guidelines.md` defines
the feature baseline and does not override these safety rules.

**Version**: 1.0.0 | **Ratified**: 2026-09-12 | **Last Amended**: 2026-09-12
