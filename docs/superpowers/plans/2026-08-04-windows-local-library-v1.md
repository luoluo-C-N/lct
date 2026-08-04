# 魔法影像库 Windows 单机可用版 1.0 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把现有原型收敛为可交付的 Windows 单机图片资料库，完成导入/截图、分页查找、详情、收藏、标签、回收站和安全永久删除闭环。

**Architecture:** 保留现有 Tauri 2、React 19、SQLite、截图和悬浮助手实现。Rust Repository 负责 schema 与查询，资产 Service 协调文件系统补偿，命令层负责 IPC 与尽力事件；前端用局部查询 Hook、游标分页和虚拟网格组织图库，图库与魔法书复用详情面板。

**Tech Stack:** Rust 2021, Tauri 2, rusqlite 0.37, React 19, TypeScript 5.6, Vite 5, Vitest 2, Testing Library, `@tanstack/react-virtual`, WebdriverIO, `tauri-driver`, EdgeDriver, GitHub Actions Windows runner.

## Global Constraints

- 目标平台为 Windows 10/11，单机、单用户、无账号和云同步。
- 标签只存储于 `tags` 与 `asset_tags` 关系表，禁止给 `assets` 增加 JSON 标签列。
- schema 只能 v3 → v4 原地迁移，不重建 `assets`。
- 图库每页默认 60 条、最大 120 条，禁止一次性加载全部资产。
- 普通图库只显示 `deleted_at IS NULL`，回收站只显示 `deleted_at IS NOT NULL`。
- 永久删除只能触碰 `$APPLOCALDATA/assets/originals`、`previews` 和 `.deleting`。
- 已提交的数据库/文件操作不因事件发送失败而改报失败。
- 现有导入、三种截图、魔法书和悬浮助手能力必须保持。
- 所有生产代码改动遵循 RED → GREEN → REFACTOR，并在每个任务后独立提交。

---

### Task 1: 完整 Asset 契约与 schema v4

**Files:**
- Modify: `src-tauri/src/domain/asset.rs`
- Modify: `src-tauri/src/domain/asset_test.rs`
- Modify: `src-tauri/src/repository/assets.rs`
- Modify: `src-tauri/src/repository/assets_test.rs`
- Modify: `src-tauri/src/services/import.rs`
- Modify: `src-tauri/src/services/import_test.rs`
- Modify: `src-tauri/src/services/capture.rs`
- Modify: `src-tauri/src/services/capture_test.rs`
- Modify: `src/lib/assets.ts`
- Modify: `src/lib/assets.test.ts`
- Modify: frontend Asset fixtures in `src/**/*.test.tsx`

**Interfaces:**
- Produces: `Asset.display_name: String` serialized as `displayName`.
- Produces: full TypeScript `Asset` contract including tags, deletion, capture, annotation and cloud fields.
- Produces: schema version `4` with `display_name` and four query indexes.
- Preserves: relationship-table tags and current import/capture persistence.

- [x] **Step 1: Write failing domain and migration tests**

Extend the Asset serialization fixture with literal `display_name: "sunset.png"` and expect `displayName`. Add an in-memory v3 fixture and assert opening the repository:

```rust
assert_eq!(schema_version(&connection), "4");
let asset = repository.get_by_id("asset-old")?.unwrap();
assert_eq!(asset.display_name, "old-photo.png");
assert_eq!(asset.tags, vec!["travel"]);
```

Assert all four indexes exist through `sqlite_master`, and a second open is idempotent.

- [x] **Step 2: Run focused tests and verify RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_test -- --nocapture
```

Expected: compile failure because `display_name` and schema v4 are absent.

- [x] **Step 3: Implement schema v4 and domain mapping**

Add `display_name` to `Asset`, `StoredAsset`, INSERT/SELECT mapping and test fixtures. Make the v3 migration execute the exact v4 ALTER/index statements in one transaction, backfill empty names from `Path::file_name()` with asset ID fallback, then update `app_meta`. Add `get_by_id(&self, id: &str) -> Result<Option<Asset>, AssetRepositoryError>` for later tasks.

- [x] **Step 4: Write failing import/capture naming tests**

Assert import preserves source filename exactly and each capture mode produces a non-empty localized display name ending in `.png`, independent of the random managed path.

- [x] **Step 5: Verify RED, implement names, and align TypeScript**

Run focused service tests, implement `import_display_name` and `capture_display_name`, then update `src/lib/assets.ts` and every TypeScript Asset fixture with the full contract:

```ts
displayName: 'sunset.png',
tags: [],
deletedAt: null,
captureMode: null,
annotationData: null,
cloudId: null,
```

- [x] **Step 6: Verify Task 1 and commit**

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
npm test
git add src-tauri/src/domain src-tauri/src/repository src-tauri/src/services src/lib/assets.ts src/lib/assets.test.ts src
git commit -m "feat: align asset contract and schema v4"
```

