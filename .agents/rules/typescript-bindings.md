---
name: typescript-bindings
description: Use this rule when modifying Rust structs that are shared with the frontend, or when dealing with IPC return types.
trigger: model_decision
---

# Generated TypeScript Bindings Flow

The project uses `ts-rs` to automatically generate TypeScript definitions from Rust structs. 
This ensures our Tauri IPC commands remain perfectly typed across the backend/frontend boundary.

## How to generate bindings:
Whenever you change a Rust struct annotated with `#[derive(TS)]`:
1. Navigate to the `src-tauri/` directory.
2. Run `cargo test` (this triggers the `export_bindings_*` tests which emit the files).
3. The generated files will be written to `src/lib/bindings/*.generated.ts`.
4. **Important**: Always commit the regenerated files alongside your Rust changes. CI will fail on binding drift (`git diff --exit-code src/lib/bindings`).

## Conventions:
- **Never hand-edit** anything in `src/lib/bindings/`.
- Use the `#[ts(type = "number")]` macro attribute on Rust `i64` timestamps to keep them as JavaScript `number`s on the frontend, rather than `bigint`.
- Types are re-exported in the `src/lib/domain.ts` barrel file for cleaner imports.
