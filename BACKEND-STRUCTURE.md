# Backend structure

Module-level only. Each box is a folder/module, not a file.

```mermaid
graph TD
    root["lib.rs / main.rs"]

    subgraph app["app/"]
        app_life["lifecycle"]
        app_win["windows"]
        app_tray["tray"]
        app_plugins["plugins"]
    end

    ipc["ipc/"]

    subgraph features["features/"]
        subgraph cal["calendar/"]
            cal_session["session"]
            cal_dom["domain/"]
            cal_caldav["caldav/"]
            cal_ics["ics/"]
            cal_store["store/"]
            cal_sync["sync"]
            cal_occ["occurrences"]
            cal_hook["hook"]
        end
        sw["stopwatch/"]
        rem["reminders/"]
        focus["focus/ (future)"]
    end

    settings["settings/"]
    cli["cli/"]
    test_support["test_support/"]

    root --> app
    app -->|"starts runtime"| cal_sync
    app -->|"starts runtime"| cal_hook
    app -->|"starts runtime"| rem
    app -->|"starts runtime"| focus
    app -->|"starts runtime"| sw
    app -->|"registers"| ipc

    ipc --> cal_session
    ipc --> sw
    ipc --> rem
    ipc --> focus
    ipc --> settings
    ipc --> app_win

    cal_session --> cal_store
    cal_session --> cal_caldav
    cal_session --> cal_occ
    cal_session --> settings

    cal_sync --> cal_store
    cal_hook --> cal_sync
    cal_occ --> cal_dom
    cal_store --> cal_ics
    cal_store --> cal_caldav
    cal_store --> cal_dom
    cal_ics --> cal_dom
    cal_caldav --> cal_dom

    sw -.->|"reads current block"| cal_session
    rem -.-> cal_session
    focus -.->|"read-only"| cal_session

    cli --> cal_session
    cli --> settings
```

Rules encoded above:

- `calendar/session` is the only calendar-facing API for `ipc/` and `cli/`. It owns the `Store` + `CalDavClient` pairing and exposes task, event, occurrence and calendar operations.
- `ipc/` and `cli/` do not import `calendar/store` or `calendar/caldav` directly. Type-only imports are the exception.
- `calendar/domain` is the leaf; it imports nothing outside `calendar/`.
- `calendar/store` is the only writer; `sync` and `hook` call it.
- `app/` only composes: it starts feature runtimes and registers `ipc/`. It holds no feature rules.
- `cli/` never touches `app/` or `ipc/`.
- Cross-feature edges into `calendar/session` are read-only (dotted).
