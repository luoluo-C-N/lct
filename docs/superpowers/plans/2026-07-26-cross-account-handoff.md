# Cross-Account Handoff Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a repeatable local Git bundle handoff so a different GPT/Codex account can resume this project from shared files.

**Architecture:** A tracked continuation document and a PowerShell refresh script describe and create the handoff. Generated bundle and manifest files live under a Git-ignored directory and are reproducible from the current repository state.

**Tech Stack:** Git, PowerShell, Markdown.

## Global Constraints

- Do not store credentials, tokens, proxy values, or untracked application data in the handoff files.
- Generated handoff artifacts must be Git ignored; instructions and refresh script must be versioned.
- Bundle creation must refuse a dirty worktree and must not alter remotes or push.

---

### Task 1: Add Continuation Guide and Refresh Script

**Files:**
- Create: `docs/CONTINUATION.md`
- Create: `scripts/refresh-handoff.ps1`
- Modify: `.gitignore`
- Create: `scripts/refresh-handoff.test.ps1`

**Interfaces:**
- Produces: `scripts/refresh-handoff.ps1 [-OutputDirectory <path>]`, which writes `magic-image-library.bundle` and `manifest.json`.

- [ ] **Step 1: Write a failing PowerShell test**

```powershell
& $script -OutputDirectory $output
$LASTEXITCODE | Should -Be 0
Test-Path (Join-Path $output 'magic-image-library.bundle') | Should -BeTrue
git bundle verify (Join-Path $output 'magic-image-library.bundle') | Should -Not -Match 'error'
```

- [ ] **Step 2: Run the test and confirm it fails**

Run: `pwsh -File scripts/refresh-handoff.test.ps1`

Expected: FAIL because the refresh script does not exist.

- [ ] **Step 3: Implement the guide, ignore rule, and script**

The guide must tell a new account how to continue in-place, restore from a bundle, refresh the handoff, and reconnect GitHub. The script must require a clean tree, create the output directory, use `git bundle create <bundle> --all`, verify it, and serialize only branch, HEAD, timestamp, and sanitized remote URL to manifest.

- [ ] **Step 4: Run verification and commit**

Run: `pwsh -File scripts/refresh-handoff.test.ps1 && git status --short`

Expected: test passes; generated `handoff/` files are absent from Git status.

Commit: `feat: add cross-account development handoff`
