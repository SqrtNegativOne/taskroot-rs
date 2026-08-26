# Frontend Architecture & Rules

This file contains rules specific to the Svelte 5 frontend of Taskroot. It supplements the root `AGENTS.md`.

## Key Concepts
- **State Management**: Frontend state is managed using Svelte 5 Runes (`$state`, `$derived`, `$effect`, `$props`). The primary store is located in `src/lib/store.svelte.ts`, which syncs with the Rust backend via Tauri IPC (`invoke` wrapped in `safeInvoke`).
- **SQLite Source of Truth**: Crucially, the frontend relies strictly on the SQLite backend as the source of truth. It avoids complex optimistic patching arrays locally. Mutations return `neverthrow` `Result`s, `await` the backend command, and then instantly re-fetch the raw state from the database. This prevents race conditions and UI pop-backs, as local SQLite queries return in ~1-3ms.
- **Local-Date Rule**: All day bucketing/comparison must go through `src/lib/time.ts` (`ymd`, `addDays`, `dayDiff`, `sameDay`), which operate on local date parts. `Date.toISOString()` is UTC-shifted and must not be used to derive a calendar day.

## Style & Idioms
- **Svelte 5 Idioms**: Strictly use Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`) instead of legacy Svelte 4 reactivity (`let foo = ...`, `$:`, `export let`).
- **Typescript Idioms**: Strongly type your code. Never use `any`, `unknown` casts, or loose interfaces (like `Record<string, unknown>`) as quick hacks to bypass ESLint, Oxlint, or TypeScript compiler errors. If fixing a type warning requires a larger architectural change—such as utilizing Svelte 5 component `generics="T"` to preserve end-to-end type safety for generic UI components—you must do the thorough refactor rather than applying a bandage solution.
- **Frontend Error Handling (`neverthrow`)**: Use the `neverthrow` library to handle errors functionally on the frontend, mirroring the Rust backend's `Result` type. Do not use standard `try/catch` for expected errors. When calling Tauri's `invoke`, use the `safeInvoke` wrapper (or `useTauriQuery` rune) located in `src/lib/safeInvoke.svelte.ts` to ensure type-safe `ResultAsync` returns.
- **Routing**: Never use magic strings for route paths (e.g., `goto('/login')`). Always import and use the centralized constants from `src/lib/routes.ts` (e.g., `goto(Routes.LOGIN)`).
