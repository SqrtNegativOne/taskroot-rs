---
name: typescript-bindings
description: Use this rule when modifying Rust structs shared with the frontend or IPC return types.
trigger: model_decision
---

# Generated TypeScript bindings

Follow [AGENTS.md](../../AGENTS.md#working-rules) and
[src-tauri/AGENTS.md](../../src-tauri/AGENTS.md#calendar-settings-and-generated-types).

Keep timestamps as JS `number` (`#[ts(type = "number")]`), run
`cargo nextest run` in `src-tauri/` after `#[derive(TS)]` changes, and commit
regenerated `src/lib/bindings/*.generated.ts`. Never hand-edit generated files.
