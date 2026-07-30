# Magic Image Library — 开发接力说明

更新日期：2026-07-31

项目状态：**本地 MVP 已完成；悬浮伴侣皮肤库完成 Task 1–2 后暂停开发，等待下一账号接手。**

## 项目现状

- 仓库：`https://github.com/luoluo-C-N/lct.git`
- 当前分支：`feat/magic-image-library`
- 本地 MVP 收尾提交：`90141e0`（`fix: address final core bug review`）
- 皮肤库最新实现提交：`5d59ce0`（`test: cover skin import compensation failures`）
- 最终交接提交：请在拉取后执行 `git log -1 --oneline --decorate` 查看；本文件随该提交一起推送。
- 技术栈：Tauri 2、Rust、rusqlite、screenshots、image、React 19、TypeScript、Vite、Vitest。
- 数据闭环：导入/截图后保存原图与预览、写入本地 SQLite、发布 `asset-created`，魔法书与传统图库按年月刷新。
- 本地数据位于 Tauri 的 `$APPLOCALDATA`：数据库为 `assets.sqlite3`，资源位于 `assets/`；前端只通过 asset protocol 暴露 `assets/previews/`。

## 已完成事项

- SQLite v2 资产结构和旧库迁移。
- 图片导入；保留源文件创建时间；失败时清理未完成文件。
- 全屏、区域、窗口三种截图模式及裁剪区域持久化。
- Release 环境关闭仅供开发使用的资产创建 IPC。
- 导入/截图成功后发布完整 `asset-created` 事件。
- 混合导入批次按文件持久化并即时发布事件：后续文件失败时，先前成功资产不会漏事件。
- 魔法书和传统图库的年份/月导航、共用月份选择器、跨视图事件刷新。
- 月份选择器使用原生 modal dialog，包含焦点进入、Escape/cancel 关闭、焦点恢复和 ARIA 关联。
- 伴侣入口的文件选择、取消、成功导入和失败提示。
- Tauri dialog 插件、权限和前后端调用链。
- 当前计划的自动化测试、生产前端构建及 Tauri debug 构建。

### 悬浮伴侣皮肤库已完成

- Task 1：伴侣皮肤领域模型、SQLite v3 迁移、三个内置皮肤、皮肤与窗口设置持久化。
- Task 2：本地 PNG/WebP 图片皮肤导入、尺寸与内存上限、居中裁剪、纹理/预览生成、安全配色提取。
- 内置皮肤、当前启用皮肤与导入回滚保护。
- 文件写入、数据库写入和重命名失败时的补偿与重试测试。
- Task 3 的精确 `zip = 8.6.0` 依赖已加入并锁定；ZIP 正常包与恶意归档 RED 测试已有未完成草稿，但测试辅助代码和生产实现均未完成。

对应规格与计划：

- `docs/superpowers/specs/2026-07-30-floating-companion-skin-library-design.md`
- `docs/superpowers/plans/2026-07-30-floating-companion-skin-library.md`

详细执行记录位于本机忽略目录：

- `.superpowers/sdd/2026-07-26-core-bug-fixes/task-3-report.md`
- `.superpowers/sdd/2026-07-26-core-bug-fixes/task-4-report.md`
- `.superpowers/sdd/2026-07-26-core-bug-fixes/task-5-report.md`
- `.superpowers/sdd/2026-07-26-core-bug-fixes/final-fix-report.md`

## 未完成事项

本次暂停时，悬浮伴侣皮肤库仍有以下计划内工作：

1. Task 3：完成当前 RED 测试草稿及其辅助代码，再实现版本化 ZIP 皮肤包的恶意归档防护、完整校验与原子导入。
2. Task 4：皮肤 IPC、跨窗口事件、独立悬浮窗口配置与生命周期。
3. Task 5：TypeScript 客户端契约、窗口入口路由、移除主窗口内嵌伴侣。
4. Task 6：主窗口皮肤库、选择/删除/导入、动效参数控制。
5. Task 7：悬浮伴侣交互、三套动画皮肤、拖动/吸边/缩放与 reduced-motion。
6. Task 8：完整验证、真实 Windows 冒烟、最终交接。

以下仍不属于当前本地 MVP/皮肤库计划，接手后不要在没有新规格/计划时直接扩展：

- 账号系统、云同步、离线队列和冲突处理。
- 产品级标注编辑流程。
- 安装包、签名、发布渠道与自动更新。
- 真实 Windows WebView2 中的完整人工无障碍/键盘巡检。

两项已知但不阻断本地 MVP 的测试债务：

- 前端 `Asset` 类型尚未补齐 Rust v2 payload 的全部字段；当前事件消费者只读取已声明字段。
- capture 命令测试尚未显式断言 region forwarding，以及截图失败时零事件。

## 测试命令与暂停前结果

在 2026-07-31、Task 3 RED 测试草稿写入之前的已提交基线 `5d59ce0`，以及只加入 ZIP 依赖的工作树上，曾通过：

```powershell
npm test
# 6 test files passed; 31 tests passed

cargo test --manifest-path src-tauri/Cargo.toml
# 39 passed; 0 failed

npm run build
# TypeScript + Vite production build succeeded

npm run tauri -- build --debug --no-bundle
# succeeded; output: src-tauri/target/debug/magic-image-library.exe

git diff --check
# exit 0
```

构建仍会提示既有的 `com.magicimagelibrary.app` 以 `.app` 结尾；这是面向 macOS bundle identifier 的非阻断建议，本项目当前目标为 Windows。构建还会产生 43 条皮肤库相关 `dead_code` 警告，因为 Task 1–2 的 Rust 模块要到 Task 4 接入 IPC 后才进入生产调用链。

