# Transparent Companion and Desktop Region Capture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the companion window's rectangular background and make region capture crop the same frozen desktop frame shown during selection.

**Architecture:** Mark the companion document before React renders so every webview root layer is transparent, and disable the native shadow in Tauri configuration. Replace the resize-only region session with a mutex-protected Rust session that hides the companion, captures and PNG-encodes the primary display once, shows that frozen frame in `RegionOverlay`, then restores the window and crops the retained pixels through dedicated complete/cancel commands.

**Tech Stack:** Tauri 2, Rust 2021, `screenshots`, `image`, `base64`, React 19, TypeScript, Vitest, Testing Library.

## Global Constraints

- Keep the configured window labels exactly `main` and `companion`; do not create a third native window.
- Region selection remains limited to the primary display.
- The companion must be hidden before the one and only desktop snapshot.
- The preview and persisted crop must come from the same in-memory `RgbaImage`.
- Region completion must not call the general live-screen `capture('region', ...)` path.
- Success emits exactly one `asset-created`; cancellation and failure emit none.
- Every terminal path restores the original companion bounds and visibility, except a restore failure retains the session for retry.
- Use TDD and commit each task independently.

---

### Task 1: Fully Transparent Collapsed Companion

**Files:**
- Create: `src/lib/windowDocument.ts`
- Create: `src/lib/windowDocument.test.ts`
- Modify: `src/main.tsx`
- Modify: `src/app.css`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/asset_protocol_config_test.rs`

**Interfaces:**
- Produces: `markWindowDocument(windowLabel: string, root?: HTMLElement): void`.
- Produces: `data-window="companion"` on the document root before React renders.
- Preserves: existing `Root windowLabel` routing and main-window background.

- [ ] **Step 1: Write failing frontend and Rust contract tests**

Add a Vitest case that calls `markWindowDocument('companion', element)` and expects `element.dataset.window === 'companion'`, plus a main-window case. Extend `config_declares_a_safe_companion_window` with:

```rust
assert_eq!(companion.shadow, Some(false));
```

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
npm test -- windowDocument
cargo test --manifest-path src-tauri/Cargo.toml config_declares_a_safe_companion_window
```

Expected: frontend import is missing and Tauri config shadow is not explicitly disabled.

- [ ] **Step 3: Implement the transparent document contract**

Implement:

```ts
export function markWindowDocument(windowLabel: string, root = document.documentElement) {
  root.dataset.window = windowLabel;
}
```

Call it in `main.tsx` immediately after reading the current window label. Scope transparent backgrounds to:

```css
html[data-window="companion"],
html[data-window="companion"] body,
html[data-window="companion"] #root { background: transparent; }
```

Set `"shadow": false` on the `companion` Tauri window only.

- [ ] **Step 4: Verify GREEN and commit**

Run both focused tests, then:

```powershell
git add src/lib/windowDocument.ts src/lib/windowDocument.test.ts src/main.tsx src/app.css src-tauri/tauri.conf.json src-tauri/src/asset_protocol_config_test.rs
git commit -m "fix: remove companion window frame"
```

---

### Task 2: Reusable In-Memory Screen Snapshot

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/src/services/capture.rs`
- Modify: `src-tauri/src/services/capture_test.rs`

**Interfaces:**
- Produces: `pub trait ScreenCapturer: Send + Sync { fn capture_primary(&self) -> Result<RgbaImage, CaptureError>; }`.
- Produces: `pub struct ScreenshotCapturer` as the production adapter.
- Produces: `encode_png_data_url(&RgbaImage) -> Result<String, CaptureError>`.
- Produces: `persist_captured_pixels(RgbaImage, CaptureMode, &Path, &AssetRepository) -> Result<Asset, CaptureError>`.
- Preserves: existing fullscreen, region, and foreground-window capture behavior.

- [ ] **Step 1: Write failing service tests**

Add literal-pixel tests proving that `encode_png_data_url` starts with `data:image/png;base64,` and decodes to the source dimensions/pixel, and that `persist_captured_pixels` creates a region asset while removing its temporary capture file. Add a fake `ScreenCapturer` test proving one call returns the supplied image.

- [ ] **Step 2: Run tests and verify RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml services::capture_test -- --nocapture
```

