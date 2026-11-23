# Windows development and troubleshooting

These notes describe implemented mitigations and conditional diagnostics.
Sandbox access depends on the current harness policy; old observations about
executable locations, timings or integrity labels are not universal guarantees.
Do not copy binaries, change ACLs or alter real user data to reproduce them.

## Data directories

`src-tauri/src/platform/data_dir.rs` resolves app data:

1. `TASKROOT_DATA_DIR` wins when set.
2. Otherwise debug/release use the normal platform directory
   (`%APPDATA%\com.arkma.taskroot` on Windows).
3. In **debug** builds, a failed write probe can select `<exe dir>/dev-data`,
   copying currently missing `account.json`, `settings.json` and `ui-state.json`
   best-effort from the normal directory. Existing files are not overwritten;
   stopwatch focus/history are not copied. This is not calendar-data caching or
   a schema migration.

The app caches its resolved directory for the process. Creation of the preferred
directory happens before the fallback probe, so a creation failure can return an
error without falling back. The CLI has a separate resolver and no automatic
debug fallback; set `TASKROOT_DATA_DIR` explicitly when app/CLI must share a
fallback data home.

WebView2 profile-directory access has a separate debug fallback implemented in
`platform/data_dir.rs` and called before Tauri construction. It respects
`WEBVIEW2_USER_DATA_FOLDER` and is independent of `TASKROOT_DATA_DIR`.
A warning identifies the selected fallback, not successful persistence or the
reason access failed.
Distinguish sandbox policy, filesystem permissions and process integrity before
changing anything. `run.bat` does not set `TASKROOT_DATA_DIR`.

## Temporary directories

`build.bat` and the dev branch of `run.bat` set both `TMP` and `TEMP` to
`src-tauri/target/cargo-tmp`. Use a writable temp directory when a confined MSVC
linker fails opening `lnk*.tmp` at final link; compiling all objects does not
prove the linker can access its scratch files.

For tests in a confined Windows/MSYS shell (Rust tests also use system temp):

```sh
T="$PWD/src-tauri/target/vitest-tmp"
mkdir -p "$T"
export TMP="$T"
export TEMP="$T"
bun run test:unit
# Rust alternative: cd src-tauri && cargo nextest run
```

Set the two variables separately. `export TMP="$T" TEMP="$TMP"` can bind `TEMP`
to the previous `TMP` value, and Windows Node temp resolution uses `TEMP`.
Setting only `TMPDIR` is not a substitute. This workaround is conditional on
temp access failing; it is not necessary for every development environment.

## Release selection

`build.bat` builds the release and writes
`src-tauri/target/release/.build-stamp` using `scripts/build-state.cmd`.
`run.bat` compares a freshly generated stamp and launches the release executable
on a match, otherwise Tauri dev. A release built outside `build.bat` has no new
stamp, and an old stamp may remain. These scripts are launch/build actions, not
read-only checks: the dev branch of `run.bat` terminates `taskroot.exe` by image
name before launching.

**The stamp is a Git-status heuristic, not a content fingerprint.** It includes
HEAD and status records for selected build paths. Successive edits to an already
dirty file, or changes to an already-untracked file, can leave identical status
output; documentation-only changes under `src/` can also invalidate it. Do not
use a stamp match as proof that the binary contains every current edit. When in
doubt, rebuild explicitly. The helper's path list must track build inputs.

## Tray and z-order diagnostics

The mini tracker's Windows topmost guard and DWM square-corner setup live in
`src-tauri/src/platform/windows.rs`. These are implemented desktop workarounds;
code presence alone does not establish that a particular machine still needs
periodic topmost reassertion.

If the tray is missing alongside access errors, inspect labels non-destructively
with `icacls .` and `icacls src-tauri\target\debug\taskroot.exe`. A Low mandatory
label is a diagnostic clue, not proof of the process's integrity or of the cause
of a tray failure. Check actual process integrity and logs. Do not recommend
administrator mode or recursively relabel the repository without evidence and
user approval.

## Editing from PowerShell

`-replace` is case-insensitive. Use exact replacements or `-creplace` for bulk
source edits so identifier casing is not silently rewritten.
