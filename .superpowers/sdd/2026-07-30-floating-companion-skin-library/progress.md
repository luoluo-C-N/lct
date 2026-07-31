# SDD ledger — plan: docs/superpowers/plans/2026-07-30-floating-companion-skin-library.md
Preflight: isolated linked worktree confirmed; plan conflict scan clean
Baseline: npm test 31/31; cargo test 14/14
Task 1: fix round 1/5 (2 addressed, 0 open; commits 2f33255..6e0fc7c)
Task 1: complete (commits cc52608..6e0fc7c, review clean)
Task 2: minor (deferred): center-crop test only asserts output dimensions, not selected pixels
Task 2: minor (deferred): skin ID generation panics if system clock predates Unix epoch
Task 2: decision required: plan mandates image::open before dimension validation, reviewer requires decode-time limits against compressed image bombs
Task 2: ruling: user approved reviewer requirement; enforce width/height/allocation limits during decode
Task 2: fix round 1/5 (2 addressed, 1 open; commits d01d2b2..fd52da7 — permanent compensation failure coverage remained)
Task 2: fix round 2/5 (1 addressed, 0 open; commits fd52da7..5d59ce0)
Task 2: complete (commits 6e0fc7c..5d59ce0, review clean; 2 deferred minors)
Task 3: dependency correction: Task 1 exposed Builtin/Imported; Task 3 will align source contract to Builtin/Image/Package and accept legacy "imported" as Image
Task 3: complete (commit c22acd1; validate-before-write ZIP import and malicious archive coverage)
Task 4: complete (commits d8830da..8b3c197; IPC, events, window lifecycle, managed mutation security)
Task 5: complete (commit 27bf38f; typed client and independent entry routing)
Task 6: complete (commit 7eddc33; main-window skin library)
Task 7: complete (commits d73f0c9..0ba7284; animated companion, drag, focus, visibility, startup retry)
Task 8 review: fixed foreground-window capture, production repository registration, real region overlay, and skin edit refresh race
Task 8 verification: cargo fmt clean; Vitest 61/61; Rust 71/71; frontend and Tauri debug builds pass
Task 8 Windows smoke: independent windows, asset loading, quick switch, focus restoration, visibility persistence, and region overlay Escape verified
Task 8 manual checks: real region drag-save, all capture modes, drag-restart placement, reduced motion, close paths, and native import/package dialogs
Task 8: complete through implementation commit c89e85b; handoff documentation and sync follow
