# Reminders feature

- `policy.rs` — pure decision logic: which task `VALARM` triggers fall inside the
  missed window at a given instant.
- `tauri.rs` — the adapter that polls the calendar store through its managed
  `StoreState` and raises desktop notifications.

Reminders are read-only: they live in the VTODO as `VALARM` triggers and are
never written here. `bootstrap` starts the poller; this feature never imports
`ipc` or `bootstrap`.