Expected: the new trait and helpers do not exist.

- [ ] **Step 3: Implement the minimal snapshot primitives**

Add direct dependency `base64 = "0.22"`. Move primary-display lookup/capture behind `ScreenshotCapturer`, encode PNG with `image::codecs::png::PngEncoder`, and make existing `capture_target` reuse the adapter for primary fullscreen/region capture where possible. Persist pixels through a unique `capture-{new_asset_id()}.png` temporary path and existing `persist_captured_image` cleanup.

- [ ] **Step 4: Verify GREEN, full Rust regression, and commit**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml services::capture_test
cargo test --manifest-path src-tauri/Cargo.toml
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/services/capture.rs src-tauri/src/services/capture_test.rs
git commit -m "refactor: add reusable screen snapshots"
```

---

### Task 3: Native Frozen Region Session and IPC Boundary

**Files:**
- Modify: `src-tauri/src/commands/companion.rs`
- Modify: `src-tauri/src/commands/companion_test.rs`
- Modify: `src-tauri/src/asset_protocol_config_test.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Produces: `RegionSelectionSession { scale_factor: f64, preview_data_url: String }`.
- Produces: `begin_companion_region_selection(app, state)` using `ScreenshotCapturer`.
- Produces: `complete_companion_region_selection(region, app, state, repository) -> Asset`.
- Produces: `cancel_companion_region_selection(app, state) -> ()`.
- Replaces: `finish_companion_region_selection`.
- Consumes: Task 2 `ScreenCapturer`, `encode_png_data_url`, and `persist_captured_pixels`.

- [ ] **Step 1: Write failing lifecycle tests**

Extend the fake window with visible state and an operation log. Inject a fake capturer and assert this exact successful prefix:

```text
bounds -> monitor_bounds -> scale_factor -> hide -> capture -> set_monitor_bounds -> show -> focus
```

Assert a second begin is rejected and capture count remains one. Assert capture, resize, show, and focus failures restore the original bounds/visibility and leave no active session.

- [ ] **Step 2: Run lifecycle tests and verify RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml commands::companion_test::region_selection -- --nocapture
```

Expected: the current window adapter cannot hide/show/focus or retain pixels.

- [ ] **Step 3: Implement begin and cancel session behavior**

Store a `RegionCaptureSession` containing original bounds and `RgbaImage` under the existing mutex. `begin` hides before capture, stores only after capture and preview encoding succeed, then covers/shows/focuses the window. Roll back bounds and visibility on every setup error. `cancel` restores first and clears only after restoration succeeds.

- [ ] **Step 4: Write failing completion and event tests**

Using a 4x4 literal image, complete a 2x2 crop and inspect the persisted original pixels. Assert `capture_mode == Some(CaptureMode::Region)`, the session is empty, and exactly one `asset-created` payload equals the returned asset. Add invalid-region, persistence-error, cancellation, and restore-error cases proving zero events; restore error must keep the session available for retry.

- [ ] **Step 5: Run completion tests and verify RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml region_selection -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml frozen_region -- --nocapture
```

Expected: complete/cancel commands and event behavior are missing.

- [ ] **Step 6: Implement completion commands and registration**

Restore bounds and visibility before taking the session image. Validate/crop/persist through Task 2 helpers, emit only after persistence, and clear the session on all post-restore results. Register `complete_companion_region_selection` and `cancel_companion_region_selection` in `main.rs`; remove the old finish command.

