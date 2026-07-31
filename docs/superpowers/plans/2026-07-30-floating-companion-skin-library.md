# Floating Companion Skin Library Implementation Plan

> **Pause checkpoint (2026-07-31):** Task 1 and Task 2 are complete and committed through `5d59ce0`. Task 3 has its exact `zip = 8.6.0` dependency added and locked, plus an unfinished RED test draft in the companion domain/repository and skin service tests. The draft currently does not compile because its fixture helpers and production contracts are not implemented. Resume inside Task 3 Step 1; do not treat the dependency or draft tests as a completed step.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a real always-on-top Tauri companion window, three built-in animated skins, and a safe local PNG/WebP/ZIP skin library.

**Architecture:** Tauri owns the two-window lifecycle, SQLite skin/settings state, file validation, and cross-window events. The existing React build selects `main` or `companion` entry by window label; the main window owns full skin management while the companion owns quick actions and quick switching. Imported files are normalized into `$APPLOCALDATA/skins/<skin-id>/` and exposed only through the scoped asset protocol.

**Tech Stack:** Tauri 2, Rust 1.97, rusqlite, image with PNG/WebP, zip 8.6.0, React 19, TypeScript, Vite, Vitest, Testing Library.

## Global Constraints

- Work only on `feat/magic-image-library`; preserve unrelated user changes.
- Use TDD for every behavior change and commit each completed task.
- Companion labels are exactly `main` and `companion`; window commands reject any other label.
- Collapsed companion content size is `72 × 72`; expanded target is approximately `232 × 320`.
- The companion is transparent, undecorated, non-resizable, always on top, skipped from the taskbar, and visible at startup.
- Closing the companion hides it; closing the main window exits the application.
- Skin files live under `$APPLOCALDATA/skins/<skin-id>/`; the asset protocol exposes only previews plus that skin tree.
- Image imports accept PNG/WebP up to `10 MiB`, decoded dimensions `128..=4096` on both axes.
- ZIP imports accept at most `20 MiB` compressed, `40 MiB` uncompressed, `16` entries, and a `64 KiB` root `manifest.json`.
- ZIP packages contain only `manifest.json` and referenced PNG/WebP files; reject absolute paths, parent traversal, symlinks, unknown files, executable/web content, and remote URLs.
- Built-in skins are `quiet-aurora`, `porcelain-pearl`, and `deep-ink`; they cannot be overwritten or deleted.
- Motion respects the user switch and `prefers-reduced-motion: reduce`.
- Public community upload, direct external references, and executable skin formats remain documented future work only.

---

### Task 1: Skin Domain, Schema v3, and Persistent Settings

**Files:**
- Create: `src-tauri/src/domain/companion.rs`
- Create: `src-tauri/src/domain/companion_test.rs`
- Create: `src-tauri/src/repository/companion.rs`
- Create: `src-tauri/src/repository/companion_test.rs`
- Modify: `src-tauri/src/domain/mod.rs`
- Modify: `src-tauri/src/repository/mod.rs`
- Modify: `src-tauri/src/repository/assets.rs`
- Modify: `src-tauri/src/repository/assets_test.rs`

**Interfaces:**
- Produces: `CompanionSkin`, `SkinSource`, `VisualPreset`, `CompanionSettings`, `WindowPlacement`, `CompanionRepository`.
- Produces repository methods:
  - `list_skins() -> Result<Vec<CompanionSkin>, CompanionRepositoryError>`
  - `get_settings() -> Result<CompanionSettings, CompanionRepositoryError>`
  - `create_skin(&CompanionSkin)`, `update_skin(&CompanionSkin)`, `delete_skin(&str)`
  - `set_active_skin(&str)`, `save_settings(&CompanionSettings)`
- Migrates existing SQLite schema from v2 to v3 without rebuilding or losing assets.

- [x] **Step 1: Write failing domain serialization and clamp tests**

