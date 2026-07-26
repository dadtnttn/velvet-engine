@echo off
setlocal
cd /d "%~dp0\..\.."

if not exist "demos\sam-tomboy\data\assets\backgrounds\png\menu.png" (
    echo Generando fondos temporales de Sam...
    python "demos\sam-tomboy\tools\generate_backgrounds.py"
    if errorlevel 1 (
        echo.
        echo No se pudieron generar los fondos.
        pause
        exit /b 1
    )
)

echo Iniciando Sam: Honest Stranger...
cargo run -p sam-tomboy
if errorlevel 1 (
    echo.
    echo La demo termino con un error.
    pause
    exit /b 1
)

endlocal