- [ ] **Step 7: Verify GREEN, full Rust regression, and commit**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
git add src-tauri/src/commands/companion.rs src-tauri/src/commands/companion_test.rs src-tauri/src/asset_protocol_config_test.rs src-tauri/src/main.rs
git commit -m "fix: capture frozen desktop regions"
```

---

### Task 4: Frozen Preview Overlay and Frontend Lifecycle

**Files:**
- Modify: `src/lib/companion.ts`
- Modify: `src/lib/companion.test.ts`
- Modify: `src/features/floating-companion/RegionOverlay.tsx`
- Modify: `src/features/floating-companion/RegionOverlay.test.tsx`
- Modify: `src/features/floating-companion/CompanionWindow.tsx`
- Modify: `src/features/floating-companion/CompanionWindow.test.tsx`
- Modify: `src/app.css`

**Interfaces:**
- Produces: `RegionSelectionSession = { scaleFactor: number; previewDataUrl: string }`.
- Produces: `completeCompanionRegionSelection(region) -> Promise<Asset>`.
- Produces: `cancelCompanionRegionSelection() -> Promise<void>`.
- Changes: `RegionOverlay` requires `previewDataUrl` and renders the frozen frame beneath selection chrome.

- [ ] **Step 1: Write failing IPC and overlay tests**

Assert literal invoke payloads for begin, complete, and cancel. Render `RegionOverlay` with `data:image/png;base64,fixture` and assert the image has accessible name `截图冻结画面` and that reverse drag still emits the hand-calculated physical region.

- [ ] **Step 2: Run focused tests and verify RED**

Run:

```powershell
npm test -- companion RegionOverlay
```

Expected: complete/cancel wrappers and preview prop are missing.

- [ ] **Step 3: Implement frontend contracts and preview**

Replace finish wrapper with complete/cancel wrappers. Render the preview as a non-draggable image filling the overlay; keep selection chrome above it and pointer events on the dialog surface.

- [ ] **Step 4: Write failing companion workflow tests**

Assert region start displays the begin response preview, completing calls only `completeCompanionRegionSelection(region)` and never general `capture`, Escape calls cancel exactly once, and complete/cancel errors leave the collapsed companion usable with `role="alert"` feedback.

- [ ] **Step 5: Run workflow tests and verify RED**

Run:

```powershell
npm test -- CompanionWindow
```

Expected: current workflow restores then performs a second live capture.

- [ ] **Step 6: Implement the frozen frontend workflow**

Store the complete region session instead of only its scale factor. On selection, clear overlay state and call complete. On Escape, clear overlay state and call cancel. Do not call `capture('region', region)`. Ensure error paths cannot trigger duplicate cancellation during unmount or rerender.

- [ ] **Step 7: Verify GREEN, full frontend regression, and commit**

Run:

```powershell
npm test -- companion RegionOverlay CompanionWindow
npm test
npm run build
git add src/lib/companion.ts src/lib/companion.test.ts src/features/floating-companion/RegionOverlay.tsx src/features/floating-companion/RegionOverlay.test.tsx src/features/floating-companion/CompanionWindow.tsx src/features/floating-companion/CompanionWindow.test.tsx src/app.css
git commit -m "fix: show frozen desktop during region capture"
```

---

### Task 5: End-to-End Verification and Handoff

**Files:**
- Modify: `docs/CONTINUATION.md`
- Modify: `docs/superpowers/plans/2026-07-31-transparent-companion-desktop-region-capture.md`
- Refresh: `handoff/magic-image-library.bundle`

**Interfaces:**
- Documents: exact commits, automated counts, and remaining/manual Windows evidence.
- Produces: an offline Git bundle containing the final branch.

- [ ] **Step 1: Run the complete verification matrix**

Run:

```powershell
npm test
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm run build
npm run tauri -- build --debug --no-bundle
```

Expected: every command exits 0 with no failing tests.

- [ ] **Step 2: Perform Windows smoke checks**

Launch the debug executable and verify: collapsed corners are transparent; region mode shows desktop and other apps as a frozen frame; a saved crop excludes the orb/overlay; Escape restores the exact orb position. Record any check that cannot be completed instead of claiming it passed.

- [ ] **Step 3: Update handoff records and commit**

Mark completed plan checkboxes, update `docs/CONTINUATION.md` with fresh counts and smoke evidence, then:

```powershell
git add docs/CONTINUATION.md docs/superpowers/plans/2026-07-31-transparent-companion-desktop-region-capture.md
git commit -m "docs: hand off frozen region capture"
```

- [ ] **Step 4: Refresh offline bundle and synchronize**

Create/replace `handoff/magic-image-library.bundle` from `feat/magic-image-library`, push the branch, and verify local HEAD, remote branch, and bundle branch resolve to the same commit.
