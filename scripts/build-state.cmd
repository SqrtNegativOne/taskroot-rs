@echo off
rem Prints a stable fingerprint of the build-relevant working tree: the checked-out
rem commit plus any uncommitted or untracked changes under the paths below.
rem build.bat stores this, and run.bat regenerates it and byte-compares the two, so
rem the freshness check costs a single git call instead of stat-ing every source
rem file. Keep this list in sync with what the release build actually consumes.
cd /d "%~dp0.."
git status --porcelain=v2 --branch -- src static src-tauri/src src-tauri/taskroot-macros src-tauri/capabilities src-tauri/icons src-tauri/build.rs src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json .cargo package.json bun.lock tsconfig.json vite.config.js svelte.config.js icon.svg | findstr /v /b /c:"# branch.head" /c:"# branch.upstream" /c:"# branch.ab"