---

### Task 2: 统一游标查询与资产详情后端

**Files:**
- Modify: `src-tauri/src/domain/asset.rs`
- Modify: `src-tauri/src/repository/assets.rs`
- Modify: `src-tauri/src/repository/assets_test.rs`
- Modify: `src-tauri/src/commands/assets.rs`
- Modify: `src-tauri/src/asset_protocol_config_test.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src/lib/assets.ts`
- Modify: `src/lib/assets.test.ts`

**Interfaces:**
- Produces: `AssetQuery`, `AssetCursor`, `AssetPage`, `AssetDetails`.
- Produces: `query_assets(query)` and `get_asset_details(id)` IPC commands.
- Consumes: Task 1 `Asset.display_name` and `AssetRepository::get_by_id`.

- [ ] **Step 1: Write failing query behavior tests**

Seed literal assets and tags, then test:

- active vs deleted isolation;
- year/month boundary;
- display-name wildcard escaping and case-insensitive match;
- source and favorite filters;
- two-tag AND matching;
- stable `(sort_timestamp, id)` pagination with identical timestamps;
- default 60 and clamped maximum 120;
- trash ordering by deletion timestamp.

- [ ] **Step 2: Run repository tests and verify RED**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml repository::assets_test::query -- --nocapture
```

Expected: missing query types and repository method.

- [ ] **Step 3: Implement normalized query and keyset SQL**

Define serde camelCase types. Normalize text and tags before SQL. Build the query with `rusqlite::params_from_iter`, `EXISTS` per tag, escaped `LIKE ... ESCAPE '\'`, and keyset predicate. Fetch `limit + 1`, remove the extra row, and derive `next_cursor` from the final returned item.

- [ ] **Step 4: Write failing IPC and details tests**

Assert literal invoke payloads in TypeScript and command registration in Rust. Create a managed image and assert details contain dimensions, size and existence flags; a missing original returns null dimensions/size without panicking. Assert asset protocol exposes originals only under `$APPLOCALDATA/assets/originals/**/*`.

- [ ] **Step 5: Implement commands and TypeScript wrappers**

Register:

```rust
commands::assets::query_assets,
commands::assets::get_asset_details,
```

Add `queryAssets(query)`, `getAssetDetails(id)` and `subscribeToAssetChanged` wrappers. Keep old month/day wrappers.

- [ ] **Step 6: Verify Task 2 and commit**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npm test -- assets
npm run build
git add src-tauri src/lib/assets.ts src/lib/assets.test.ts
git commit -m "feat: add paged asset queries"
```

---

### Task 3: 资产写操作、统一事件和安全删除

**Files:**
- Create: `src-tauri/src/services/assets.rs`
- Create: `src-tauri/src/services/assets_test.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/repository/assets.rs`
- Modify: `src-tauri/src/repository/assets_test.rs`
- Modify: `src-tauri/src/commands/assets.rs`
- Create: `src-tauri/src/commands/assets_test.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/services/import.rs`
- Modify: `src-tauri/src/services/capture.rs`
- Modify: `src-tauri/src/commands/companion.rs`
- Modify: `src/lib/assets.ts`
- Modify: `src/lib/assets.test.ts`

**Interfaces:**
- Produces: favorite, tags, soft-delete, restore and permanent-delete commands.
- Produces: `AssetChangedEvent` and best-effort `asset-changed` emission.
- Produces: crash-safe `.deleting/<id>/manifest.json` staging and startup recovery.

- [ ] **Step 1: Write failing repository mutation tests**

Assert favorite and normalized tags return the full updated asset. Test empty removal, case-insensitive dedupe, 20-tag and 40-character limits, missing asset errors, soft delete idempotency and restore. Permanent delete must reject an active asset at the service boundary.

- [ ] **Step 2: Run focused tests and verify RED**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml repository::assets_test -- --nocapture
```

