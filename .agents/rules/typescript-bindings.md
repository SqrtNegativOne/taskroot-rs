---
name: typescript-bindings
description: Use this rule when modifying Rust structs shared with the frontend or IPC return types.
trigger: model_decision
---

# Generated TypeScript bindings

Follow [AGENTS.md](../../AGENTS.md#working-rules) and
[src-tauri/AGENTS.md](../../src-tauri/AGENTS.md#calendar-settings-and-generated-types).

Annotate shared types with `#[derive(specta::Type)]` and IPC handlers with
`#[specta::specta]`, register commands/events/extra types in
`src-tauri/src/ipc/mod.rs`, run `cargo nextest run` in `src-tauri/` to
regenerate `src/lib/bindings/index.generated.ts`, and commit the updated output.
Never hand-edit generated files.
