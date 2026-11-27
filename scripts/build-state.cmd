@echo off
setlocal
rem Prints a content-safe fingerprint of the build-relevant working tree:
rem HEAD commit, hash of tracked diffs under build paths, and hashes of any
rem untracked files under those paths. Keep BUILD_PATHS in sync with what the
rem release build consumes.
cd /d "%~dp0.."
set "BUILD_PATHS=src static src-tauri/src src-tauri/taskroot-macros src-tauri/capabilities src-tauri/icons src-tauri/build.rs src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json .cargo package.json bun.lock tsconfig.json vite.config.js svelte.config.js icon.svg"

git rev-parse HEAD || exit /b 1
git diff HEAD -- %BUILD_PATHS% 2>nul | git hash-object --stdin || exit /b 1
for /f "delims=" %%F in ('git ls-files --others --exclude-standard -- %BUILD_PATHS%') do (
    echo %%F
    git hash-object "%%F" || exit /b 1
)
exit /b 0
