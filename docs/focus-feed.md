# Focus feed

The focus feed lets another local application read the task currently tracked in
Taskroot (the task shown by the mini tracker and the Do screen). It is intended
for tools such as distraction blockers that need to know what the user is
working on.

## Enable it

Open **Settings -> Integrations -> Focus feed**:

- **Enable Focus Feed** turns the local server on.
- **Focus Feed Port** selects the loopback port (default `8765`).

Changes apply after Taskroot restarts. The server binds to `127.0.0.1` only; it
is not reachable from other machines.

## Endpoints

### `GET /focus`

Returns one JSON snapshot:

```json
{
  "taskId": "4d2f0c1e-…",
  "title": "Learn Python",
  "label": "Learn Python",
  "labelSource": "task",
  "phase": "running",
  "active": true,
  "tags": ["learning", "python"],
  "list": "Tasks"
}
```

| Field | Meaning |
| --- | --- |
| `taskId` | The tracked task's iCalendar `UID`, or `null`. |
| `title` | The task's current title from the calendar store. Falls back to `label` when the task is not loaded. |
| `label` | The label shown by the mini tracker (a task title or a custom string). |
| `labelSource` | `task`, `custom` or `unset`. |
| `phase` | `paused`, `running`, `breaking` or `stopped`. |
| `active` | `true` only while work is ticking (not paused and not on a break). |
| `tags` | The task's tag names, or an empty array. |
| `list` | The task list's display name, when known. |

`title`, `tags` and `list` are read from the live store, so renaming a task is
reflected without reselecting it. The snapshot is rebuilt every 500 ms.

### `GET /focus/events`

A [server-sent events](https://html.spec.whatwg.org/multipage/server-sent-events.html)
stream. The first event is the current snapshot; a new `focus` event is sent
whenever the snapshot changes. A comment frame is sent every 15 seconds to keep
the connection alive.

```
event: focus
data: {"taskId":"4d2f0c1e-…","title":"Learn Python", …}

```

### `GET /health`

Returns `{"ok":true}`.

## Example

```powershell
Invoke-RestMethod http://127.0.0.1:8765/focus
```

```bash
curl http://127.0.0.1:8765/focus
curl -N http://127.0.0.1:8765/focus/events
```

A blocker can treat `active: true` as "enforce the current `title`/`tags`" and
stop enforcing while `active` is `false` (paused or on a break). Poll `/focus`
every few seconds, or keep one `/focus/events` connection open.

## Notes

- The feed has no authentication and does not send CORS headers. It is meant for
  native local applications; a web page cannot read it cross-origin.
- The server runs for the lifetime of the process. To move it to another port,
  change the setting and restart Taskroot.
- No task data is written to the calendar by the feed; it is read-only.
