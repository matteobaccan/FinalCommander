@echo off
rem The Windows half of status.sh. Batch rather than PowerShell because this is three lines
rem of work, and a `.cmd` is something the host can run directly.
setlocal
if "%FCMD_DIR%"=="" (
  echo {"error":"not a directory on this machine"}
  exit /b 0
)
cd /d "%FCMD_DIR%" 2>nul || (
  echo {"error":"cannot enter that directory"}
  exit /b 0
)
for /f "delims=" %%b in ('git rev-parse --abbrev-ref HEAD 2^>nul') do set BRANCH=%%b
if "%BRANCH%"=="" (
  echo {"status":"not a git repository"}
  exit /b 0
)
set COUNT=0
for /f "delims=" %%f in ('git status --porcelain 2^>nul') do set /a COUNT+=1
echo {"status":"on %BRANCH%, %COUNT% file(s) changed"}
