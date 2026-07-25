# 魔法影像库 MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建可在 Windows 上运行的图片资料库 MVP，包含魔法书浏览、图片导入、本地索引、截图、悬浮入口与为云同步预留的资源接口。

**Architecture:** 使用 Tauri 2 承载 Windows 原生能力与 Rust 后端，React/TypeScript 提供资料库和悬浮角色界面。资源先写入本地应用数据目录与 SQLite 索引；所有写入通过 `AssetRepository`，使后续同步客户端可复用同一资源模型。

**Tech Stack:** Tauri 2、Rust、React 19、TypeScript、Vite、SQLite（rusqlite）、Vitest、Playwright。

## Global Constraints

- 首版仅支持 Windows 10/11；手机端、滚动长截图、第三方云盘和多人共享不在范围内。
- 所有图片资源以本地优先方式写入；网络故障不得阻塞截图或导入。
- 日期栏显示月/日与星期，不显示“今日”。
- 图片默认收束叠放，鼠标悬停才突出；单图居中且不显示滑轨。
- 悬浮角色、截图服务和资料库通过明确接口通信，不直接读写彼此 UI 状态。

---

## File Structure

- `src-tauri/src/domain/asset.rs`: 资源、相册、标签和截图元数据类型。
- `src-tauri/src/repository/assets.rs`: SQLite 资源持久化与查询接口。
- `src-tauri/src/services/capture.rs`: 区域、窗口、全屏截图命令。
- `src-tauri/src/commands/assets.rs`: 前端可调用的资源命令。
- `src/lib/assets.ts`: 前端 `AssetRepository` 调用封装。
- `src/features/library/`: 魔法书、传统图库、月份与日期栏组件。
- `src/features/floating-companion/`: 悬浮角色窗口与命令菜单。
- `src/features/import/`: 图片导入和轻量标注流程。
- `tests/`: Rust 单元测试、Vitest 组件测试和 Playwright 窗口级测试。

### Task 1: Scaffold Windows Desktop Shell

**Files:**
- Create: `package.json`, `vite.config.ts`, `src/main.tsx`, `src/App.tsx`
- Create: `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, `src-tauri/tauri.conf.json`
- Test: `src/App.test.tsx`

**Interfaces:**
- Produces: `App` React component and `tauri dev` Windows shell.

- [ ] **Step 1: Write the failing component test**

```tsx
import { render, screen } from '@testing-library/react';
import App from './App';

