@echo off
rem The Windows half of case.sh, through PowerShell: three lines of batch would not survive a
rem file name with a bracket in it, and PowerShell is on every Windows worth supporting.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0case.ps1"
