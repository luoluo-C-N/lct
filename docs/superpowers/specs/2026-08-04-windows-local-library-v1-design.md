# 魔法影像库 Windows 单机可用版 1.0 设计

**日期：** 2026-08-04

**状态：** 已批准

## 1. 二次产品定义

魔法影像库 1.0 是面向单个 Windows 10/11 用户的本地图片资料库。产品的首要价值是让用户可靠地完成“收集、查找、查看、整理、删除与恢复”图片，而不是展示主题、皮肤或云端愿景。

核心闭环为：

```text
导入或截图
→ 自动生成缩略图并入库
→ 在默认图库中立即出现
→ 按月份、文件名、来源、标签和收藏状态查找
→ 打开大图详情
→ 修改收藏和标签
→ 删除到回收站
→ 恢复或二次确认后永久删除
```

1.0 必须在真实 Windows 安装包中完成上述闭环，重启后数据保持正确，且现有截图、魔法书和悬浮助手能力不回退。

## 2. 版本边界

### 2.0 当前项目审计与纠偏

本轮审计确认以下结构性问题，并通过本规格纠偏：

- **投入顺序倒置**：皮肤包校验、悬浮伴侣和截图异常恢复已经高度完善，但资产详情、收藏、标签 UI、删除、搜索等核心闭环仍未完成。1.0 停止扩展皮肤和主题，把交付资源转向资产管理。
- **契约不一致**：Rust `Asset` 已有标签、删除时间、截图模式和预留字段，TypeScript 类型却缺失这些属性。1.0 先统一契约再增加 UI。
- **查询接口碎片化**：现有月份和日期接口适合魔法书，不适合搜索、组合筛选和大数据分页。图库改用统一游标查询，魔法书暂时保留旧接口。
- **变更同步单一**：只有 `asset-created`，无法同步收藏、标签、删除和恢复。1.0 引入统一资产变更事件，并保证命令返回值是当前窗口的主确认。
- **发布工程缺失**：仓库没有项目 README、CI、Playwright E2E 或可验收安装包流程。1.0 将这些设为发布门槛，而不是完成代码后再补。
- **性能目标缺失**：现有列表按整月加载且没有虚拟化。1.0 以 10,000 张资产为基线，限制单页大小并增加查询计划和耗时测试。
- **模块职责过大**：`companion.rs`、`skins.rs` 和对应测试均超过 600 行。1.0 不做无关重写，只在触及资产管理和设置入口时拆分职责。
- **需求文档混杂版本**：旧 PRD 同时要求本地库、视觉主题、云同步、托盘、定时截图和移动端愿景。1.0 以本规格为唯一产品与实现边界，旧文档仅作历史参考。

### 2.1 纳入 1.0

- 本地 PNG、JPEG、WebP 导入。
- 全屏、主显示器区域和前台窗口截图。
- 图库默认首页与魔法书特色视图。
- 大图预览与资产详情。
- 收藏、标签编辑。
- 按月份、显示名称、来源、标签和收藏组合筛选。
- 软删除、回收站恢复和永久删除。
- 10,000 张资产性能基线。
- 数据迁移、错误恢复、CI、Windows E2E 和安装包验收。

### 2.2 延后

- 相册和批量操作。
- 截图标注和复杂图片编辑。
- 星空主题扩展和新的皮肤格式。
- 账号、云同步和移动端。
- 定时截图、系统托盘、全局快捷键、开机自启和自动备份。

数据库现有 `annotation_data`、`sync_version`、`cloud_id` 和 `album_id` 字段保留以兼容旧数据，但 1.0 不新增空同步服务，不在普通 UI 中展示未实现能力。现有皮肤导入与切换保持可用，但不继续扩展。

## 3. 信息架构

主导航包含：

1. **图库**：默认首页，承担浏览、搜索、筛选和资产管理。
2. **魔法书**：按月份和日期回顾图片的特色视图。
3. **回收站**：恢复或永久删除已删除资产。

导航底部提供 **设置** 入口。设置页承载悬浮助手显示、动效和现有皮肤管理；皮肤库不再占用一级导航。

### 3.1 图库

顶部固定工具栏依次提供月份选择、显示名称搜索、来源筛选、标签筛选、仅看收藏、导入图片和截图菜单。结果区使用游标分页与虚拟化网格；卡片仅显示缩略图、收藏状态、来源和显示名称。