- [ ] **Step 3: Implement transactional repository mutations**

Add explicit `AssetNotFound`, `InvalidTags` and `AssetNotDeleted` errors. Use one transaction for relationship-tag replacement. Return `get_by_id` after commit. Add record-only delete helpers restricted to services.

- [ ] **Step 4: Write failing deletion compensation tests**

Use temporary managed roots and injected filesystem operations to prove:

- paths outside originals/previews are rejected without mutation;
- files are staged with a valid manifest before the database record is removed;
- database failure restores both files;
- successful delete removes record, tag relations and staging directory;
- startup recovery restores files when the row remains and removes staging when the row is gone;
- invalid manifests are preserved.

- [ ] **Step 5: Implement `services::assets`**

Define `DeletionManifest`, `permanently_delete`, `recover_pending_deletions` and a production filesystem adapter. Call recovery during Tauri setup after repositories are registered.

- [ ] **Step 6: Write failing command/event tests**

Assert successful mutations emit one literal `asset-changed` payload, committed commands still return success when the test emitter fails, and no event occurs before persistence. Assert import/capture creation emits `kind: created`.

- [ ] **Step 7: Implement commands and best-effort event helper**

Add `emit_asset_changed_best_effort` that logs emission failures with `eprintln!` and never changes the committed result. Migrate import, capture and frozen-region completion to the unified event while temporarily retaining `asset-created` until frontend Task 4.

- [ ] **Step 8: Verify Task 3 and commit**

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
npm test -- assets
git add src-tauri src/lib/assets.ts src/lib/assets.test.ts
git commit -m "feat: manage asset lifecycle safely"
```

---

### Task 4: 默认图库、查询 Hook 和虚拟网格

**Files:**
- Modify: `package.json`
- Modify: `package-lock.json`
- Create: `src/hooks/useAssetQuery.ts`
- Create: `src/hooks/useAssetQuery.test.ts`
- Create: `src/features/library/LibraryWorkspace.tsx`
- Create: `src/features/library/LibraryWorkspace.test.tsx`
- Create: `src/features/library/LibraryToolbar.tsx`
- Create: `src/features/library/LibraryToolbar.test.tsx`
- Create: `src/features/library/AssetGrid.tsx`
- Create: `src/features/library/AssetGrid.test.tsx`
- Modify: `src/App.tsx`
- Modify: `src/App.test.tsx`
- Modify: `src/app.css`
- Modify: `src/features/library/MagicBookView.tsx`
- Modify: `src/features/library/ClassicGallery.tsx`

**Interfaces:**
- Produces: `useAssetQuery(initialQuery)` with `items`, `status`, `loadMore`, `refresh`, `hasMore`, `error`.
- Produces: virtualized `AssetGrid`.
- Consumes: Task 2 query IPC and Task 3 asset-changed events.

- [ ] **Step 1: Install virtualization dependency**

```powershell
npm install @tanstack/react-virtual
```

This dependency is used only for viewport virtualization; no global state library is added.

- [ ] **Step 2: Write failing Hook tests**

Use deferred promises to assert first-page loading, append without duplicates, load-more retry, 250ms text debounce, immediate chip/month query, stale response discard, and event-driven refresh only when a changed asset could affect the active query.

- [ ] **Step 3: Verify RED and implement `useAssetQuery`**

```powershell
npm test -- useAssetQuery
```

Keep request version refs, page cursor state and unlisten cleanup inside the Hook. Command responses update local data directly; events synchronize other windows/views.

- [ ] **Step 4: Write failing toolbar/grid tests**

Assert accessible controls for month, search, source, tags, favorite, import and screenshot. Seed 200 assets and assert the DOM renders only a bounded virtual window, uses `loading="lazy"`, and calls load-more near the end.

- [ ] **Step 5: Implement toolbar, virtual grid and workspace**

Use stable row height/aspect ratio and `@tanstack/react-virtual`. Give tests a literal scroll viewport rectangle and ResizeObserver fixture so jsdom virtualization is deterministic. Preserve filters and scroll while selecting a card. Reuse existing import and screenshot IPC wrappers from main-window controls.

- [ ] **Step 6: Write failing navigation tests and make gallery default**

Assert initial render is `图库`, top-level entries are 图库/魔法书/回收站, and 皮肤库 is absent from primary navigation. Implement App routing state. Replace `ClassicGallery` with `LibraryWorkspace`, delete `ClassicGallery.tsx`, and migrate or delete `ClassicGallery.test.tsx` assertions after equivalent query/navigation coverage exists in the new workspace tests.

- [ ] **Step 7: Migrate asset-created listeners**

Update MagicBook and the new query Hook to listen to `asset-changed`. Remove `asset-created` emission and wrapper after all frontend consumers migrate; keep creation refresh month-aware in MagicBook.

- [ ] **Step 8: Verify Task 4 and commit**

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
git add package.json package-lock.json src src-tauri
git commit -m "feat: make the paged gallery the default"
```

