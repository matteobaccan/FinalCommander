@echo off
rem Windows has had `clip` since 2003, so this half is short.
setlocal enabledelayedexpansion
if "%SCMD_CURSOR%"=="" if "%SCMD_SELECTED%"=="" (
  echo {"status":"nothing under the cursor"}
  exit /b 0
)
echo %SCMD_COMMAND% | find ".path" >nul
if not errorlevel 1 (
  echo %SCMD_DIR%\%SCMD_CURSOR% | clip
  echo {"status":"path copied"}
) else (
  echo %SCMD_CURSOR% | clip
  echo {"status":"name copied"}
)