单击卡片打开右侧详情面板。关闭面板不得重置月份、筛选条件或滚动位置。

### 3.2 资产详情面板

详情面板宽约 380px，显示：

- 较大预览图；
- 显示名称、像素尺寸和文件大小；
- 创建时间和导入时间；
- 导入/截图来源及截图模式；
- 原始文件路径；
- 收藏开关；
- 标签编辑；
- 移入回收站。

图库和魔法书复用同一个详情面板与同一套资产变更命令。

### 3.3 回收站

回收站不按月份限制，按 `deleted_at DESC, id DESC` 游标分页。每项提供恢复和永久删除。永久删除必须弹出二次确认，并明确说明原图和缩略图都无法恢复。

### 3.4 悬浮助手

悬浮助手继续只负责导入、三种截图、打开主界面、显示状态和现有快速外观切换。搜索、标签、删除等管理操作只在主窗口执行。

## 4. 资产契约

Rust 与 TypeScript 使用完整一致的 `Asset` 契约：

```ts
type Asset = {
  id: string;
  createdAt: string;
  importedAt: string;
  source: 'import' | 'capture';
  originalPath: string;
  previewPath: string;
  displayName: string;
  albumId: string | null;
  tags: string[];
  favorite: boolean;
  deletedAt: string | null;
  captureMode: 'fullscreen' | 'region' | 'window' | null;
  annotationData: string | null;
  syncVersion: number;
  cloudId: string | null;
};
```

标签仍以 `tags` 和 `asset_tags` 关系表为唯一来源。`assets` 表不增加 JSON 标签列。

导入资产的 `display_name` 使用源文件名；截图使用本地化可读名称，例如 `区域截图 2026-08-04 14-32-08.png`。托管目录中的随机文件名不用于用户搜索或显示。

详情面板按需读取文件系统信息，不把易变化的文件大小和尺寸冗余写入数据库：

```ts
type AssetDetails = {
  asset: Asset;
  originalExists: boolean;
  previewExists: boolean;
  width: number | null;
  height: number | null;
  fileSize: number | null;
};
```

`get_asset_details(id)` 从 Repository 读取资产后，只检查托管路径并读取元数据。详情大图使用托管 `original_path`，文件缺失或解码失败时回退到 `preview_path`。Tauri asset protocol scope增加 `$APPLOCALDATA/assets/originals/**/*`，不开放任意本地路径。

## 5. 数据库迁移

schema 从 v3 升至 v4，执行原地迁移，不重建 `assets`：

```sql
ALTER TABLE assets ADD COLUMN display_name TEXT NOT NULL DEFAULT '';
CREATE INDEX IF NOT EXISTS idx_assets_active_created
  ON assets(deleted_at, created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_assets_source_created
  ON assets(source, created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_assets_favorite_created
  ON assets(favorite, created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_asset_tags_tag_asset
  ON asset_tags(tag_id, asset_id);
```

迁移函数单独保留并可注入内存数据库测试。旧记录的 `display_name` 从 `original_path` 文件名回填；无法解析时使用资产 ID。迁移成功后才把 `app_meta.schema_version` 写为 `4`。

## 6. 查询与分页

图库和回收站使用统一命令：

```rust
pub struct AssetQuery {
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub text: Option<String>,
    pub tags: Vec<String>,
    pub source: Option<AssetSource>,
    pub favorite_only: bool,
    pub deleted: bool,
    pub cursor: Option<AssetCursor>,
    pub limit: u32,
}

pub struct AssetCursor {
    pub sort_timestamp: DateTime<Utc>,
    pub id: String,
}

pub struct AssetPage {
    pub items: Vec<Asset>,
    pub next_cursor: Option<AssetCursor>,
}
```

查询规则：

- `limit` 默认 60，最大 120。
- 普通图库要求 `deleted_at IS NULL`；回收站要求 `deleted_at IS NOT NULL`。
- 普通图库按 `created_at DESC, id DESC`；回收站按 `deleted_at DESC, id DESC`。
- 分页使用稳定游标，不使用大偏移量 `OFFSET`。
- 显示名称搜索去除首尾空白，英文比较忽略大小写，并对 SQL 通配符进行转义。
- 多标签采用 AND 逻辑。
- 月份、名称、来源、收藏和标签可以组合。
- 空搜索词、空标签和重复标签在进入 Repository 前归一化。

