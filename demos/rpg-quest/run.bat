@echo off
setlocal
cd /d "%~dp0\..\.."

echo Iniciando RPG Quest: La liberacion de Solaria...
cargo run -p rpg-quest --release

if errorlevel 1 (
    echo.
    echo La demo termino con un error.
    pause
)
