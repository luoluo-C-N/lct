# 跨账号开发接力设计

## 目标

让同一台电脑上的不同 GPT/Codex 账号能够从同一项目状态继续开发，不依赖会话历史。每个账号都应能读取当前进度、从 GitHub 更新代码，或在网络不可用时从本地 Git bundle 恢复完整提交历史。

## 方案

版本库中维护两份可同步材料：

- `docs/CONTINUATION.md`：面向下一位开发代理的当前状态、分支、验证命令、接力步骤和已知事项。
- `scripts/refresh-handoff.ps1`：从当前 Git 仓库生成接力 bundle，并写入本地元数据。

本地 `handoff/` 目录被 Git 忽略，其中包含 `magic-image-library.bundle` 与 `manifest.json`。bundle 打包项目本地分支和标签及其完整可达历史，但排除 Codex 内部 checkpoint refs；它不包含 `node_modules`、构建产物、凭据或未追踪文件。

## 使用流程

有 GitHub 网络连接时，账号切换后打开已有项目目录，运行 `git pull --ff-only`，读取 `docs/CONTINUATION.md`，并继续当前分支。

需要在新目录恢复或网络不可用时，复制 `handoff/magic-image-library.bundle`，执行 `git clone <bundle 路径> <新目录>`，进入新目录后读取 `docs/CONTINUATION.md`。若可联网，再将 GitHub 作为 `origin` 并拉取更新。

每次准备交接前运行 `scripts/refresh-handoff.ps1`。脚本会拒绝在存在未提交改动时生成 bundle，避免把“已同步快照”与未保存工作混淆；它将当前分支、HEAD、生成时间和远端 URL 写入 manifest。

## 安全与非目标

接力文件不得存储 GitHub token、代理地址、用户名密码或 Windows 凭据。脚本不修改远端、不执行 push、不安装依赖，也不复制大型依赖目录。它不是跨账号自动同步服务；GitHub 仍是联网时的同步源，bundle 是离线接力快照。

## 验证

PowerShell 验证脚本在干净工作区生成 bundle 与 manifest；`git bundle verify` 必须通过。测试还应确认 manifest 不包含敏感字段，且有未提交改动时脚本失败并不覆盖现有 bundle。