---

### Task 5: 共享资产详情、收藏和标签编辑

**Files:**
- Create: `src/features/assets/AssetDetailsPanel.tsx`
- Create: `src/features/assets/AssetDetailsPanel.test.tsx`
- Create: `src/features/assets/AssetTagsEditor.tsx`
- Create: `src/features/assets/AssetTagsEditor.test.tsx`
- Modify: `src/features/library/LibraryWorkspace.tsx`
- Modify: `src/features/library/LibraryWorkspace.test.tsx`
- Modify: `src/features/library/ImageStack.tsx`
- Modify: `src/features/library/MagicBookView.tsx`
- Modify: `src/features/library/MagicBookView.test.tsx`
- Modify: `src/app.css`

**Interfaces:**
- Produces: one `AssetDetailsPanel` used by gallery and MagicBook.
- Consumes: Task 2 details and Task 3 mutation commands.

- [ ] **Step 1: Write failing details tests**

Assert preview/details loading, original fallback, metadata rendering, screenshot mode, close focus restoration, and missing-file state. Mutation tests must prove optimistic busy state, command-response replacement, error rollback and duplicate-submit prevention.

- [ ] **Step 2: Run tests and verify RED**

```powershell
npm test -- AssetDetailsPanel
```

- [ ] **Step 3: Implement details panel**

Use an unframed right-side panel, 380px desktop width and full-width mobile fallback. Use semantic buttons, checkbox/switch for favorite, and clear close/delete commands. Do not put a card inside the panel.

- [ ] **Step 4: Write failing tag editor tests**

Assert Enter/comma addition, trimming, case-insensitive dedupe, Backspace/remove, 20/40 validation, cancel, save payload and server-error recovery.

- [ ] **Step 5: Implement tags and integrate both views**

Keep validation mirrored for immediate feedback while Rust remains authoritative. Update `ImageStack` card click to select an asset; mount the same panel from `MagicBookView`.

- [ ] **Step 6: Verify Task 5 and commit**

```powershell
npm test
npm run build
git add src/features src/app.css
git commit -m "feat: add shared asset details and editing"
```

---

### Task 6: 回收站、永久删除确认和设置收敛

**Files:**
- Create: `src/features/trash/TrashView.tsx`
- Create: `src/features/trash/TrashView.test.tsx`
- Create: `src/features/trash/PermanentDeleteDialog.tsx`
- Create: `src/features/trash/PermanentDeleteDialog.test.tsx`
- Create: `src/features/settings/SettingsView.tsx`
- Create: `src/features/settings/SettingsView.test.tsx`
- Modify: `src/features/assets/AssetDetailsPanel.tsx`
- Modify: `src/features/assets/AssetDetailsPanel.test.tsx`
- Modify: `src/App.tsx`
- Modify: `src/App.test.tsx`
- Modify: `src/app.css`

**Interfaces:**
- Produces: paged recycle bin and explicit permanent-delete confirmation.
- Produces: secondary settings destination containing companion controls and existing SkinLibrary.

- [ ] **Step 1: Write failing trash tests**

Assert deleted query, pagination, restore command-response removal, external restore event removal, permanent-delete dialog, exact confirmation, busy lock, failure retention and retry.

- [ ] **Step 2: Verify RED and implement TrashView**

```powershell
npm test -- TrashView PermanentDeleteDialog
```

The dialog text must state that original and preview files cannot be recovered. Escape/cancel never deletes.

- [ ] **Step 3: Add soft-delete from details**

Write a failing panel test, call `softDeleteAsset`, close the panel only after success, and show an inline error without hiding the asset when the command fails.

