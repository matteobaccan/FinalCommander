@echo off
rem The Windows half: `tasklist` for the list, and `openfiles` or Handle for the second
rem question — which needs administrator rights on Windows, so it says so instead of failing.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0procs.ps1"
