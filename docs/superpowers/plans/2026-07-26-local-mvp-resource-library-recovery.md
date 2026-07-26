# Local MVP Resource Library Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the demo library with a query-backed local asset browser that shows imported and captured resources by month and day in both library views.

**Architecture:** The Rust repository and existing Tauri list commands remain the source of truth. A small TypeScript IPC adapter exposes typed month/day queries; library components consume that adapter through injected loaders so component tests exercise request, result, empty, and retry behavior without a live Tauri runtime.

**Tech Stack:** Tauri 2, Rust, React 19, TypeScript, Vitest, Testing Library.

## Global Constraints

- Only the local MVP is in scope; account, cloud upload, offline sync queue, retry, and conflict-copy behavior remain deferred.
- Every resource browser reads through `list_assets_by_month` or `list_assets_by_day`; no hard-coded assets or dates remain in library UI.
- A month change must replace its date flow and displayed assets; it must not retain data from the previous month.
- Image previews use Tauri `convertFileSrc`; command failures have a visible Chinese error state and a retry action.
- A single asset is centered and has no image slider; two or more assets show the slider.
- Tests must use real component behavior and hand-authored asset fixtures, not assertions about mocked DOM placeholders.

---

### Task 1: Add Typed Asset IPC Adapter

**Files:**
- Create: `src/lib/assets.ts`
- Create: `src/lib/assets.test.ts`

**Interfaces:**
- Consumes: Tauri `invoke('list_assets_by_month', { year, month })`, `invoke('list_assets_by_day', { year, month, day })`, and `convertFileSrc(path)`.
- Produces: `Asset`, `listAssetsByMonth(year, month)`, `listAssetsByDay(year, month, day)`, `assetPreviewUrl(path)`.

- [ ] **Step 1: Write failing adapter tests**

```ts
it('requests assets for the selected month', async () => {
  await listAssetsByMonth(2026, 7);
  expect(invoke).toHaveBeenCalledWith('list_assets_by_month', { year: 2026, month: 7 });
});

it('converts a stored preview path into a webview URL', () => {
  expect(assetPreviewUrl('C:/cache/preview.png')).toBe('asset://localhost/C:/cache/preview.png');
});
```

- [ ] **Step 2: Run the adapter test and verify it fails**

Run: `npm test -- src/lib/assets.test.ts`

Expected: FAIL because the adapter module does not exist.

- [ ] **Step 3: Implement the minimal adapter**

```ts
export type Asset = {
  id: string;
  createdAt: string;
  importedAt: string;
  source: 'import' | 'capture';
  originalPath: string;
  previewPath: string;
  albumId: string | null;
  favorite: boolean;
  syncVersion: number;
};

export const listAssetsByMonth = (year: number, month: number) =>
  invoke<Asset[]>('list_assets_by_month', { year, month });
```

Add the matching day query and preview URL helper using `convertFileSrc`.

- [ ] **Step 4: Run the adapter test and verify it passes**

Run: `npm test -- src/lib/assets.test.ts`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/assets.ts src/lib/assets.test.ts
git commit -m "feat: add typed asset query adapter"
```

### Task 2: Make Magic Book Query-Backed

**Files:**
- Modify: `src/features/library/MagicBookView.tsx`
- Create: `src/features/library/DateFlow.tsx`
- Create: `src/features/library/ImageStack.tsx`
- Modify: `src/features/library/MagicBookView.test.tsx`

**Interfaces:**
- Consumes: `Asset`, `listAssetsByMonth`, `listAssetsByDay` from `src/lib/assets.ts`.
- Produces: `MagicBookView({ initialMonth, loadMonth?, loadDay? })`, which renders query-derived dates and assets.

- [ ] **Step 1: Write failing component tests**

```tsx
it('replaces date flow and image cards with the selected month response', async () => {
  const loadMonth = vi.fn()
    .mockResolvedValueOnce([asset('july', '2026-07-25T12:00:00Z')])
    .mockResolvedValueOnce([asset('may', '2026-05-18T12:00:00Z')]);
  render(<MagicBookView initialMonth={{ year: 2026, month: 7 }} loadMonth={loadMonth} />);
  await userEvent.click(await screen.findByRole('button', { name: '2026 年 7 月' }));
  await userEvent.click(screen.getByRole('button', { name: '5 月' }));
  expect(await screen.findByRole('img', { name: 'may' })).toBeVisible();
  expect(screen.queryByRole('img', { name: 'july' })).not.toBeInTheDocument();
});

