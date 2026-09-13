# Feature Specification: AI Providers and User-Controlled Updates

**Feature Branch**: `002-ai-provider-and-update-controls`
**Status**: Complete

## Goal

Extend FileMoa with four selectable AI providers and a safe, user-controlled
GitHub update flow without weakening the local-first file organization model.

## Decisions

- The user selects the provider for each AI classification run; there is no
  automatic provider fallback.
- AI classification is available only after the user presses the explicit AI
  action. The normal rule-based preview never calls an AI service.
- External providers receive only basename, extension, size, and timestamps.
  File contents and full paths are never sent.
- AI suggestions are shown in a review preview. They never move files directly.
- Existing categories are preferred. A new category can be proposed, but only
  an explicit user approval can create it.
- Low-confidence suggestions go to `Review required` and are not moved.
- OpenAI, Anthropic Claude, and Google Gemini use user-supplied API keys.
  Ollama is detected only at `http://127.0.0.1:11434` and uses installed models.
- Keys are protected with Windows DPAPI and never returned to the UI after save.
- API keys are tested only when the user presses `Connection test`.
- Provider registration links and editable model IDs are shown in settings.
- Startup update checks are silent and set a badge only. A user-initiated check
  opens a version/release-notes dialog with `Install now` and `Install later`.
- Installation failure keeps the current version and exposes retry; automatic
  installation on startup is forbidden.

## Acceptance scenarios

1. Given a provider key is saved, when FileMoa restarts, then the provider shows
   `key saved` without displaying the key and the key can be used for a test.
2. Given Ollama is running locally, when settings refresh, then FileMoa lists its
   installed models. It must not scan network addresses or download models.
3. Given an AI classification run, when the user confirms the transfer summary,
   then only approved metadata is sent and the response appears as suggestions.
4. Given a newer signed GitHub release, when the user presses `Check updates`,
   then the dialog shows its version and release notes without installing it.
5. Given `Install later`, when the app is started again, then the update badge and
   check remain available. Given an installation failure, then the old version
   remains runnable.

## Non-goals

No raw AI prompts/responses are persisted, no file content is uploaded, and no
Windows Authenticode certificate is assumed for the current unsigned installer.