```rust
#[test]
fn serializes_companion_skin_for_the_frontend() {
    let skin = CompanionSkin::builtin(VisualPreset::QuietAurora);
    assert_eq!(serde_json::to_value(skin).unwrap()["id"], "quiet-aurora");
    assert_eq!(serde_json::to_value(skin).unwrap()["visualPreset"], "quiet_aurora");
}

#[test]
fn clamps_motion_settings_to_safe_ranges() {
    let settings = MotionSettings::new(9.0, -2.0);
    assert_eq!(settings.flow_speed, 2.0);
    assert_eq!(settings.flow_intensity, 0.0);
}
```

- [x] **Step 2: Run the domain tests and confirm RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml domain::companion_test -- --nocapture
```

Expected: compile failure because `domain::companion` and the types do not exist.

- [x] **Step 3: Define the domain types and three complete presets**

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionSkin {
    pub id: String,
    pub name: String,
    pub source: SkinSource,
    pub visual_preset: VisualPreset,
    pub texture_path: Option<PathBuf>,
    pub preview_path: Option<PathBuf>,
    pub flow_colors: Vec<String>,
    pub flow_speed: f32,
    pub flow_intensity: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanionSettings {
    pub active_skin_id: String,
    pub motion_enabled: bool,
    pub visible: bool,
    pub placement: Option<WindowPlacement>,
}
```

Define all values for `QuietAurora`, `PorcelainPearl`, and `DeepInk`; do not implement a hue-rotation shortcut.

- [x] **Step 4: Write failing v2-to-v3 migration and repository tests**

```rust
#[test]
fn migrates_v2_and_seeds_builtin_skins_without_losing_assets() {
    let connection = v2_connection_with_one_asset();
    let repository = CompanionRepository::from_connection(connection).unwrap();
    assert_eq!(repository.list_skins().unwrap().len(), 3);
    assert_eq!(repository.get_settings().unwrap().active_skin_id, "quiet-aurora");
}

#[test]
fn protects_builtin_and_active_skins() {
    let repository = in_memory_repository();
    assert!(matches!(
        repository.delete_skin("quiet-aurora"),
        Err(CompanionRepositoryError::BuiltinSkin)
    ));
}
```

- [x] **Step 5: Run repository tests and confirm RED**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml repository::companion_test -- --nocapture
```

Expected: compile failure because `CompanionRepository` and schema v3 do not exist.

- [x] **Step 6: Implement schema v3 and repository transactions**

Create `companion_skins` and `companion_settings` tables. Update `assets::migrate_schema` so v2 adds these tables, writes version `3`, and a v3 database remains accepted. Seed each built-in with `INSERT INTO companion_skins (id, name, source, visual_preset, texture_path, preview_path, flow_colors, flow_speed, flow_intensity, created_at) VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?6, ?7, ?8) ON CONFLICT(id) DO NOTHING`. Wrap active-skin changes and deletes in transactions.

```sql
CREATE TABLE companion_skins (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  source TEXT NOT NULL,
  visual_preset TEXT NOT NULL,
  texture_path TEXT,
  preview_path TEXT,
  flow_colors TEXT NOT NULL,
  flow_speed REAL NOT NULL,
  flow_intensity REAL NOT NULL,
  created_at TEXT NOT NULL
);
CREATE TABLE companion_settings (
  singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
  active_skin_id TEXT NOT NULL REFERENCES companion_skins(id),
  motion_enabled INTEGER NOT NULL,
  visible INTEGER NOT NULL,
  placement_json TEXT
);
```

- [x] **Step 7: Run focused and existing migration tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml repository::companion_test
cargo test --manifest-path src-tauri/Cargo.toml repository::assets_test
```

Expected: both suites pass; the existing asset migration remains green.

- [x] **Step 8: Commit**

```powershell
git add src-tauri/src/domain src-tauri/src/repository
git commit -m "feat: persist companion skins and settings"
```

---

