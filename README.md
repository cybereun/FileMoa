# FileMoa

FileMoa is a Korean/English Windows app that previews safe file organization before
moving anything. It classifies common documents (including HWP/HWPX), media,
archives, installers, development files, and Other. It also protects detected project
folders and avoids overwrite collisions.

## Run locally

```powershell
npm install
npm run tauri dev
```

## Test and build

```powershell
npm run release:check
npm run build:nsis
```

The NSIS setup executable is created under
`src-tauri\target\release\bundle\nsis\`.

## Release and auto-update

1. Generate an updater keypair once: `npx tauri signer generate -w .\keys\filemoa.key`.
2. Add the generated public key to `src-tauri/tauri.conf.json` under
   `plugins.updater.pubkey`.
3. Add the private key contents as the GitHub repository secret
   `TAURI_SIGNING_PRIVATE_KEY`; add its password as
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` if one was used.
4. Bump the version in `package.json`, `Cargo.toml`, and `tauri.conf.json`, commit,
   push, and create a `v0.1.0` tag. GitHub Actions builds the NSIS installer and draft
   Release. Publish the draft after reviewing its release notes and update assets.

Updater signatures verify the origin of updates. They are different from Windows
Authenticode signing; without an Authenticode certificate, Windows SmartScreen can
still display a warning for the first download.
