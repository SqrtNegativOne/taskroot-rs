# Known limitations and follow-ups

Current bugs and gaps — **not** architectural rules to preserve. Verify against
code and write behavioral regression tests before changing backend behavior;
remove entries once resolved.

1. **`EventScope::All` reschedule does not resolve exception ids.** Unlike
   `delete_event`, `reschedule_event` updates the selected exception row instead
   of its master when called with an exception id and `all`. Resolving series
   identity consistently also requires defining how date shifts and existing
   overrides behave. Start at
   [events.rs](../src-tauri/src/features/calendar/store/ops/events.rs).

2. **Cross-collection moves and scoped deletes lack conflict retry / rollback.**
   Moves `PUT` to the destination before `DELETE`ing the source (a delete
   failure can leave a remote duplicate), and neither moves nor scoped deletes
   retry `412 Conflict` the way same-collection updates do. Start at
   [store/ops/](../src-tauri/src/features/calendar/store/ops/).