### Task 2: Normalize PNG and WebP Skin Imports

**Files:**
- Create: `src-tauri/src/services/skins.rs`
- Create: `src-tauri/src/services/skins_test.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`

**Interfaces:**
- Consumes: `CompanionRepository`, `CompanionSkin`.
- Produces:
  - `import_image_skin(path: &Path, name: Option<&str>, data_dir: &Path, repository: &CompanionRepository) -> Result<CompanionSkin, SkinImportError>`
  - `derive_flow_colors(image: &DynamicImage) -> [String; 2]`
- Normalizes the source into `texture.png` and `preview.png` inside a generated skin directory.

- [x] **Step 1: Enable WebP decoding and write failing import tests**

Update image features:

```toml
image = { version = "0.25", default-features = false, features = ["jpeg", "png", "webp"] }
```

Tests:

```rust
#[test]
fn imports_and_center_crops_a_rectangular_webp() {
    let skin = import_fixture("wide.webp");
    assert!(skin.texture_path.as_ref().unwrap().ends_with("texture.png"));
    assert_eq!(image::open(skin.texture_path.unwrap()).unwrap().dimensions(), (1024, 1024));
}

#[test]
fn rejects_oversized_or_too_small_images_without_leaving_files() {
    assert!(matches!(import_fixture_result("64.png"), Err(SkinImportError::Dimensions { .. })));
    assert_skin_directory_is_empty();
}
```

- [x] **Step 2: Run focused tests and confirm RED**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml services::skins_test -- --nocapture
```

Expected: compile failure because `services::skins` does not exist.

- [x] **Step 3: Implement guarded normalization**

The implementation must:

```rust
const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
const MIN_DIMENSION: u32 = 128;
const MAX_DIMENSION: u32 = 4096;

pub fn import_image_skin(
    path: &Path,
    name: Option<&str>,
    data_dir: &Path,
    repository: &CompanionRepository,
) -> Result<CompanionSkin, SkinImportError> {
    validate_source_size(path, MAX_IMAGE_BYTES)?;
    let decoded = image::open(path)?;
    validate_dimensions(decoded.dimensions())?;
    let square = center_crop_square(decoded);
    let colors = derive_flow_colors(&square);
    persist_normalized_skin(square, colors, data_dir, repository)
}
```

Create the skin in a temporary sibling directory, save a maximum `1024 × 1024` texture and `256 × 256` preview, persist the repository record, then atomically rename the directory. Delete the record and temporary files on any failure.

- [x] **Step 4: Implement deterministic safe palette extraction**

Resize to `16 × 16`, ignore pixels with alpha below `0.2`, average in linear RGB, clamp saturation and luminance, and generate a lighter companion color. Tests assert stable hex output for purple, near-white, near-black, and transparent fixtures.

- [x] **Step 5: Run service and repository tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml services::skins_test
cargo test --manifest-path src-tauri/Cargo.toml repository::companion_test
```

Expected: all pass and failed imports leave no directory or database row.

