@echo off
cd /d "%~dp0"
rem The linker writes its scratch file (lnk{...}.tmp) to %TEMP%, which a confined
rem build cannot write, so the build dies at link time with LNK1104. Keep the
rem toolchain's scratch space inside the repo instead.
set "TMP=%~dp0src-tauri\target\cargo-tmp"
set "TEMP=%TMP%"
if not exist "%TMP%" mkdir "%TMP%"

set "STAMP=%~dp0src-tauri\target\release\.build-stamp"
set "BEFORE=%TMP%\build-state-before.txt"
set "AFTER=%TMP%\build-state-after.txt"
del /q "%STAMP%" >nul 2>&1
call "%~dp0scripts\build-state.cmd" > "%BEFORE%"
set "HAS_BEFORE=%ERRORLEVEL%"

bun run tauri build
if errorlevel 1 exit /b 1

rem Record the build-state fingerprint only if sources did not change mid-build.
if not "%HAS_BEFORE%"=="0" exit /b 0
call "%~dp0scripts\build-state.cmd" > "%AFTER%"
if errorlevel 1 exit /b 0
fc /b "%BEFORE%" "%AFTER%" >nul 2>&1
if errorlevel 1 exit /b 0
copy /y "%AFTER%" "%STAMP%" >nul
exit /b 0
