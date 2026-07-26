# Core Bug Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 BUG-01 至 BUG-09，使截图、导入、月份导航和资源刷新可可靠工作。

**Architecture:** 资源 schema 通过可测试迁移升级至 v2，Rust 服务返回扩展后的 `Asset` 并由命令发出事件。前端通过共享月份选择器和资产创建事件在当前月份内局部刷新。

**Tech Stack:** Tauri 2、Rust、rusqlite、screenshots、image、React 19、TypeScript、Vitest。

## Global Constraints

- 仅支持 Windows 10/11；窗口截图只截取前台窗口。
- v1 到 v2 迁移使用 `app_meta.schema_version` 与六条 `ALTER TABLE`，不重建 assets 表。
- `asset-created` 只触发当前显示月份的重载。
- 正式构建不得注册 `create_asset` IPC 命令。

---

### Task 1: Extend the Asset Schema and Migration

**Files:**
- Modify: `src-tauri/src/domain/asset.rs`, `src-tauri/src/repository/assets.rs`, `src-tauri/src/repository/assets_test.rs`

**Interfaces:**
- Produces: `Asset { tags, deleted_at, capture_mode, annotation_data, cloud_id }` and `migrate_schema(&Connection)`.

- [ ] Write failing migration and mapping tests for a v1 in-memory schema.
- [ ] Run `cargo test repository::assets_test` and verify the migration tests fail.
- [ ] Implement versioned schema creation and v1-to-v2 migration; update row mapping and queries.
- [ ] Run `cargo test repository::assets_test` and verify it passes.
- [ ] Commit with `fix: migrate asset schema to v2`.

### Task 2: Fix Import Metadata and Capture Modes

**Files:**
- Modify: `src-tauri/src/services/import.rs`, `src-tauri/src/services/import_test.rs`, `src-tauri/src/services/capture.rs`, `src-tauri/src/services/capture_test.rs`

**Interfaces:**
- Produces: `CropRegion`, `capture(mode, region, data_directory, repository)` and import assets retaining file creation time.

- [ ] Write failing tests for source file creation time, region crop validation, and mode dispatch.
- [ ] Run `cargo test services` and verify the new tests fail.
- [ ] Implement metadata fallback, unique capture filenames, region crop, and foreground-window crop.
- [ ] Run `cargo test services` and verify it passes.
- [ ] Commit with `fix: preserve metadata and honor capture mode`.

### Task 3: Harden Commands and Publish Asset Events

**Files:**
- Modify: `src-tauri/src/commands/assets.rs`, `src-tauri/src/main.rs`, `src-tauri/src/asset_protocol_config_test.rs`

**Interfaces:**
- Produces: `capture(mode, region, app, repository)`, `import_files(...)`, and `asset-created` event payloads.

- [ ] Write failing command-boundary tests for region forwarding, import/capture event emission, and release-only IPC registration.
- [ ] Run `cargo test commands asset_protocol_config_test` and verify the tests fail.
- [ ] Emit every successful asset, guard `create_asset` by `debug_assertions`, and update handler registration.
- [ ] Run `cargo test` and verify it passes.
- [ ] Commit with `fix: constrain asset IPC and emit creation events`.

### Task 4: Share Month Navigation and Fix Companion Actions

**Files:**
- Create: `src/features/library/MonthPicker.tsx`, `src/features/library/MonthPicker.test.tsx`
- Modify: `src/features/library/MagicBookView.tsx`, `src/features/library/MagicBookView.test.tsx`, `src/features/library/ClassicGallery.tsx`, `src/features/library/ClassicGallery.test.tsx`, `src/features/floating-companion/CompanionMenu.tsx`, `src/features/floating-companion/CompanionWindow.tsx`, related tests, `src/lib/assets.ts`, `src/lib/desktop.ts`

**Interfaces:**
- Produces: `MonthPicker({ month, onSelect })`, `asset-created` subscriptions, `CompanionMenu({ onCapture, onImport })`.

- [ ] Write failing Vitest cases for year changes, gallery month navigation, companion import click, capture error alert, and current-month-only event reload.
- [ ] Run `npm test -- MonthPicker MagicBookView ClassicGallery CompanionWindow` and verify the new tests fail.
- [ ] Implement shared picker, dialog import, accessible error state, and event subscriptions with cleanup.
- [ ] Run `npm test` and verify it passes.
- [ ] Commit with `fix: complete library navigation and companion actions`.

### Task 5: Verify the Complete Fix Batch

**Files:**
- Modify: only test fixes required by verification.

- [ ] Run `npm run build`.
- [ ] Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [ ] Run `npm run tauri -- build --debug --no-bundle`.
- [ ] Inspect `git diff --check` and `git status --short`.
- [ ] Commit any verification-only correction with `test: verify core bug fixes`.