it('renders the library shell', () => {
  render(<App />);
  expect(screen.getByRole('main', { name: '影像资料库' })).toBeVisible();
});
```

- [ ] **Step 2: Run the test and verify it fails**

Run: `npm test -- App.test.tsx`
Expected: FAIL because `App` does not exist.

- [ ] **Step 3: Implement the minimal shell**

```tsx
export default function App() {
  return <main aria-label="影像资料库" />;
}
```

- [ ] **Step 4: Verify and commit**

Run: `npm test -- App.test.tsx && npm run tauri dev`
Expected: test passes and a Windows shell opens.

Commit: `feat: scaffold magic image library desktop shell`

### Task 2: Implement Local Asset Repository

**Files:**
- Create: `src-tauri/src/domain/asset.rs`, `src-tauri/src/repository/assets.rs`
- Create: `src-tauri/src/commands/assets.rs`
- Test: `src-tauri/src/repository/assets_test.rs`

**Interfaces:**
- Produces: `AssetRepository::create`, `list_by_month`, `list_by_day`, `set_tags`.

- [ ] **Step 1: Write failing Rust test**

```rust
#[test]
fn lists_only_assets_from_requested_month() {
    let repo = test_repository();
    repo.create(new_asset("2026-07-25T12:00:00Z")).unwrap();
    repo.create(new_asset("2026-08-01T12:00:00Z")).unwrap();
    assert_eq!(repo.list_by_month(2026, 7).unwrap().len(), 1);
}
```

- [ ] **Step 2: Run failing test**

Run: `cargo test repository::assets_test::lists_only_assets_from_requested_month`
Expected: FAIL because repository types are absent.

- [ ] **Step 3: Implement SQLite schema and repository**

Define `Asset { id, created_at, imported_at, source, original_path, preview_path, album_id, favorite, sync_version }`; add indexed `assets(created_at)` table and parameterized month query.

- [ ] **Step 4: Verify and commit**

Run: `cargo test`
Expected: PASS.

Commit: `feat: add local asset repository`

### Task 3: Add Import and Screenshot Services

**Files:**
- Create: `src-tauri/src/services/import.rs`, `src-tauri/src/services/capture.rs`
- Modify: `src-tauri/src/commands/assets.rs`
- Test: `src-tauri/src/services/import_test.rs`

**Interfaces:**
- Consumes: `AssetRepository::create`.
- Produces: `import_files(paths)` and `capture(mode)` Tauri commands returning `Asset`.

- [ ] **Step 1: Write failing import test**

```rust
#[test]
fn import_copies_file_and_creates_asset() {
    let asset = import_test_png();
    assert!(asset.original_path.exists());
    assert_eq!(asset.source, AssetSource::Import);
}
```

- [ ] **Step 2: Run failing test**

Run: `cargo test services::import_test::import_copies_file_and_creates_asset`
Expected: FAIL because import service is absent.

- [ ] **Step 3: Implement import and capture**

Copy user-selected files into an application-owned directory, generate previews, then persist the asset. Implement `CaptureMode::{Region,Window,Fullscreen}` behind the same persistence path.

- [ ] **Step 4: Verify and commit**

Run: `cargo test`
Expected: PASS.

Commit: `feat: add import and screenshot services`

### Task 4: Build Responsive Library Views

**Files:**
- Create: `src/features/library/MagicBookView.tsx`, `DateFlow.tsx`, `ImageStack.tsx`, `MonthPicker.tsx`, `ClassicGallery.tsx`
- Create: `src/lib/assets.ts`
- Test: `src/features/library/MagicBookView.test.tsx`

**Interfaces:**
- Consumes: `listAssetsByMonth(year, month): Promise<Asset[]>`.
- Produces: month switch, date switch, hover image emphasis, scroll/drag image navigation and traditional-gallery toggle.

- [ ] **Step 1: Write failing UI test**

```tsx
it('replaces the date flow after selecting a month', async () => {
  render(<MagicBookView initialMonth={{ year: 2026, month: 7 }} />);
  await userEvent.click(screen.getByRole('button', { name: '2026 年 7 月' }));
  await userEvent.click(screen.getByRole('button', { name: '5 月' }));
  expect(await screen.findByText('05 / 18')).toBeVisible();
});
```

- [ ] **Step 2: Run failing test**

Run: `npm test -- MagicBookView.test.tsx`
Expected: FAIL because components are absent.

- [ ] **Step 3: Implement the views**

Keep the selected date fixed in the left rail's visual middle; map wheel/click to date selection. Render compact image cards by default, show the slider only for more than one asset, and expand only the hovered card. Month selection performs close, data replacement, then open.

- [ ] **Step 4: Verify and commit**

Run: `npm test -- MagicBookView.test.tsx`
Expected: PASS.

Commit: `feat: add magic book library view`

### Task 5: Add Floating Companion and End-to-End Coverage

**Files:**
- Create: `src/features/floating-companion/CompanionWindow.tsx`, `CompanionMenu.tsx`
- Modify: `src-tauri/tauri.conf.json`, `src/App.tsx`
- Create: `e2e/library.spec.ts`

**Interfaces:**
- Consumes: `capture(mode)` and `import_files(paths)` commands.
- Produces: draggable always-on-top companion window with region/window/fullscreen capture actions.

- [ ] **Step 1: Write failing Playwright test**

```ts
test('companion opens capture actions', async ({ page }) => {
  await page.getByRole('button', { name: '悬浮角色' }).click();
  await expect(page.getByRole('menuitem', { name: '区域截图' })).toBeVisible();
});
```

- [ ] **Step 2: Run failing test**

Run: `npx playwright test e2e/library.spec.ts`
Expected: FAIL because the companion is absent.

- [ ] **Step 3: Implement the companion**

Create an always-on-top draggable child window with independent animation state; mouse proximity expands to avatar and exposes screenshot/import commands.

- [ ] **Step 4: Verify and commit**

Run: `npm test && cargo test && npx playwright test`
Expected: PASS.

Commit: `feat: add floating companion workflow`

## Follow-up Plan: Account and Cloud Sync

Create a separate plan after the local MVP is accepted. It will add account registration/login, object storage upload queues, offline retry, sync versions and conflict-copy behavior without changing the `AssetRepository` consumer contract.

## Self-Review

- Spec coverage: Tasks 1-5 cover Windows shell, local storage, import/screenshot, both library modes, month/date/image interactions, responsive behavior, and floating companion. Cloud auth/sync is deliberately separated because it is an independent deployable backend subsystem.
- Placeholder scan: no incomplete implementation markers.
- Interface consistency: all resource creation routes through `AssetRepository`; the UI consumes the command wrapper only.
