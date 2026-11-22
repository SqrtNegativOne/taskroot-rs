@echo off
cd /d "%~dp0"
rem The linker writes its scratch file (lnk{...}.tmp) to %TEMP%, which a confined
rem build cannot write, so the build dies at link time with LNK1104. Keep the
rem toolchain's scratch space inside the repo instead.
set "TMP=%~dp0src-tauri\target\cargo-tmp"
set "TEMP=%TMP%"
if not exist "%TMP%" mkdir "%TMP%"

bun run tauri build
if errorlevel 1 exit /b 1

rem Record the exact git state this release was built from so run.bat can tell,
rem in milliseconds, whether the release exe still matches the sources.
set "STAMP=%~dp0src-tauri\target\release\.build-stamp"
call "%~dp0scripts\build-state.cmd" > "%STAMP%"
if errorlevel 1 del /q "%STAMP%" >nul 2>&1
exit /b 0
