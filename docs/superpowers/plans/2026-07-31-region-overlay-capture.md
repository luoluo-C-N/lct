# Region Overlay Capture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the companion's region capture action open a real display-sized selection overlay and pass a DPI-correct `CropRegion` to the existing Tauri capture command.

**Architecture:** Reuse the existing `companion` native window instead of creating a third privileged window. A small Rust session state saves and restores the companion's physical bounds while the React `RegionOverlay` covers the primary display, owns pointer selection and Escape cancellation, and scales the selected logical rectangle to screenshot pixels before IPC.

**Tech Stack:** Tauri 2, Rust, React 19, TypeScript, Vitest, Testing Library.

## Global Constraints

- Keep the configured window labels exactly `main` and `companion`.
- Region mode must never call `capture` without a non-zero `CropRegion`.
- Cancel, capture failure, and overlay setup failure must leave the companion restored and usable.
- Convert CSS-pixel selection coordinates with the native window scale factor before invoking capture.
- Use TDD and commit the completed fix separately from Task 8 handoff documentation.

---

### Task 1: Region Selection Overlay and Native Window Session

**Files:**
- Create: `src/features/floating-companion/RegionOverlay.tsx`
- Create: `src/features/floating-companion/RegionOverlay.test.tsx`
- Modify: `src/features/floating-companion/CompanionWindow.tsx`
- Modify: `src/features/floating-companion/CompanionWindow.test.tsx`
- Modify: `src/lib/desktop.ts`
- Modify: `src/lib/companion.ts`
- Modify: `src/lib/companion.test.ts`
- Modify: `src-tauri/src/commands/companion.rs`
- Modify: `src-tauri/src/commands/companion_test.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/app.css`

**Interfaces:**
- Produces: `CropRegion = { x: number; y: number; width: number; height: number }`.
- Produces: `beginCompanionRegionSelection() -> Promise<{ scaleFactor: number }>` and `finishCompanionRegionSelection() -> Promise<void>`.
- Consumes: `capture(mode: CaptureMode, region?: CropRegion)` and existing `capture(mode, region, app, repository)` IPC.

- [x] **Step 1: Write failing React and IPC wrapper tests**

Assert that a normalized drag emits `{x,y,width,height}`, Escape cancels, a click without area does not capture, the companion begins/restores the native region session, and `capture('region', region)` invokes Tauri with both fields.

- [x] **Step 2: Run the focused tests and confirm RED**

Run:

```powershell
npm test -- RegionOverlay CompanionWindow companion
```

Expected: fail because the overlay, wrapper parameter, and native session wrappers do not exist.

- [x] **Step 3: Implement the minimal React overlay and client contracts**

Render a full-window crosshair surface only during selection. Normalize reverse drags, reject zero width/height, multiply every coordinate by `scaleFactor`, round to integers, restore the native window before capture, and restore on Escape or errors.

- [x] **Step 4: Write failing Rust session tests**

Use an injected window adapter to prove begin stores the original physical bounds and covers the primary monitor, finish restores exactly once, duplicate begin is rejected, and failed setup rolls back.

- [x] **Step 5: Run the Rust tests and confirm RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml commands::companion_test -- --nocapture
```

Expected: fail because region selection state and commands do not exist.

- [x] **Step 6: Implement and register the native session commands**

Manage one mutex-protected saved physical bounds value. `begin_companion_region_selection` reads the companion's bounds, covers the primary monitor used by region capture, stores the prior bounds only after success, and returns the scale factor. `finish_companion_region_selection` restores the saved bounds and rejects calls without an active session.

- [x] **Step 7: Run focused and complete tests**

Run:

```powershell
npm test -- RegionOverlay CompanionWindow companion
cargo test --manifest-path src-tauri/Cargo.toml commands::companion_test
npm test
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all tests pass and the region action always carries a non-zero region.

- [x] **Step 8: Build, smoke, and commit**

Build the debug executable, verify selection and Escape restoration in the Windows app, then commit:

The display-sized overlay and Escape restoration were verified on Windows. Automated drag-to-capture coverage passes; manual drag-and-save confirmation remains required because desktop user input interrupted the GUI automation attempt.

```powershell
git add src src-tauri docs/superpowers/plans/2026-07-31-region-overlay-capture.md
git commit -m "fix: select a real screenshot region"
```
