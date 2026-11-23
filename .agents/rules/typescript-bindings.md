---
name: typescript-bindings
description: Use this rule when modifying Rust structs shared with the frontend or IPC return types.
trigger: model_decision
---

# Generated TypeScript bindings

Follow [the root guide](../../AGENTS.md#working-rules) and
[backend generated-type rules](../../src-tauri/AGENTS.md#settings-and-generated-types).

Run `cargo nextest run` in `src-tauri/` after shared `#[derive(TS)]` changes and
include regenerated `src/lib/bindings/*.generated.ts`. These are generated files,
not a hand-maintained or complete runtime IPC contract.