- [x] **Step 6: Commit**

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/services
git commit -m "feat: import local image skins"
```

---

### Task 3: Validate and Import Versioned ZIP Skin Packages

**Files:**
- Modify: `src-tauri/src/services/skins.rs`
- Modify: `src-tauri/src/services/skins_test.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`

**Interfaces:**
- Produces:
  - `SkinPackageManifest`
  - `import_zip_skin(path: &Path, data_dir: &Path, repository: &CompanionRepository) -> Result<CompanionSkin, SkinImportError>`
- Uses zip `8.6.0` with only deflate support.

- [x] **Step 1: Add the exact ZIP dependency and failing happy-path test**

```toml
zip = { version = "=8.6.0", default-features = false, features = ["deflate"] }
```

```rust
#[test]
fn imports_a_v1_package_after_full_validation() {
    let package = zip_fixture(vec![
        ("manifest.json", valid_manifest("texture.webp")),
        ("texture.webp", valid_webp_bytes()),
    ]);
    let skin = import_zip_skin(&package, data_dir(), repository()).unwrap();
    assert_eq!(skin.source, SkinSource::Package);
    assert_eq!(skin.flow_colors, vec!["#B79CFF", "#FFE4B5"]);
}
```

- [x] **Step 2: Add malicious archive tests before implementation**

```rust
#[test]
fn rejects_traversal_symlinks_bombs_and_unreferenced_files() {
    assert_rejected(zip_with("../escape.png"), SkinImportErrorKind::UnsafePath);
    assert_rejected(zip_with_symlink("texture.png"), SkinImportErrorKind::Symlink);
    assert_rejected(zip_over_uncompressed_limit(), SkinImportErrorKind::ArchiveTooLarge);
    assert_rejected(zip_with("payload.html"), SkinImportErrorKind::UnsupportedEntry);
    assert_rejected(zip_with_extra("unused.png"), SkinImportErrorKind::UnreferencedEntry);
}
```

Also cover compressed size, entry count, manifest size, unknown version, remote URL, invalid color, invalid speed/intensity, missing texture, and invalid decoded dimensions.

- [x] **Step 3: Run the ZIP tests and confirm RED**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml services::skins_test::zip -- --nocapture
```

Expected: compile failure because ZIP interfaces are absent.

- [x] **Step 4: Implement validate-before-extract**

Iterate entries with `ZipFile::enclosed_name()`, reject `!is_file()`, inspect `unix_mode()` for symlinks, sum declared and copied bytes, and stream each accepted file through `Read::take(remaining + 1)`. Never call bulk `ZipArchive::extract`.

```rust
const MAX_ZIP_BYTES: u64 = 20 * 1024 * 1024;
const MAX_UNCOMPRESSED_BYTES: u64 = 40 * 1024 * 1024;
const MAX_ENTRIES: usize = 16;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
```

Parse manifest v1, validate that the exact root entry set is `manifest.json` plus referenced PNG/WebP files, then normalize via the same image pipeline from Task 2. Clamp `flowSpeed` to `0.5..=2.0` and `flowIntensity` to `0.0..=1.0`.

- [x] **Step 5: Run all skin service tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml services::skins_test
```

Expected: valid packages import; every malicious or malformed package is rejected without residue.

- [x] **Step 6: Commit**

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/services/skins.rs src-tauri/src/services/skins_test.rs
git commit -m "feat: validate local skin packages"
```

---

### Task 4: Skin IPC, Cross-Window Events, and Window Lifecycle

**Files:**
- Create: `src-tauri/src/commands/companion.rs`
- Create: `src-tauri/src/commands/companion_test.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Create: `src-tauri/capabilities/companion.json`
- Modify: `src-tauri/src/asset_protocol_config_test.rs`

**Interfaces:**
- Produces commands:
  - `list_companion_skins`
  - `get_companion_settings`
  - `import_companion_skin`
  - `update_companion_skin`
  - `set_active_companion_skin`
  - `delete_companion_skin`
  - `show_companion`, `hide_companion`, `focus_main_window`
  - `set_companion_expanded`
  - `save_companion_placement`
- Emits complete committed payloads on `companion-skin-changed`, `companion-settings-changed`, and `companion-visibility-changed`.

- [x] **Step 1: Write failing Tauri configuration tests**

```rust
#[test]
fn config_declares_a_safe_companion_window() {
    let companion = window_config("companion");
    assert_eq!(companion.width, 72.0);
    assert_eq!(companion.height, 72.0);
    assert_eq!(companion.decorations, false);
    assert_eq!(companion.always_on_top, true);
    assert_eq!(companion.skip_taskbar, true);
    assert_eq!(companion.resizable, false);
}
```

Also assert the asset protocol contains exactly previews and skins, and that `companion.json` grants core default plus only the start-dragging permission needed by the webview.

- [x] **Step 2: Run the configuration test and confirm RED**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml asset_protocol_config_test -- --nocapture
```

