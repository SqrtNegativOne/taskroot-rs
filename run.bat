@echo off
cd /d "%~dp0"
set "EXE=src-tauri\target\release\taskroot.exe"
set "STAMP=src-tauri\target\release\.build-stamp"
set "CURRENT=src-tauri\target\cargo-tmp\build-state.txt"

if not exist "%EXE%" goto dev
if not exist "%STAMP%" goto dev

if not exist "src-tauri\target\cargo-tmp" mkdir "src-tauri\target\cargo-tmp"
call "%~dp0scripts\build-state.cmd" > "%CURRENT%" 2>nul
if errorlevel 1 goto dev
fc /b "%CURRENT%" "%STAMP%" >nul 2>&1
if errorlevel 1 goto dev

echo Launching the latest release build...
start "" "%EXE%"
exit /b 0

:dev
rem The linker writes its scratch file (lnk{...}.tmp) to %TEMP%, which a confined
rem dev run cannot write, so the build dies at link time with LNK1104. Keep the
rem toolchain's scratch space inside the repo instead.
set "TMP=%~dp0src-tauri\target\cargo-tmp"
set "TEMP=%TMP%"
if not exist "%TMP%" mkdir "%TMP%"
echo No up-to-date release build. Launching the development server...
echo (Run build.bat afterwards for instant launches.)
taskkill /IM taskroot.exe /F >nul 2>&1
bun run tauri dev
