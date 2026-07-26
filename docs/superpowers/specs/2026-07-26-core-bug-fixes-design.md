# 核心缺陷修复设计

## 范围

本批次只实现综合需求规格 v2 的 BUG-01 至 BUG-09。Phase 1 功能、主题和云同步不在本批次内。

## 数据与迁移

`Asset` 增加 `tags`、`deleted_at`、`capture_mode`、`annotation_data` 与 `cloud_id`。标签只由现有 `tags` 和 `asset_tags` 关系表持久化；repository 查询通过关联查询填充 `Asset.tags`，写入只更新关系表。启动时独立的迁移函数读取 `app_meta.schema_version`：新库创建 v2 schema 并记录版本 2；旧库从 v1 按固定顺序执行四条幂等的 `ALTER TABLE ... ADD COLUMN`（`deleted_at`、`capture_mode`、`annotation_data`、`cloud_id`），再记录版本 2。迁移函数接受 `Connection`，使其能用内存数据库单测。

## 截图与命令

`capture` 接收 `CaptureMode` 和可选 `CropRegion`。全屏保留第一显示器截图；区域模式要求有效区域并对全屏图像裁剪；窗口模式在 Windows 使用 `GetForegroundWindow` 与 `GetWindowRect`，并裁剪所得图像。捕获文件使用 `new_asset_id`，资产记录 `capture_mode`。

`import_files` 读取源文件的创建时间作为 `created_at`，回退为导入时刻。`create_asset` 只在 debug 构建注册。每个成功的导入资产及成功截图均发出 `asset-created`，负载为资产本身。

## 前端

新增共享 `MonthPicker`，持有可独立切换的年份并向调用者返回 `{ year, month }`。魔法书和传统图库都使用它。伴侣菜单接收导入回调；窗口组件使用 Tauri 文件选择器并在截图错误时显示 `role=alert`。

两个资料库视图监听 `asset-created`。只有事件资产的 `createdAt` 月份等于当前显示月份时，才递增刷新版本并重新请求数据；卸载时取消监听。

## 验证

Rust 覆盖 v1 到 v2 迁移、文件时间、唯一捕获命名和三种截图分支。Vitest 覆盖年份月份选择、两种视图的月份导航、导入点击、错误状态和匹配/月不匹配事件刷新。命令测试覆盖 `capture(mode, region)` 与 `import_files` 的成功事件。
