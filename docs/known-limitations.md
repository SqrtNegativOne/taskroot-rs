# Audited limitations and follow-ups

These were identified by a read-only source audit of the working tree during the
agent-documentation cleanup. **They are bugs, coverage gaps or improvement
candidates—not architectural rules to preserve.** No runtime reproduction or
fix is claimed. Recheck the code and write behavior regressions before changing
backend behavior; remove/update entries when resolved.

## Prioritize correctness and data safety

1. **Changed mixed alarm lists can lose identity.** The ICS reader omits unsupported
   triggers, but nested writes position-match every `VALARM`. If an unsupported
   alarm precedes a supported one, editing the supported list can rewrite the wrong
   alarm and delete another. Unrelated edits preserve alarms correctly; that is a
   different guarantee. Preserve identity/unsupported components during list edits.
   Start at [alarm parsing](../src-tauri/src/features/calendar/ics/read.rs) and
   [nested editing](../src-tauri/src/features/calendar/ics/lines.rs); test a mixed
   list with distinct ACTION/DESCRIPTION values and a changed supported trigger.

2. **Recurring-task completion is not revalidated after conflict.** The initial
   guard protects a repeating master, but refetch updates raw/ETag without reparsing
   stored items, and the retry does not rerun the guard. A remote RRULE added between
   reads can therefore bypass the protection. Reparse/revalidate fresh resource
   state before re-merging. Start at
   [task updates](../src-tauri/src/features/calendar/store/ops/tasks.rs) and
   [refetch](../src-tauri/src/features/calendar/store/ops/mod.rs); test a `412`
   followed by remotely added recurrence.

3. **Collection changes bypass the load revision guard.** Collection deletion
   mutates the resource map directly, and list replacement does not bump the
   resource revision. An in-flight load can restore deleted data or overwrite list
   metadata. A detected stale revision currently returns success without applying
   or retrying. Include collection mutations in revision tracking and make skipped
   Refresh behavior deliberate. Start at
   [collection lifecycle](../src-tauri/src/features/calendar/caldav/calendars.rs),
   [Store](../src-tauri/src/features/calendar/store/model.rs) and
   [load adapter](../src-tauri/src/ipc/session.rs); test overlapping load/list edits.

4. **“All” reschedule does not resolve exception ids.** Unlike series deletion,
   rescheduling updates the selected exception rather than its master when that
   exception's id is passed with `all`. Resolve series identity consistently and
   define what happens to overrides. Start at
   [event operations](../src-tauri/src/features/calendar/store/ops/events.rs);
   test `all` from an exception, not only from the master.

5. **Write preflight and failure handling are inconsistent.** Task/event deletion
   lacks collection-writability preflight; moves validate the destination but not
   the source. Moves PUT before DELETE and can leave a remote duplicate if deletion
   fails. Ordinary same-collection updates retry a conflict once, but moves/scoped
   deletes do not. Keep conditional writes and resource preservation while deciding
   on consistent source/target checks and partial-move recovery. Start at
   [store operations](../src-tauri/src/features/calendar/store/ops/).

## Reliability and test guarantees

- **Release reuse is not content-safe.** `scripts/build-state.cmd` fingerprints Git
  status, not unstaged/untracked bytes. A second edit can leave the same stamp.
  Prefer content hashing, or the simpler policy of automatic release reuse only
  for a clean selected tree at the recorded commit. Also account for edits during
  build/stamp creation. See [Windows notes](windows-development.md#release-selection).
- **IPC source scans miss builder-return keys and required arguments.** Prefer a
  thin typed/generated command-argument contract, or parsed syntax with bounded
  helper resolution, over progressively complex text heuristics. Keep meaningful
  cross-surface checks; do not inline good builders just to satisfy a scan. See
  [testing limits](testing.md#ipc-source-contract-what-it-does-and-does-not-prove).
- **Fallback success can hide failures.** An empty cached collection list can turn
  failed discovery into a successful empty load, ending startup retries. App data
  directory creation can fail before the debug fallback probe. Test those wrapper
  paths, not only helpers. See
  [calendar loading](../src-tauri/src/features/calendar/load.rs) and
  [data resolution](../src-tauri/src/platform/data_dir.rs).
- **Launcher day range is stale after midnight.** A retained launcher captures its
  agenda range on mount. Recompute the local date when opening or when the day
  changes; test reopening without recreating the webview. Start at
  [LauncherPalette](../src/lib/launcher/LauncherPalette.svelte).

## Small simplifications, separately testable

- Centralize DOM cleanup via `@testing-library/svelte/vitest` in test setup instead
  of registering identical cleanup in every component test. Do not remove mock/
  timer resets or the Vitest browser condition.
- The pointer teardown registry has no reactive consumer; an ordinary Set can
  replace `SvelteSet` while preserving cancellation/unmount behavior.
- The mini tracker can use the engine's `currentMs` directly rather than repeat
  its pause-remaining branch. Do not remove the clock's tick/lifecycle setup.
- The “frequent recurrence” filter uses FREQ categories regardless of INTERVAL.
  Rename/explain that choice, or deliberately change the product definition;
  don't treat it as actual measured recurrence spacing.

Historical Windows sandbox/manifest/tray anecdotes are not established current
bugs. Inspect symptoms before changing ACLs, runtime signatures or build linkage.
