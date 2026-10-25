@echo off
rem On Windows the shell itself is the opener: `start` with an empty title, which is the one
rem piece of batch trivia everybody has to look up once.
setlocal
if "%FCMD_DIR%"=="" (
  echo {"error":"not a directory on this machine"}
  exit /b 0
)
cd /d "%FCMD_DIR%" 2>nul || (
  echo {"error":"cannot enter that directory"}
  exit /b 0
)
echo %FCMD_COMMAND% | find ".folder" >nul
if not errorlevel 1 (
  start "" explorer .
  echo {"status":"handed this folder to Explorer"}
  exit /b 0
)
if "%FCMD_CURSOR%"=="" (
  echo {"status":"nothing under the cursor"}
  exit /b 0
)
start "" "%FCMD_CURSOR%"
echo {"status":"handed %FCMD_CURSOR% to the shell"}
