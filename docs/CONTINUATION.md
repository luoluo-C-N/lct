# Magic Image Library — 开发接力说明

## 当前快照

- 工作分支：`feat/magic-image-library`
- 最近功能：本地资源的月/日查询、魔法书与传统图库展示、预览文件协议与跨 Rust/TypeScript 的 IPC 字段契约。
- 已验证：`npm test`、`cargo test --manifest-path src-tauri/Cargo.toml`、`npm run build`、`npm run tauri -- build --debug --no-bundle`。
- 当前本地 MVP 不含账号、云同步、离线队列、真实区域/窗口截图或独立悬浮窗口；这些必须以新的规格和计划继续。

## 同一台电脑切换 GPT/Codex 账号后继续

1. 在新账号的 Codex 中打开本项目目录。
2. 先执行 `git pull --ff-only`，再读取本文件、`docs/superpowers/specs/` 与 `docs/superpowers/plans/`。
3. 执行 `git status --short --branch`，确认没有其他账号遗留的未提交改动。
4. 继续工作前运行相关测试；完成一段工作后执行 `scripts/refresh-handoff.ps1`，再提交并推送。

## 从本地离线接力包恢复

接力包位于 `handoff/magic-image-library.bundle`，该目录只保存在本机，不随 Git 推送。

```powershell
git clone C:\path\to\handoff\magic-image-library.bundle C:\path\to\new\magic-image-library
Set-Location C:\path\to\new\magic-image-library
git remote add origin https://github.com/luoluo-C-N/lct.git
git fetch origin
git switch feat/magic-image-library
```

如果新目录已有 GitHub 网络连接，随后执行 `git pull --ff-only` 获取最新提交。`handoff/manifest.json` 记录 bundle 的分支、提交和生成时间；它不存储 token、密码、代理或系统凭据。

## 刷新接力包

在工作区干净、且所有需要交接的改动都已提交后运行：

```powershell
pwsh -NoProfile -File scripts\refresh-handoff.ps1
```

脚本会生成并验证 bundle。若工作区有未提交改动，它会退出且不会覆盖已有接力包。要将文件生成到其他位置，使用：

```powershell
pwsh -NoProfile -File scripts\refresh-handoff.ps1 -OutputDirectory D:\handoff\magic-image-library
```

## 给下一位开发代理的提示

请先阅读本文件与相关规格/计划，检查 Git 状态和现有实现；不要假设会话记忆可跨账号保留。所有跨账号同步以 GitHub 提交为准，离线 bundle 仅用于本地恢复。