it('shows no slider for exactly one selected-day asset', async () => {
  render(<MagicBookView initialMonth={{ year: 2026, month: 7 }} loadMonth={() => Promise.resolve([asset('only', '2026-07-25T12:00:00Z')])} />);
  expect(await screen.findByRole('img', { name: 'only' })).toBeVisible();
  expect(screen.queryByRole('slider', { name: '图片浏览滑轨' })).not.toBeInTheDocument();
});
```

- [ ] **Step 2: Run the component tests and verify they fail**

Run: `npm test -- src/features/library/MagicBookView.test.tsx`

Expected: FAIL because the view renders fixed demo cards and accepts no query loader.

- [ ] **Step 3: Implement focused library components**

```tsx
const dates = [...new Set(monthAssets.map((asset) => asset.createdAt.slice(0, 10)))].sort().reverse();
const selectedAssets = monthAssets.filter((asset) => asset.createdAt.slice(0, 10) === selectedDate);

return <ImageStack assets={selectedAssets} />;
```

Load on month change, clear previous assets before the request resolves, and expose Chinese loading, empty, failure, and retry states. `DateFlow` renders dates from props. `ImageStack` renders `<img src={assetPreviewUrl(asset.previewPath)} alt={asset.id} />`, centers one asset, and renders its range input only for multiple assets.

- [ ] **Step 4: Run component tests and verify they pass**

Run: `npm test -- src/features/library/MagicBookView.test.tsx`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/features/library/MagicBookView.tsx src/features/library/DateFlow.tsx src/features/library/ImageStack.tsx src/features/library/MagicBookView.test.tsx
git commit -m "feat: load magic book assets from local repository"
```

### Task 3: Add Query-Backed Traditional Gallery and Integration States

**Files:**
- Create: `src/features/library/ClassicGallery.tsx`
- Create: `src/features/library/ClassicGallery.test.tsx`
- Modify: `src/App.tsx`
- Modify: `src/app.css`

**Interfaces:**
- Consumes: `listAssetsByMonth(year, month): Promise<Asset[]>`.
- Produces: `ClassicGallery({ initialMonth, loadMonth? })` and an application gallery mode that shows current-month assets.

- [ ] **Step 1: Write failing gallery tests**

```tsx
it('renders every asset returned for the initial month', async () => {
  render(<ClassicGallery initialMonth={{ year: 2026, month: 7 }} loadMonth={() => Promise.resolve([
    asset('sunset', '2026-07-25T12:00:00Z'),
    asset('forest', '2026-07-24T12:00:00Z'),
  ])} />);
  expect(await screen.findByRole('img', { name: 'sunset' })).toBeVisible();
  expect(screen.getByRole('img', { name: 'forest' })).toBeVisible();
});

it('renders a retry action after a month query fails', async () => {
  const loadMonth = vi.fn().mockRejectedValueOnce(new Error('offline')).mockResolvedValueOnce([]);
  render(<ClassicGallery initialMonth={{ year: 2026, month: 7 }} loadMonth={loadMonth} />);
  await screen.findByText('无法加载本月图片');
  await userEvent.click(screen.getByRole('button', { name: '重试' }));
  expect(loadMonth).toHaveBeenCalledTimes(2);
});
```

- [ ] **Step 2: Run gallery tests and verify they fail**

Run: `npm test -- src/features/library/ClassicGallery.test.tsx`

Expected: FAIL because `ClassicGallery` does not exist.

- [ ] **Step 3: Implement the gallery and integrate it**

```tsx
{status === 'error' && <button type="button" onClick={load}>重试</button>}
{status === 'ready' && assets.map((asset) => <img key={asset.id} src={assetPreviewUrl(asset.previewPath)} alt={asset.id} />)}
```

Replace the gallery placeholder in `App`, pass `{ year: 2026, month: 7 }`, and add grid/empty/error styles without changing the companion behavior.

- [ ] **Step 4: Run gallery and full frontend tests**

Run: `npm test`

Expected: PASS.

- [ ] **Step 5: Run production and Rust verification**

Run: `npm run build && cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/features/library/ClassicGallery.tsx src/features/library/ClassicGallery.test.tsx src/App.tsx src/app.css
git commit -m "feat: display local assets in classic gallery"
```

## Self-Review

- Spec coverage: Tasks 1–3 cover typed Tauri queries, date-derived magic-book data replacement, preview rendering, single/multiple asset states, query-backed gallery, empty/error/retry UX, and production verification.
- Placeholder scan: the plan contains no deferred implementation instructions; all commands, files, interfaces, and expected test outcomes are concrete.
- Type consistency: all components consume the `Asset` and loader signatures defined in Task 1; month props use `{ year: number; month: number }` throughout.
