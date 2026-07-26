# 本地 MVP 真实资源资料库恢复设计

## 目标与范围

本次恢复仅完成本地 MVP 的真实资源浏览闭环，不包含账号、上传、离线同步队列、重试或冲突处理。图片导入或截图成功后，资源必须能在魔法书与传统图库中按月份和日期查看。

## 架构

Rust `AssetRepository` 继续是本地资源的唯一事实来源。已有的 `list_assets_by_month(year, month)` 与 `list_assets_by_day(year, month, day)` Tauri 命令返回序列化的 `Asset`；前端通过 `src/lib/assets.ts` 调用它们，不得再使用日期或图片演示数据。

前端定义与 Rust JSON 形状对应的 `Asset` 类型。`MagicBookView` 加载所选月份的资源并从 `createdAt` 派生去重、降序的日期流；选中日期时，它调用按日查询，只渲染该日的资源。`ClassicGallery` 消费同一月度查询接口，以缩略图网格展示资源。两种视图都显示加载、空态和错误状态。

## 组件边界

- `src/lib/assets.ts`：Tauri IPC 边界；导出 `listAssetsByMonth`、`listAssetsByDay` 与前端 `Asset` 类型。
- `src/features/library/MagicBookView.tsx`：管理月份、日期与请求生命周期；将图片集合和交互状态下传。
- `src/features/library/DateFlow.tsx`：只负责渲染可访问的日期选择列表。
- `src/features/library/ImageStack.tsx`：只负责渲染当前日资源的叠页、预览图、单图居中与多图滑轨。
- `src/features/library/ClassicGallery.tsx`：只负责将当月资源渲染成可访问的缩略图网格。
- `src/App.tsx`：保留浏览模式状态，向两个视图传入初始月份。

## 数据与错误处理

资源预览使用 Tauri 的 `convertFileSrc` 将本地预览路径转换为 WebView 可加载 URL。资产查询被拒绝或失败时，界面展示中文错误信息和“重试”按钮；重试调用当前查询。所选月份没有资源时，日期栏与图片区均显示空态，且不保留上个月数据。所选日期没有资源时，图片区显示该日空态。

`Asset` 的时间序列化采用现有 Rust `DateTime<Utc>` 的 camelCase JSON 字段；日期流以 `createdAt` 的 UTC 日历日期派生。本恢复不改变数据库 schema 或截图元数据，时区与截图模式属于后续本地 MVP 交互/截图阶段。

## 验证

Vitest 覆盖：月份查询替换日期流和缩略图；点击日期发出正确的日查询并替换叠页；单图隐藏滑轨、多图显示滑轨；空态、错误态和重试。Rust 回归测试继续证明月份与日期查询的数据库边界。生产构建与 Rust 测试必须通过。

## 非目标

本次不实现区域/窗口截图、独立悬浮窗口、拖动/滚轮高级交互、搜索/标签/收藏管理、账户或云同步。这些功能将在后续恢复计划中单独执行。
