@echo off
setlocal
cd /d "%~dp0\..\.."
if not exist "target\release\velvet-stakes.exe" (
  echo Building velvet-stakes...
  cargo build -p velvet-stakes --release
  if errorlevel 1 exit /b 1
)
start "" "target\release\velvet-stakes.exe"
echo Launched Velvet Stakes.
endlocal
