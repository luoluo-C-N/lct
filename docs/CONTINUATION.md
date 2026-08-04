# Magic Image Library - Development Handoff

Updated: 2026-08-04

Status: **The local MVP, floating companion skin library, transparent companion, and frozen desktop region capture are implemented. Windows local library 1.0 Task 1 is complete; resume at Task 2. Automated verification is green at the checkpoint described below.**

## Repository State

- Repository: `https://github.com/luoluo-C-N/lct.git`
- Branch: `feat/magic-image-library`
- Latest completed implementation commit: `831ab81` (`feat: align asset contract and schema v4`)
- Current local pause checkpoint: `f8f12b1` (`wip: pause asset query implementation`)
- Windows local library 1.0 plan: Task 1 complete; Tasks 2-7 remain. Resume with Task 2, unified cursor queries and asset details backend.
- Frozen region capture commits: `83d9966`, `c4b66fa`, `76b5982`, `29cad98`, `785c2fb`, `b2253ec`, `43bcefb`, `76e2275`, `b1540d1`, `2e7f60f`, `7b8e241`.
- Skin edit race fix: `8a8d3fa` (`fix: preserve unsaved skin edits on refresh`)
- Asset repository state fix: `6eca1cf` (`fix: register asset repository state`)
- The final handoff commit is the commit containing this file; run `git log -1 --oneline --decorate` after pulling.
- Stack: Tauri 2, Rust, rusqlite, screenshots, image, zip 8.6.0, React 19, TypeScript, Vite, Vitest.

## Completed Scope

- SQLite schema v4 with in-place v1 -> v2 -> v3 -> v4 migration, assets, tags, companion skins, settings, visibility, placement, asset display names, and query indexes.
- Full Rust and TypeScript asset contracts now include display name, tags, deletion, capture, annotation, and cloud fields; imports preserve source filenames and captures generate non-empty `.png` display names.
- File import preserves source creation time with defensive fallbacks and emits complete `asset-created` payloads after persistence.
- Fullscreen, primary-display region overlay, and Win32 foreground-window capture branches persist `capture_mode` and emit `asset-created`.
- Shared `MonthPicker`, cross-year navigation, MagicBook and ClassicGallery month-aware event refresh, and delayed unlisten cleanup.
- Independent `main` and `companion` Tauri windows; companion is transparent, undecorated, always on top, skipped from the taskbar, and close-to-hide.
- Main-window show/hide control, companion quick actions, keyboard Escape/focus restoration, threshold-based native drag, debounced placement persistence, and anchored expansion.
- Built-in skins `quiet-aurora`, `porcelain-pearl`, and `deep-ink`, explicit visual tokens, motion switch, and reduced-motion CSS.
- Local PNG/WebP skins normalized to `texture.png` and `preview.png`, with safe palette extraction and database/filesystem compensation.
- Versioned ZIP skin packages with traversal, absolute path, symlink, directory, unknown entry, unreferenced entry, remote URL, entry count, manifest size, compressed size, uncompressed size, dimensions, allocation, color, and motion validation.
- Main skin library for import, activation, rename, motion controls, and deletion; built-ins and active skins remain protected.
- Repository states are registered on the Tauri AppHandle, fixing the production `state not managed for field repository` IPC failure.
- The collapsed companion document and native window are transparent through `html`, `body`, and `#root`, with the native shadow disabled so only the orb is visible.
- QQ/WeChat-style region capture hides the companion, takes one primary-display snapshot, shows that frozen desktop frame during selection, and crops the same retained pixels without recapturing.
- Frozen region sessions use primary-monitor DPI conversion, restore the companion before persistence, emit `asset-created` exactly once after success, and remain recoverable after setup, completion, cancellation, restoration, or partial-file failures.

## Security Decisions

- Image import limit: 10 MiB; PNG/WebP only; decoded width and height must each be `128..=4096`; decode allocation ceiling is 32 MiB.
- ZIP limit: 20 MiB compressed, 40 MiB uncompressed, 16 entries, and a 64 KiB root `manifest.json`.
- ZIP extraction is validate-before-write and never uses bulk archive extraction.
- Skin packages contain exactly `manifest.json` plus referenced root PNG/WebP files; executable/web content, remote URLs, parent traversal, absolute paths, symlinks, directories, duplicates, and extra files are rejected.
- Imported skins live under `$APPLOCALDATA/skins/<skin-id>/`; the asset protocol exposes only previews and the managed skin tree.
- Built-in IDs cannot be overwritten, updated, or deleted. Managed path/source/preset/timestamp fields cannot be changed through IPC.
- `create_asset` remains debug-only.

## Final Automated Verification

Executed on Windows on 2026-08-04 at the pause checkpoint:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
# exit 0

