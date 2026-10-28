# CLAUDE.md — Stingray File Manager

Read `SPEC.md` first; it is the source of truth for scope and behaviour.

## Stack

- Tauri 2 (Rust in `src-tauri/`), Svelte 5 + TypeScript + Vite in `src/`.
- rclone runs as a sidecar (`rclone rcd`). The frontend never talks to it directly; every call goes
  through the Rust `rclone::client` module. The rclone, remotes and files modules come from Orbit
  (quietbytehq/orbit); keep fixes in sync between the two projects where it makes sense.

## Rules

- **Credentials:** only in rclone's config file. Never log tokens or passwords.
- **Types in sync:** when a Rust serde struct changes, update `src/lib/api/types.ts` in the same commit.
- **Errors:** commands return `Result<T, AppError>`; `message` is plain language, `details` the raw text.
- **Platform code:** Windows-only code behind `cfg(windows)`; Linux must build without it.
- **Wording:** plain, sentence case. Don't show "remote", "mount", "rcd" or "RC" in the UI.
- **Icons:** regenerate with `python3 assets/logo/gen.py`; don't edit generated PNGs by hand.

## Commands

```
npm install
npm run fetch:rclone
npm run tauri dev
npm run check && npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run bundle:linux     # or bundle:windows
```