现有 `list_assets_by_month` 和 `list_assets_by_day` 暂时保留给魔法书，以减少回归面。

## 7. 写操作与事件

新增命令：

```rust
update_asset_favorite(id: String, favorite: bool) -> Result<Asset, AssetCommandError>
set_asset_tags(id: String, tags: Vec<String>) -> Result<Asset, AssetCommandError>
soft_delete_asset(id: String) -> Result<Asset, AssetCommandError>
restore_asset(id: String) -> Result<Asset, AssetCommandError>
permanently_delete_asset(id: String, app: AppHandle) -> Result<(), AssetCommandError>
query_assets(query: AssetQuery) -> Result<AssetPage, AssetCommandError>
get_asset_details(id: String) -> Result<AssetDetails, AssetCommandError>
```

收藏和标签更新必须在数据库事务成功后返回完整 `Asset`。标签归一化规则为：去首尾空白、删除空值、按大小写不敏感方式去重、最多 20 个标签、单标签最多 40 个 Unicode 标量。

所有资产变更发出统一事件：

```ts
type AssetChangedEvent = {
  kind: 'created' | 'updated' | 'trashed' | 'restored' | 'deleted';
  assetId: string;
  asset?: Asset;
};
```

创建、更新、移入回收站和恢复事件包含完整资产；永久删除只包含 ID。事件仅在持久化成功后发送。事件发送是提交后的尽力通知：发送失败必须记录诊断信息，但不能把已经提交的命令改报为失败。旧 `asset-created` 在图库和魔法书迁移完成前继续发送，随后在同一版本内删除旧监听。

## 8. 删除安全与一致性

软删除只写 `deleted_at`，不移动或删除文件。恢复只清空 `deleted_at`。

永久删除仅允许已位于回收站的资产。后端必须：

1. 从数据库读取完整资产。
2. 将 `original_path` 和 `preview_path` 规范化并验证其分别位于 `$APPLOCALDATA/assets/originals` 与 `$APPLOCALDATA/assets/previews`。
3. 在 `$APPLOCALDATA/assets/.deleting/<asset-id>/manifest.json` 写入资产 ID、原始路径和缩略图路径，再把存在的文件原子移动到同目录下的 `original` 与 `preview` 暂存文件。
4. 在数据库事务中删除 `asset_tags` 和 `assets` 记录。
5. 数据库失败时把暂存文件移动回原位置。
6. 数据库提交后删除暂存目录，并发送 `deleted` 事件。

启动时读取每个暂存目录的 manifest：数据库仍有对应记录时按 manifest 恢复文件；数据库已无对应记录时清理暂存目录。manifest 缺失或非法时不自动删除其内容，而是记录诊断并保留目录供人工恢复。这样进程崩溃不会静默留下“记录存在但文件已被删除”的状态。

任何输入路径超出托管目录都必须拒绝永久删除，且不得触碰该文件。

## 9. 前端状态与模块边界

新增 `useAssetQuery` 管理查询条件、游标页、加载更多、刷新和竞态取消。查询条件变化会清空旧页并请求第一页；加载下一页只追加，重复 ID 去重。

模块职责：

- `features/library/LibraryWorkspace.tsx`：图库页面布局与详情面板选择状态。
- `features/library/LibraryToolbar.tsx`：查询条件控件。
- `features/library/AssetGrid.tsx`：虚拟化网格和加载更多。
- `features/assets/AssetDetailsPanel.tsx`：详情展示。
- `features/assets/AssetTagsEditor.tsx`：标签编辑。
- `features/trash/TrashView.tsx`：回收站列表和确认流程。
- `features/settings/SettingsView.tsx`：悬浮助手和皮肤设置入口。
- `hooks/useAssetQuery.ts`：分页查询状态。
- `lib/assets.ts`：类型、IPC 封装和事件订阅。

Rust 侧把 `repository/assets.rs` 保持为 schema 与 SQL 所有者，但把路径删除协调放入 `services/assets.rs`，命令层只负责参数、AppHandle 路径解析与事件。现有截图和导入服务继续复用 Repository。

不引入全局状态库；当前规模使用局部 Hook 和 Tauri 事件即可。

## 10. 性能设计

