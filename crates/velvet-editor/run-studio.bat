@echo off
setlocal
cd /d "%~dp0\..\.."
if not exist "target\release\velvet-studio.exe" (
  echo Building velvet-studio...
  cargo build -p velvet-editor --release
  if errorlevel 1 exit /b 1
)
echo Launching Velvet Studio (interactive dual-mode)...
echo Tab=mode  S=drop button  drag=move  Esc=quit
start "" "target\release\velvet-studio.exe" gui templates/visual-novel --interactive
endlocal
