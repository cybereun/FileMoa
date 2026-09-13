# Tasks: AI Providers and User-Controlled Updates

## Phase 1 — Secure provider settings

- [X] Add provider defaults, registration URLs, and editable model settings.
- [X] Store external API keys using Windows DPAPI in per-user app data.
- [X] Return only masked key presence and provider status to React.
- [X] Add Ollama localhost detection and installed-model listing.
- [X] Add explicit connection-test commands for all four providers.

## Phase 2 — UI and update safety

- [X] Add bilingual AI settings panel with provider selection, model field,
  save/remove buttons, connection test, and official registration links.
- [X] Add silent startup update check and update-available badge.
- [X] Add release-notes/version modal with Install now/Later actions.
- [X] Keep failed installations on the existing version and show retryable error.

## Phase 3 — Remaining integration

- [X] Add metadata-only AI classification command and structured response parser.
- [X] Add transfer-count/cost confirmation before every external AI request.
- [X] Add confidence/review-required queue and approved-category merge flow.
- [X] Add UI tests for provider selection, secret masking, and update modal states.
