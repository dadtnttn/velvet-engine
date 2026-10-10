@echo off
setlocal
cd /d "%~dp0\..\.."
cargo build --release -p rpg-quest-vs3 || exit /b 1
start "" "target\release\rpg-quest-vs3.exe"
endlocal