cargo test --manifest-path src-tauri/Cargo.toml
# 86 passed, 0 failed

npm test
# 13 files, 71 tests passed

npm run build
# TypeScript and Vite build succeeded
```

Task 2 repository query coverage currently passes 7 focused tests. Task 2 remains incomplete: IPC commands, asset details, asset protocol scope, and TypeScript query wrappers are not implemented yet.

Executed on Windows on 2026-07-31:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
# exit 0

npm test
# 13 files, 71 tests passed

cargo test --manifest-path src-tauri/Cargo.toml
# 82 passed, 0 failed

npm run build
# TypeScript and Vite build succeeded

npm run tauri -- build --debug --no-bundle
# succeeded: src-tauri/target/debug/magic-image-library.exe

git diff --check
# exit 0
```

The build still warns that `com.magicimagelibrary.app` ends in `.app`. This is a non-blocking macOS naming warning; the current target is Windows.

## Windows Smoke Evidence

Verified in the real debug executable:

- Main and companion windows appear independently.
- Main asset list loads two existing July 2026 assets; the previous repository-state IPC error is gone.
- Companion expands by click without swallowing the click as a drag.
- Quick switch to `deep-ink` updates the visual and reports success.
- Escape collapses the menu and keyboard Space can reopen it, demonstrating focus restoration.
- Main skin library displays all three built-ins.
- Companion hide persists across restart; the main window restores it and updates its visibility button.
- Region capture opens a primary-display-sized selection overlay; Escape restores the exact companion window and reports cancellation.
- Main/companion focus behavior and always-on-top placement were visually observed.
- The collapsed companion was launched from the current debug build and desktop content remained visible around the orb; no black rectangular webview/native frame was visible.

Manual verification required:

- Complete a real region drag and confirm the frozen desktop preview and cropped asset. The Windows automation tool could not activate/click the 72x72 borderless always-on-top companion, so this drag was not completed. Automated coverage verifies frozen preview rendering, same-frame cropping, DPI conversion, event count, cancellation, and retry/recovery behavior.
- Exercise fullscreen and foreground-window capture end to end on the target monitor/window.
- Drag the companion, restart, and confirm placement persistence and edge-aware expansion at multiple screen edges.
- Verify companion close hides, main close exits, and show/hide behavior across all close paths.
- Toggle motion and Windows reduced-motion settings and visually confirm all animations stop.
- Import PNG, WebP, valid ZIP, traversal ZIP, oversized ZIP, and malformed manifest through native file dialogs.
- Exercise the image import dialog and all failure/cancel paths in the packaged Windows interaction.

## Known Residuals

- If a Tauri event emit fails after a database/filesystem commit, the command returns an error but does not roll back the committed asset or skin. There is no transaction spanning SQLite, filesystem, and the event bus.
- Frozen region selection currently covers the primary display only; multi-display selection remains future work.
- The `.app` bundle identifier warning remains.
- Community upload, direct external skin references, executable/programmatic skins, accounts, cloud sync, installers, signing, updates, and product-grade annotation remain future work and require new specifications.

## Continue On Another Account

```powershell
git switch feat/magic-image-library
git pull --ff-only
git status --short --branch
git log -1 --oneline --decorate
npm install
npm run tauri -- dev
```

Read this file first, then:

- `docs/superpowers/specs/2026-08-04-windows-local-library-v1-design.md`
- `docs/superpowers/plans/2026-08-04-windows-local-library-v1.md`
- `docs/superpowers/specs/2026-07-31-transparent-companion-desktop-region-capture-design.md`
- `docs/superpowers/plans/2026-07-31-transparent-companion-desktop-region-capture.md`
- `docs/superpowers/specs/2026-07-30-floating-companion-skin-library-design.md`
- `docs/superpowers/plans/2026-07-30-floating-companion-skin-library.md`
- `docs/superpowers/plans/2026-07-31-region-overlay-capture.md`
- `.superpowers/sdd/2026-07-30-floating-companion-skin-library/verification-report.md`

Do not reset or discard a dirty worktree. Treat GitHub, this document, and the local bundle as the source of truth rather than relying on prior account conversation history.

## Offline Handoff Bundle

The local-only bundle is `handoff/magic-image-library.bundle`; `handoff/manifest.json` records its branch, HEAD, timestamp, and sanitized remote.

Restore with:

```powershell
git clone C:\path\to\handoff\magic-image-library.bundle C:\path\to\new\magic-image-library
Set-Location C:\path\to\new\magic-image-library
git remote set-url origin https://github.com/luoluo-C-N/lct.git
git fetch origin
git switch feat/magic-image-library
```

Refresh only from a clean, fully committed worktree:

```powershell
pwsh -NoProfile -File scripts\refresh-handoff.ps1
```