- [ ] **Step 4: Write failing settings/navigation tests**

Assert Settings is a secondary button, SkinLibrary is rendered inside Settings, companion show/hide remains functional, and returning to Gallery reconstructs the default query without stale detail selection.

- [ ] **Step 5: Implement SettingsView and navigation**

Move existing companion visibility UI out of the main header into Settings. Render SkinLibrary below assistant settings. Do not change skin backends.

- [ ] **Step 6: Verify Task 6 and commit**

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
git add src
git commit -m "feat: complete trash and settings flows"
```

---

### Task 7: 性能门槛、CI、真实桌面 E2E 和 1.0 交接

**Files:**
- Modify: `src-tauri/src/repository/assets_test.rs`
- Create: `wdio.conf.ts`
- Create: `e2e/local-library.e2e.ts`
- Create: `e2e/fixtures/` PNG/JPEG/WebP files
- Modify: `package.json`
- Modify: `package-lock.json`
- Create: `.github/workflows/windows-ci.yml`
- Create: `README.md`
- Create: `docs/WINDOWS_SMOKE_TEST.md`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/asset_protocol_config_test.rs`
- Modify: `docs/CONTINUATION.md`
- Modify: `docs/superpowers/plans/2026-08-04-windows-local-library-v1.md`
- Refresh: `handoff/magic-image-library.bundle`

**Interfaces:**
- Produces: measurable 10,000-asset performance gate.
- Produces: Windows CI and E2E commands.
- Produces: install/build/user documentation and synchronized handoff.

- [ ] **Step 1: Add deterministic performance test**

Seed 10,000 assets, 500 tags and 30,000 relations in a transaction. Warm once, then assert a common month first page is under 300ms and a combined search first page under 500ms. Use `EXPLAIN QUERY PLAN` assertions that common queries use the v4 indexes; print timings on failure.

- [ ] **Step 2: Run performance test and optimize only measured regressions**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml repository::assets_test::queries_ten_thousand_assets_within_budget -- --nocapture
```

If the test fails, inspect the query plan before changing indexes or SQL. No speculative cache layer.

- [ ] **Step 3: Add WebdriverIO/Tauri Driver desktop E2E harness**

```powershell
npm install --save-dev @wdio/cli @wdio/local-runner @wdio/mocha-framework @wdio/spec-reporter webdriverio
cargo install tauri-driver --locked
```

Configure `tauri-driver` with matching Microsoft Edge WebDriver to launch the actual debug Tauri executable. Use a task-specific temporary app data directory and debug-only controlled fixture import/capture hooks. Cover import fixture, region-capture IPC boundary, search/favorite/tags, trash/restore/permanent delete and process-restart persistence. Never register fixture hooks in release builds.

- [ ] **Step 4: Add Windows CI**

Run npm install, Vitest, Rust tests, fmt check, frontend build and `tauri build --debug --no-bundle` on `windows-latest`. Cache Cargo and npm directories using lockfile hashes. E2E runs in a separate Windows job that installs `tauri-driver`, resolves the installed Edge version to a matching EdgeDriver, and uploads driver/application logs plus screenshots on failure.

- [ ] **Step 5: Correct release configuration and documentation**

Change identifier to `com.magicimagelibrary.desktop`, enable Windows bundle targets for release candidates, and extend config tests. README must document purpose, supported formats, install/dev/test commands, app data location, backup recommendation, unsigned-build warning and known limitations.

- [ ] **Step 6: Run full verification matrix**

```powershell
npm test
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm run build
npm run tauri -- build --debug --no-bundle
npm run test:e2e
git diff --check
```

All commands must exit 0. Record exact frontend, Rust and E2E counts.

- [ ] **Step 7: Perform Windows smoke checklist**

Use the built application to verify the complete user loop, transparent companion, region drag, foreground-window/fullscreen capture, restart persistence, 10,000-item navigation, trash compensation failure message and installer launch. Record any item not completed; do not infer manual success from automated tests.

- [ ] **Step 8: Update handoff, commit, refresh and synchronize**

Mark plan checkboxes, update `docs/CONTINUATION.md`, commit documentation, run `scripts/refresh-handoff.ps1`, push the branch, and verify local HEAD, remote branch, bundle branch and `handoff/manifest.json` resolve to the same commit.