Expected: fail because the companion config and skin scope are absent.

- [x] **Step 3: Add the second configured window and capabilities**

Add window label `companion`, URL `index.html?window=companion`, size and safe flags. Extend asset protocol scope with:

```json
"$APPLOCALDATA/skins/**/*"
```

Keep the main capability assigned only to `main`; add a companion-specific capability for `companion`.

- [x] **Step 4: Write failing command/event tests with MockRuntime**

```rust
#[test]
fn setting_active_skin_emits_exactly_once_after_commit() {
    let (app, events) = mock_app_with_companion_repository();
    set_active_companion_skin("deep-ink".into(), app.handle().clone()).unwrap();
    assert_eq!(events.named("companion-skin-changed").len(), 1);
    assert_eq!(events.first_payload()["activeSkinId"], "deep-ink");
}

#[test]
fn an_invalid_window_operation_changes_nothing() {
    assert!(matches!(
        set_known_window_visible("other", true, &app),
        Err(CompanionCommandError::UnknownWindow)
    ));
}
```

Cover failed import/set/delete emitting zero events and companion close hiding without exiting.

- [x] **Step 5: Implement commands and lifecycle**

Manage `CompanionRepository` next to `AssetRepository`. Register all commands. On companion close request, call `prevent_close()` and hide. On main close, call `app.exit(0)`.

For expansion, compute the new logical size and position from the current monitor work area:

```rust
pub fn anchored_companion_bounds(
    current: LogicalPosition<f64>,
    monitor: LogicalRect,
    expanded: bool,
) -> (LogicalPosition<f64>, LogicalSize<f64>)
```

Choose left/right and up/down expansion based on available space, then clamp the result. Emit visibility/settings events only after state persistence succeeds.

- [x] **Step 6: Run focused Rust tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml commands::companion_test
cargo test --manifest-path src-tauri/Cargo.toml asset_protocol_config_test
```

Expected: commands, event counts, configuration, anchoring, close behavior, and scope pass.

- [x] **Step 7: Commit**

```powershell
git add src-tauri
git commit -m "feat: add companion window lifecycle and IPC"
```

---

### Task 5: TypeScript Client and Window-Specific Entry Routing

**Files:**
- Create: `src/lib/companion.ts`
- Create: `src/lib/companion.test.ts`
- Create: `src/Root.tsx`
- Create: `src/Root.test.tsx`
- Modify: `src/main.tsx`
- Modify: `src/App.tsx`
- Modify: `src/App.test.tsx`
- Modify: `src/lib/desktop.ts`

**Interfaces:**
- Produces frontend `CompanionSkin`, `CompanionSettings`, `SkinImportSelection`.
- Produces typed invoke/listen wrappers matching Task 4.
- `Root` renders `<App />` for `main` and `<CompanionWindow />` for `companion`.

- [x] **Step 1: Write failing routing and client contract tests**

```tsx
it('renders only the companion entry for the companion window', () => {
  render(<Root windowLabel="companion" />);
  expect(screen.getByRole('complementary', { name: '悬浮助手' })).toBeVisible();
  expect(screen.queryByRole('main', { name: '影像资料库' })).not.toBeInTheDocument();
});
```

```ts
it('invokes package import for zip and image import for png', async () => {
  await importCompanionSkin('C:\\skins\\theme.zip');
  expect(invoke).toHaveBeenCalledWith('import_companion_skin', { path: 'C:\\skins\\theme.zip' });
});
```

- [x] **Step 2: Run focused Vitest and confirm RED**

```powershell
npm test -- Root companion
```

Expected: fail because `Root` and client wrappers do not exist.

- [x] **Step 3: Implement exact TypeScript contracts**

```ts
export type VisualPreset =
  | 'quiet_aurora'
  | 'porcelain_pearl'
  | 'deep_ink'
  | 'custom';

