@echo off
rem Windows has had `clip` since 2003, so this half is short.
setlocal enabledelayedexpansion
if "%FCMD_CURSOR%"=="" if "%FCMD_SELECTED%"=="" (
  echo {"status":"nothing under the cursor"}
  exit /b 0
)
echo %FCMD_COMMAND% | find ".path" >nul
if not errorlevel 1 (
  echo %FCMD_DIR%\%FCMD_CURSOR% | clip
  echo {"status":"path copied"}
) else (
  echo %FCMD_CURSOR% | clip
  echo {"status":"name copied"}
)
