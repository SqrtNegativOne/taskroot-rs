@echo off
rem The linker writes its scratch file (lnk{...}.tmp) to %TEMP%, which a confined
rem dev run cannot write, so the build dies at link time with LNK1104. Keep the
rem toolchain's scratch space inside the repo instead.
set "TMP=%~dp0src-tauri\target\cargo-tmp"
set "TEMP=%TMP%"
if not exist "%TMP%" mkdir "%TMP%"
bun run tauri dev
