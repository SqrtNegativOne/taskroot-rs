# Windows development and troubleshooting

Implemented Windows mitigations and conditional diagnostics. Inspect actual
symptoms and logs before changing permissions, integrity labels or build flags.

## Data directories

`src-tauri/src/platform/data_dir.rs` resolves the app data directory once per
process:

1. `TASKROOT_DATA_DIR`, if set (errors if it cannot be created).
2. The platform app-data directory (`%APPDATA%\com.arkma.taskroot` on Windows).
3. In **debug** builds, if the platform directory cannot be created or fails a
   write probe, fallback to `<exe dir>/dev-data`, best-effort seeding missing
   `account.json`, `settings.json` and `ui-state.json` without overwriting
   existing files.

The CLI has no automatic debug fallback; set `TASKROOT_DATA_DIR` explicitly if
the app and CLI need to share a fallback directory.

`configure_dev_webview_storage` independently redirects `WEBVIEW2_USER_DATA_FOLDER`
to `<exe dir>/dev-data/EBWebView` in debug builds when `%LOCALAPPDATA%\<id>` is
unwritable (respecting an explicit `WEBVIEW2_USER_DATA_FOLDER`).

## Temporary directories

`build.bat` and the dev branch of `run.bat` point `TMP` and `TEMP` at
`src-tauri/target/cargo-tmp` so confined MSVC linkers can create `lnk*.tmp`.

If tests fail with temp-directory access errors in a confined Windows/MSYS
shell, set both variables separately (Node on Windows reads `TEMP`; `TMPDIR`
alone is not enough):

```sh
T="$PWD/src-tauri/target/vitest-tmp"
mkdir -p "$T"
export TMP="$T"
export TEMP="$T"
bun run test:unit
# Rust: cd src-tauri && cargo nextest run
```

## Release selection

`build.bat` builds the release binary and writes a content-safe fingerprint to
`src-tauri/target/release/.build-stamp` via `scripts/build-state.cmd` (`HEAD` +
tracked diff hash + untracked file hashes under build inputs). It skips writing
the stamp if sources changed mid-build. `run.bat` compares the current
fingerprint against `.build-stamp` and launches the release binary on a match,
otherwise kills `taskroot.exe` and starts `bun run tauri dev`. Keep
`BUILD_PATHS` in `scripts/build-state.cmd` aligned with build inputs.

## Lazy window creation

Tauri/WebView2 deadlocks on Windows if a window is created inside a synchronous
IPC command or event callback
([wry#583](https://github.com/tauri-apps/wry/issues/583)). Because the `effect`,
`launcher` and `minitracker` windows are created lazily (e.g., on the first
break transition):

- `platform/window_creation.rs` runs native creation on a blocking worker.
- `platform/window_lifecycle.rs` serializes create/show/hide/toggle per window
  with async locks so overlapping hides wait for creation and duplicate shows
  reuse one window.
- Commands and shortcut handlers that open lazy windows must stay asynchronous.

Unit tests cover the async lifecycle state machine, not WebView2 itself; verify
first-break, first-launcher and first mini-tracker opens manually on Windows.

## Tray, z-order and PowerShell

- The mini tracker's Windows topmost guard and DWM square-corner setup live in
  `src-tauri/src/platform/windows.rs`.
- If the tray icon is missing alongside access errors, inspect labels
  non-destructively with `icacls .` and
  `icacls src-tauri\target\debug\taskroot.exe` before taking action.
- In PowerShell, `-replace` is case-insensitive; use `-creplace` for bulk source
  edits so identifier casing is preserved.
