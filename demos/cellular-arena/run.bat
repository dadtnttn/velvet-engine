@echo off
setlocal
cd /d "%~dp0\..\.."
if not exist "target\release\cellular-arena.exe" (
  echo Building cellular-arena...
  cargo build -p cellular-arena --release
  if errorlevel 1 exit /b 1
)
start "" "target\release\cellular-arena.exe"
echo Launched Cellular Arena.
endlocal
