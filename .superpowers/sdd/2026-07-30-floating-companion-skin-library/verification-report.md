# Floating Companion Skin Library Verification Report

Date: 2026-07-31
Branch: `feat/magic-image-library`
Implementation HEAD: `c89e85b`
Review base: `5b592ed`

## Automated Verification

| Check | Result |
| --- | --- |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass |
| `npm test` | Pass: 12 files, 61 tests |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Pass: 71 tests |
| `npm run build` | Pass |
| `npm run tauri -- build --debug --no-bundle` | Pass |
| `git diff --check` | Pass |

Debug executable: `src-tauri/target/debug/magic-image-library.exe`

Non-blocking warning: bundle identifier `com.magicimagelibrary.app` ends in `.app`.

## Review Findings Resolved

1. Production asset IPC failed because repository state was not registered on the AppHandle. Fixed by `6eca1cf`; MockRuntime regression and real asset loading verified.
2. Region capture button omitted `CropRegion`, so the Rust branch always returned `MissingCropRegion`. Fixed by `c89e85b` with a display-sized overlay, DPI conversion, and native restore session.
3. Equivalent skin event payloads could reset an unsaved edit because `SkinControls` synchronized on object identity. Fixed by `8a8d3fa` with a persisted-field revision and deterministic regression test.

No additional load-bearing security, event ordering, focus, keyboard, or test-quality finding remained after these fixes.

## Windows Smoke Matrix

| Behavior | Evidence |
| --- | --- |
| Main and companion startup | Verified |
| Companion always on top | Visually verified |
| Asset repository IPC and July assets | Verified |
| Normal click expands without drag | Verified |
| Quick switch to `deep-ink` | Verified |
| Escape collapse and focus restoration | Verified; Space reopened trigger |
| Skin library shows three built-ins | Verified |
| Hide persistence and main restore | Verified across restart |
| Primary-display RegionOverlay | Verified |
| Escape restores companion bounds | Verified |
| Real region drag and saved crop | Manual verification required; automation interrupted by user input |
| Fullscreen/window capture end to end | Manual verification required |
| Drag, restart, placement restore | Manual verification required |
| Companion close-hide / main close-exit | Manual verification required |
| Motion switch / Windows reduced motion | Manual verification required |
| PNG/WebP/valid ZIP/malicious ZIP dialogs | Manual verification required |
| Image import native dialog | Manual verification required |

## Security Boundaries Verified By Tests

- 10 MiB PNG/WebP source limit, `128..=4096` dimensions, and 32 MiB decode allocation.
- 20 MiB compressed ZIP, 40 MiB uncompressed ZIP, 16 entries, and 64 KiB manifest.
- Traversal, absolute paths, symlinks, directories, unsupported/unreferenced entries, remote URLs, malformed manifests, missing textures, invalid dimensions/colors/motion, and archive limits reject without persisted residue.
- Built-in and active skin protections, managed field validation, exact asset protocol scope, and companion capability scope.
- Event payloads follow persistence; failed skin operations emit no skin-change event.

## Future Scope

Community upload, direct external references, programmable skin formats, cloud sync, annotation workflows, installers, signing, and auto-update remain out of scope and require new design work.
