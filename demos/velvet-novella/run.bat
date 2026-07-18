@echo off
setlocal
cd /d "%~dp0\..\.."
if not exist "target\release\velvet-novella.exe" (
  echo Building velvet-novella...
  cargo build -p velvet-novella --release
  if errorlevel 1 exit /b 1
)
start "" "target\release\velvet-novella.exe"
echo Launched Velvet Novella.
endlocal