export type CompanionSkin = {
  id: string;
  name: string;
  source: 'builtin' | 'image' | 'package';
  visualPreset: VisualPreset;
  texturePath: string | null;
  previewPath: string | null;
  flowColors: string[];
  flowSpeed: number;
  flowIntensity: number;
  createdAt: string;
};
```

Add wrappers and subscriptions. Every subscription returns the Tauri unlisten function and callers handle rejected registration promises.

- [x] **Step 4: Implement entry routing and remove embedded companion**

Read `getCurrentWebviewWindow().label` in `main.tsx`, pass it to `Root`, and remove `<CompanionWindow />` from `App`. Preserve dependency injection in `Root` tests so Vitest does not require a real Tauri window.

- [x] **Step 5: Run focused and existing app tests**

```powershell
npm test -- Root companion App
```

Expected: main and companion render exactly once and existing library behavior remains green.

- [x] **Step 6: Commit**

```powershell
git add src/main.tsx src/Root.tsx src/Root.test.tsx src/App.tsx src/App.test.tsx src/lib
git commit -m "feat: route independent companion frontend"
```

---

### Task 6: Main Window Skin Library

**Files:**
- Create: `src/features/skins/SkinLibrary.tsx`
- Create: `src/features/skins/SkinLibrary.test.tsx`
- Create: `src/features/skins/SkinCard.tsx`
- Create: `src/features/skins/SkinControls.tsx`
- Create: `src/features/skins/SkinPreview.tsx`
- Modify: `src/App.tsx`
- Modify: `src/App.test.tsx`
- Modify: `src/app.css`

**Interfaces:**
- Consumes: Task 5 client wrappers.
- Produces the `skins` main-window view and reusable `SkinPreview`.
- `SkinPreview` renders all three complete built-in token sets and custom textures.

- [x] **Step 1: Write failing library behavior tests**

```tsx
it('lists all builtins and switches only after the command succeeds', async () => {
  renderSkinLibrary();
  expect(screen.getByText('静谧极光')).toBeVisible();
  expect(screen.getByText('雾白珍珠')).toBeVisible();
  expect(screen.getByText('深海墨色')).toBeVisible();
  await userEvent.click(screen.getByRole('button', { name: '使用深海墨色' }));
  expect(setActiveCompanionSkin).toHaveBeenCalledWith('deep-ink');
});

