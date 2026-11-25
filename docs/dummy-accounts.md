# Dummy CalDAV accounts for testing

A disposable loopback Radicale fixture with two independent dummy accounts, for
testing multi-account behavior without touching a real server or credentials.

## Credentials (test-only)

| Username | Password | Sees |
|---|---|---|
| `taskroot-dummy-a` | `dummy-a-only` | its own collections only |
| `taskroot-dummy-b` | `dummy-b-only` | its own collections only |

Server: `http://127.0.0.1:15232` (loopback only; the script refuses the real
port `5232` or any occupied port). Each account gets seeded `tasks` and
`events` collections built from ICS fixtures. These credentials exist for
automated/manual testing only and grant access to nothing else.

## Start and stop

```bash
# First time: create a venv under target/ and install Radicale (never global pip)
src-tauri/target/dummy-caldav/venv/Scripts/python -m pip install -r scripts/dummy-caldav-requirements.txt

# Start the server (seeds both accounts; logs to server.log under target/)
src-tauri/target/dummy-caldav/venv/Scripts/python scripts/dummy-caldav.py

# Smoke test auth, seeding and cross-account isolation, then stop
src-tauri/target/dummy-caldav/venv/Scripts/python scripts/dummy-caldav.py --check

# Run the opt-in Rust contract (ignored test) against a running server
src-tauri/target/dummy-caldav/venv/Scripts/python scripts/dummy-caldav.py --check-backend
```

All state lives under `src-tauri/target/dummy-caldav/` (git-ignored); delete
that folder to reset. Connect the profiles from Settings → Account using the
credentials above — or set `TASKROOT_CALDAV_PASSWORD` for headless CLI use.