验收数据集为 10,000 张资产、500 个唯一标签、30,000 个标签关系和跨 36 个月分布的数据。

性能目标：

- 常见月份第一页查询在开发机 SQLite 热缓存下小于 300ms。
- 搜索与组合过滤第一页小于 500ms。
- IPC 单页最多返回 120 条资产。
- 图库 DOM 中同时存在的卡片不超过可视区域加前后各两行。
- 缩略图统一使用 512px `preview_path`，图片启用懒加载。
- 输入搜索采用 250ms debounce；月份和 chips 立即查询。

性能测试记录查询耗时和 `EXPLAIN QUERY PLAN`，防止无索引全表扫描在常见路径中回归。

## 11. 错误体验

所有可恢复错误必须有明确出口：

- 查询失败：内容区 `role="alert"` 和重试按钮，保留当前筛选。
- 加载更多失败：网格尾部错误和重试，不清空已加载数据。
- 收藏/标签保存失败：详情面板显示错误，恢复最后一次服务端值。
- 软删除/恢复失败：保留当前卡片并显示错误。
- 永久删除失败：关闭忙碌态但保留回收站项，显示具体阶段的安全错误。
- 文件缺失：详情显示“原文件缺失”，禁止打开原图，但仍允许移入回收站或删除记录。
- 事件订阅失败：页面显示同步提示，同时保留手动刷新。

前端不得只依赖事件确认本地操作。命令返回的资产立即更新当前视图；事件用于同步其他视图和窗口。

## 12. 测试与发布门槛

### 12.1 Rust

- v3 → v4 原地迁移与旧名称回填。
- 查询条件组合、标签 AND、游标稳定性、最大页大小和软删除隔离。
- 收藏与标签事务及完整资产返回。
- 软删除、恢复、托管路径校验、永久删除补偿和启动恢复。
- 10,000 条数据性能基准与查询计划断言。

### 12.2 Vitest

- 图库为默认首页，设置取代一级皮肤入口。
- 查询条件、debounce、分页追加、竞态结果丢弃和事件刷新。
- 详情面板复用、收藏回滚、标签验证和删除确认。
- 回收站恢复、永久删除失败和加载更多失败。
- 魔法书打开同一详情面板。

### 12.3 Windows E2E

至少覆盖：

1. 导入图片并在图库出现。
2. 区域截图并在图库出现。
3. 搜索、收藏和标签组合过滤。
4. 删除到回收站、恢复、再次删除并永久删除。
5. 重启应用后收藏、标签和删除状态保持。

### 12.4 CI 与安装包

GitHub Actions 在 Windows 上执行 `npm test`、`cargo test`、`cargo fmt --check`、`npm run build` 和 Tauri debug 无 bundle 构建。正式候选版本额外生成 Windows 安装包，完成手工 smoke 清单后才可标记 1.0。

修正 bundle identifier，不再以 `.app` 结尾。1.0 必须具备 README、安装说明、数据目录说明、备份提醒和已知限制。签名与自动更新不纳入 1.0，但未签名安装包必须明确提示。

## 13. 交付阶段

1. **资产契约与 schema v4**：完整 Asset、显示名称、查询类型和迁移。
2. **统一查询与图库**：分页、筛选、虚拟网格和默认导航。
3. **详情与整理**：大图详情、收藏、标签及魔法书复用。
4. **安全删除**：软删除、回收站、恢复和永久删除补偿。
5. **设置收敛**：皮肤库移入设置，保持悬浮助手能力。
6. **发布工程**：性能基准、E2E、CI、文档和 Windows 安装包验收。

每个阶段必须使用测试优先方式独立提交。前一阶段测试未通过时不得进入下一阶段。

## 14. 验收标准

- 应用启动后默认显示图库。
- 用户能够在真实 Windows 安装包中完成核心闭环。
- 图库和回收站不一次性加载全部资产。
- 10,000 张数据下月份第一页查询小于 300ms，组合搜索第一页小于 500ms。
- 标签关系表是唯一标签来源。
- 软删除不删除文件，永久删除只能作用于托管目录且具备失败补偿。
- 图库和魔法书复用同一详情与编辑行为。
- 皮肤管理位于设置，不再占据一级导航。
- 现有导入、三种截图、魔法书和悬浮助手测试无回归。
- Rust、Vitest、Windows E2E、CI 构建和安装包 smoke 清单全部通过。