it('keeps the current skin and exposes an alert when import fails', async () => {
  importCompanionSkin.mockRejectedValue(new Error('invalid package'));
  await importThroughLibrary('broken.zip');
  expect(await screen.findByRole('alert')).toHaveTextContent('皮肤导入失败');
});
```

Also test cancel, image/ZIP filters, rename, safe slider ranges, built-in delete absence, active custom delete fallback, event refresh, and delayed-listener cleanup.

- [x] **Step 2: Run focused tests and confirm RED**

```powershell
npm test -- SkinLibrary
```

Expected: fail because skin components do not exist.

- [x] **Step 3: Implement the view and controls**

Add `view: 'book' | 'gallery' | 'skins'` to `App`. The skin grid uses semantic buttons, `aria-pressed` for the active skin, a recoverable `role="alert"`, and `aria-live="polite"` for successful imports.

`SkinPreview` maps explicit preset classes:

```ts
const presetClass: Record<VisualPreset, string> = {
  quiet_aurora: 'skin-preview--aurora',
  porcelain_pearl: 'skin-preview--pearl',
  deep_ink: 'skin-preview--ink',
  custom: 'skin-preview--custom',
};
```

Do not use CSS `hue-rotate`.

- [x] **Step 4: Implement file selection and asset URLs**

The dialog filters are:

```ts
[
  { name: '皮肤文件', extensions: ['png', 'webp', 'zip'] }
]
```

Convert preview/texture paths with `convertFileSrc`. Keep all command errors recoverable and preserve active selection until successful payload arrives.

- [x] **Step 5: Run focused and app tests**

```powershell
npm test -- SkinLibrary App
```

Expected: all skin management and existing main shell tests pass.

- [x] **Step 6: Commit**

```powershell
git add src/features/skins src/App.tsx src/App.test.tsx src/app.css
git commit -m "feat: add local skin library"
```

---

### Task 7: Companion Interaction and Three Animated Skins

**Files:**
- Modify: `src/features/floating-companion/CompanionWindow.tsx`
- Modify: `src/features/floating-companion/CompanionWindow.test.tsx`
- Modify: `src/features/floating-companion/CompanionMenu.tsx`
- Create: `src/features/floating-companion/CompanionOrb.tsx`
- Create: `src/features/floating-companion/CompanionOrb.test.tsx`
- Create: `src/features/floating-companion/useCompanionPosition.ts`
- Create: `src/features/floating-companion/useCompanionPosition.test.ts`
- Modify: `src/app.css`

**Interfaces:**
- Consumes: `SkinPreview` visual tokens and Task 5 client/window wrappers.
- Produces collapsed orb, anchored expanded menu, drag persistence, quick switching, status motion, and accessible keyboard behavior.

- [ ] **Step 1: Write failing window interaction tests**

```tsx
it('expands, restores focus on Escape, and requests anchored native resize', async () => {
  renderCompanion();
  const trigger = screen.getByRole('button', { name: '打开悬浮助手菜单' });
  await userEvent.click(trigger);
  expect(setCompanionExpanded).toHaveBeenCalledWith(true);
  await userEvent.keyboard('{Escape}');
  expect(setCompanionExpanded).toHaveBeenLastCalledWith(false);
  expect(trigger).toHaveFocus();
});

it('starts native dragging only from the orb drag handle', async () => {
  renderCompanion();
  fireEvent.pointerDown(screen.getByTestId('companion-drag-handle'), { button: 0 });
  expect(startDragging).toHaveBeenCalledTimes(1);
});
```

Also test quick skin switch, show main, hide companion, import/capture statuses, placement debounce, subscription cleanup, and failed actions.

- [ ] **Step 2: Write failing visual token and reduced-motion tests**

```tsx
it.each([
  ['quiet_aurora', 'companion-orb--aurora'],
  ['porcelain_pearl', 'companion-orb--pearl'],
  ['deep_ink', 'companion-orb--ink'],
])('renders the complete %s preset', (preset, className) => {
  render(<CompanionOrb skin={skinWith(preset)} motionEnabled status="idle" />);
  expect(screen.getByTestId('companion-orb')).toHaveClass(className);
});
```

Assert the DOM has two arc runners, one surface current, five star points, and a central star. With motion disabled, the root has `companion-orb--motion-off`.

- [ ] **Step 3: Run focused tests and confirm RED**

```powershell
npm test -- CompanionOrb CompanionWindow useCompanionPosition
```

Expected: fail because the new orb and window APIs do not exist.

- [ ] **Step 4: Implement semantic structure and window behavior**

The trigger exposes `aria-haspopup="menu"` and `aria-expanded`. Expanded content has labelled quick skin buttons and actions. Escape collapses and restores focus. Pointer dragging ignores non-primary buttons and interactive menu descendants. Position events debounce for `250 ms` before calling `saveCompanionPlacement`.

- [ ] **Step 5: Implement the approved animated baseline**

CSS must provide:

- `.companion-orb__ring-track`
- two `.companion-orb__ring-runner` layers moving at `4.8s` and `7.2s` in opposite directions
- `.companion-orb__surface-current` at `7.5s`
- `.companion-orb__surface-sheen` at `5.8s`
- five staggered `.companion-orb__star-point`
- `.companion-orb__central-star` at `3.8s`

Define separate variables and material backgrounds under:

```css
.companion-orb--aurora {
  --orb-core: #5b518a;
  --orb-edge: #eee7ff;
  --flow-primary: #bd9fff;
  --flow-secondary: #fff4dc;
}
.companion-orb--pearl {
  --orb-core: #c9c5c3;
  --orb-edge: #ffffff;
  --flow-primary: #e3bd7e;
  --flow-secondary: #fff8ea;
}
.companion-orb--ink {
  --orb-core: #17454f;
  --orb-edge: #8ee7df;
  --flow-primary: #49d9cf;
  --flow-secondary: #d4fff8;
}
```

Under `@media (prefers-reduced-motion: reduce)` and `.companion-orb--motion-off`, set every animation to `none`.

- [ ] **Step 6: Run companion, skin, and main-window tests**

```powershell
npm test -- CompanionOrb CompanionWindow useCompanionPosition SkinLibrary App Root
```

Expected: all pass, including existing import and capture error recovery.

- [ ] **Step 7: Commit**

```powershell
git add src/features/floating-companion src/app.css
git commit -m "feat: animate independent companion skins"
```

---

### Task 8: Full Verification, Windows Smoke, Handoff, and Stop

**Files:**
- Modify: `docs/CONTINUATION.md`
- Create: `.superpowers/sdd/2026-07-30-floating-companion-skin-library/verification-report.md`
- Modify: `.superpowers/sdd/2026-07-30-floating-companion-skin-library/progress.md`

**Interfaces:**
- Consumes the completed application.
- Produces final evidence, updated cross-account handoff, clean pushed branch, and refreshed local bundle.

- [ ] **Step 1: Run formatting and complete automated verification**

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
npm run tauri -- build --debug --no-bundle
git diff --check
```

