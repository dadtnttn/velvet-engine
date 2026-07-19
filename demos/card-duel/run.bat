@echo off
setlocal
cd /d "%~dp0\..\.."
if not exist "target\release\card-duel.exe" (
  echo Building card-duel...
  cargo build -p card-duel --release
  if errorlevel 1 exit /b 1
)
start "" "target\release\card-duel.exe"
echo Launched Card Duel.
endlocal
