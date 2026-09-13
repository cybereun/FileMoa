# Implementation Plan: AI Providers and User-Controlled Updates

## Architecture

- Keep filesystem organization in Rust.
- Add `src-tauri/src/ai.rs` for provider metadata, DPAPI-backed key storage,
  Ollama localhost probing, and connection tests.
- Expose only key-presence/configuration status to React; never return secrets.
- Keep provider-specific HTTP calls in Rust with TLS verification enabled.
- Keep update download/install behind a modal action; startup only checks and
  sets UI state.
- Keep release signing (Tauri updater signature) separate from optional Windows
  Authenticode signing.

## Verification

- `npm run build` for the bilingual ribbon and settings UI.
- `cargo fmt --check` for Rust source.
- `cargo check`/`cargo test` on a machine with sufficient Windows paging space.
- Manual checks: API key is absent from returned JSON and the settings file only
  contains encrypted byte arrays; Ollama probes only loopback; update check never
  calls `downloadAndInstall` until the modal's install button is pressed.