Expected: every command exits `0`; record exact test counts and the executable path.

- [ ] **Step 2: Launch the debug application for Windows smoke**

```powershell
npm run tauri -- dev
```

Verify and record:

1. Main and companion appear at startup.
2. Companion remains above another normal window.
3. Drag, restart, restored position, and edge-aware expansion work.
4. Companion close hides; main shows it again; main close exits.
5. All three skins render and animate at `72 × 72`.
6. Motion switch and Windows reduced-motion behavior stop animation.
7. PNG, WebP, valid ZIP, traversal ZIP, oversized ZIP, and malformed manifest paths behave as specified.
8. Import, three capture modes, quick switch, open main, keyboard, Escape, and focus restoration work.

If GUI automation cannot establish a fact, mark that item “manual verification required”; do not report it as passed.

- [ ] **Step 3: Review only the new feature range**

Create a diff from `5b592ed` to current HEAD and request a fresh read-only review for spec compliance, security boundaries, event consistency, focus/keyboard behavior, and test quality. Fix only confirmed load-bearing findings with TDD and a dedicated commit.

- [ ] **Step 4: Update the handoff document**

Add:

- latest implementation and handoff commits
- completed window/skin capabilities
- exact test commands/counts
- Windows smoke result
- file/package limits and security decisions
- known issues and future community/direct-reference/programmable-skin requirements
- startup and cross-account continuation steps

- [ ] **Step 5: Commit handoff documentation**

```powershell
git add docs/CONTINUATION.md
git commit -m "docs: hand off floating companion skin library"
```

- [ ] **Step 6: Refresh and verify the local offline bundle**

```powershell
pwsh -NoProfile -File scripts/refresh-handoff.ps1
git bundle verify handoff/magic-image-library.bundle
Get-Content handoff/manifest.json
```

Expected: manifest branch is `feat/magic-image-library` and its HEAD equals local HEAD.

- [ ] **Step 7: Push and prove synchronization**

```powershell
git push origin feat/magic-image-library
git status --short --branch
git rev-parse HEAD
git rev-parse origin/feat/magic-image-library
git ls-remote --heads origin feat/magic-image-library
```

Expected: clean status and all three hashes are identical.

- [ ] **Step 8: Stop**

Report completed scope, tests, smoke evidence, commit, push, handoff document, bundle path, and remaining known issues. Do not start another feature.
