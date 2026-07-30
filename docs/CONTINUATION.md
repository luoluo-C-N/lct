# Magic Image Library — 开发接力说明

更新日期：2026-07-30

项目状态：**本地 MVP 已完成当前计划并暂停开发，等待下一账号接手。**

## 项目现状

- 仓库：`https://github.com/luoluo-C-N/lct.git`
- 当前分支：`feat/magic-image-library`
- 最新实现提交：`90141e0`（`fix: address final core bug review`）
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

详细执行记录位于本机忽略目录：

- `.superpowers/sdd/2026-07-26-core-bug-fixes/task-3-report.md`
- `.superpowers/sdd/2026-07-26-core-bug-fixes/task-4-report.md`
- `.superpowers/sdd/2026-07-26-core-bug-fixes/task-5-report.md`
- `.superpowers/sdd/2026-07-26-core-bug-fixes/final-fix-report.md`

## 未完成事项

以下不属于本轮本地 MVP，接手后不要在没有新规格/计划时直接扩展：

- 账号系统、云同步、离线队列和冲突处理。
- 独立悬浮伴侣窗口及完整 Windows 桌面交互。
- 产品级标注编辑流程。
- 安装包、签名、发布渠道与自动更新。
- 真实 Windows WebView2 中的完整人工无障碍/键盘巡检。

两项已知但不阻断本地 MVP 的测试债务：

- 前端 `Asset` 类型尚未补齐 Rust v2 payload 的全部字段；当前事件消费者只读取已声明字段。
- capture 命令测试尚未显式断言 region forwarding，以及截图失败时零事件。

## 测试命令与暂停前结果

在实现提交 `90141e0` 上已通过：

```powershell
npm test
# 6 test files passed; 31 tests passed

cargo test --manifest-path src-tauri/Cargo.toml
# 14 passed; 0 failed

npm run build
# TypeScript + Vite production build succeeded

npm run tauri -- build --debug --no-bundle
# succeeded; output: src-tauri/target/debug/magic-image-library.exe

git diff --check
# exit 0
```

构建仍会提示既有的 `com.magicimagelibrary.app` 以 `.app` 结尾；这是面向 macOS bundle identifier 的非阻断建议，本项目当前目标为 Windows。

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

## 下一步建议

1. 切换账号后先拉取并确认分支、HEAD 与干净工作树。
2. 阅读本文件，以及：
   - `docs/superpowers/specs/2026-07-26-core-bug-fixes-design.md`
   - `docs/superpowers/plans/2026-07-26-core-bug-fixes.md`
3. 先做一次真实 Windows Tauri 手工冒烟：导入成功/失败、截图、月份切换、Escape 与焦点恢复。
4. 若继续本地 MVP 稳定性工作，优先补齐两项测试债务。
5. 若进入云同步、独立伴侣窗口或发布阶段，先新增规格与计划，再开始实现。

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
