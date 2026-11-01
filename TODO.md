Option/null: make null the single wire meaning of "absent" for AppEvent/AppTask row fields.

- src-tauri/src/features/calendar/domain/event.rs and task.rs: drop #[ts(optional)] from the Option<T> row fields (rrule, exdates, recurring_event_id, status, color, etag, is_all_day, timezone, remote_collection_id, ...). Keep it only where a field is a genuinely omittable input, and audit those.
- Run cargo nextest run in src-tauri to regenerate src/lib/bindings/*.generated.ts. Expect rrule: string | null instead of rrule?: string.
- src/lib/domain/events.ts: isRecurring takes readonly rrule: string | null / recurringEventId: string | null and tests !== null. Drop the "string | null" union on optional fields.
- Fix call sites that bun run check breaks (buildNewEvent in src/lib/domain/events.ts, plan/sidebar drafts, test factories) by passing null explicitly.
- src/components/inspector-pane/InspectorPane.test.ts: keep the null-recurrence case; drop the undefined case if that shape is no longer reachable from the backend.
- Re-run bun run check, bun run lint, bun run test:unit, bun run test.

Not doing: skip_serializing_if on AppEvent. AppEvent is also the create_event/update_event input and the ics writer treats None remote_collection_id as "default collection", so absent and null must stay interchangeable on the wire.