暂停时保留了 Task 3 的未提交 RED 测试草稿。当前 HEAD 加工作树运行 `cargo test --manifest-path src-tauri/Cargo.toml --no-run` 会编译失败（105 errors），主要因为测试已引用但生产代码尚未提供 `SkinSource::Image`、`SkinSource::Package`、`import_zip_skin`、`SkinImportErrorKind`，同时 ZIP fixture helpers 也尚未补齐。这是明确的 WIP 状态，不代表上述已提交基线回归。

## 开发环境与启动步骤

暂停时使用的环境：

- Windows / PowerShell
- Node.js `v24.18.0`
- npm `11.16.0`
- rustc `1.97.1`
- cargo `1.97.1`
- Tauri Windows 构建依赖：Microsoft C++ Build Tools 与 WebView2 Runtime

新副本首次启动：

```powershell
git clone https://github.com/luoluo-C-N/lct.git C:\path\to\magic-image-library
Set-Location C:\path\to\magic-image-library
git switch feat/magic-image-library
git pull --ff-only
npm install
npm run tauri -- dev
```

只调试静态前端可用 `npm run dev`，但导入、截图、文件对话框和本地数据库依赖 Tauri runtime，应以 `npm run tauri -- dev` 为准。

## 关键设计决策

- 本轮边界是本地优先 MVP；云同步字段只保留数据契约，不实现远端行为。
- Rust 负责文件复制、预览生成、截图和 SQLite；React 通过受限 Tauri commands/events 使用这些能力。
- 预览可以通过 asset protocol 展示，原图目录不直接暴露给前端。
- `asset-created` 在单个资产成功持久化后立即发送，而不是等待整个批次成功，保证部分成功批次的 UI/磁盘/数据库一致。
- 两个图库视图只在事件年月匹配当前视图时刷新，并对过期请求和延迟完成的监听注册做失效清理。
- MonthPicker 通过 portal + 原生 `showModal()` 避免侧栏裁剪，并保持键盘焦点生命周期。

## 已知问题

- 若资产已持久化后 Tauri event emit 本身失败，命令会返回错误，但不会回滚磁盘与 SQLite；本轮没有引入跨存储和事件系统的事务。
- 未在真实 Windows WebView2 窗口手工执行 MonthPicker 焦点与 Escape 行为巡检。
- 上述两项测试债务仍在，但不影响当前使用路径。
- `.app` bundle identifier 构建警告尚未处理。
- 皮肤库 Task 1–2 目前只有 Rust 测试覆盖，尚未通过 IPC 暴露给前端；因此用户界面暂时看不到皮肤库功能。
- Task 3 已加入并锁定 `zip 8.6.0` 依赖，并保留了未完成的 ZIP RED 测试草稿；ZIP 包解析、Zip Slip/符号链接/压缩炸弹/重复路径防护与导入事务均未实现。
- 当前 Task 3 WIP 无法编译；恢复开发时应先补齐测试辅助代码和最小契约，再按 TDD 逐项进入 GREEN，不要将暂停点当成可发布构建。
- 皮肤库新增模块在接入 IPC 前会产生 43 条 `dead_code` 构建警告。

## 下一步建议

1. 切换账号后先拉取并确认分支、HEAD 与干净工作树。
2. 阅读本文件，以及：
   - `docs/superpowers/specs/2026-07-26-core-bug-fixes-design.md`
   - `docs/superpowers/plans/2026-07-26-core-bug-fixes.md`
   - `docs/superpowers/specs/2026-07-30-floating-companion-skin-library-design.md`
   - `docs/superpowers/plans/2026-07-30-floating-companion-skin-library.md`
3. 继续皮肤库时从计划 Task 3 Step 1 的 WIP 开始：先检查 `src-tauri/src/domain/companion_test.rs`、`repository/companion_test.rs`、`services/skins_test.rs` 的未完成 RED 草稿，补齐 fixture helpers 和最小 source/error 契约；不要把已存在的依赖或测试草稿误判为 Task 3 已完成。
4. 完成 Task 3 后依次执行 Task 4–7；Task 4 接入生产调用链后再处理现有 `dead_code` 警告。
5. 最终按 Task 8 做真实 Windows 冒烟、全量验证、交接与推送。

## 同一台电脑切换 GPT/Codex 账号

1. 在新账号的 Codex 中打开此项目目录。
2. 要求代理先读取本文件、`docs/superpowers/specs/` 和 `docs/superpowers/plans/`。
3. 执行：

```powershell
git switch feat/magic-image-library
git pull --ff-only
git status --short --branch
git log -1 --oneline --decorate
```

4. 若状态不是干净的，不要覆盖、reset 或丢弃改动；先确认改动来源。
5. 不要依赖旧账号的会话记忆；以 GitHub 提交、本文件和本地 bundle 为交接依据。

## 从本地离线接力包恢复

接力包位于 `handoff/magic-image-library.bundle`，该目录只保存在本机，不随 Git 推送：

```powershell
git clone C:\path\to\handoff\magic-image-library.bundle C:\path\to\new\magic-image-library
Set-Location C:\path\to\new\magic-image-library
git remote set-url origin https://github.com/luoluo-C-N/lct.git
git fetch origin
git switch feat/magic-image-library
```

`handoff/manifest.json` 记录 bundle 的分支、HEAD、生成时间和已脱敏远端地址，不存储 token、密码、代理或系统凭据。

在工作区干净且所有交接改动已提交后刷新接力包：

```powershell
pwsh -NoProfile -File scripts\refresh-handoff.ps1
```

脚本会先验证 bundle；若工作区有未提交改动，会退出且不会覆盖已有接力包。自定义输出位置：

```powershell
pwsh -NoProfile -File scripts\refresh-handoff.ps1 -OutputDirectory D:\handoff\magic-image-library
```
