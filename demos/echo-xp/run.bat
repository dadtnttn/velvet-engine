@echo off
setlocal enabledelayedexpansion

echo [ECHO//XP] Checking asset installation...
if not exist "demos\echo-xp\data\assets\wallpapers\bliss.png" (
    echo [ECHO//XP] Assets missing. Running Python importer...
    python demos\echo-xp\tools\import_winxp_assets.py
    if errorlevel 1 (
        echo [ECHO//XP] Asset import failed!
        pause
        exit /b 1
    )
)

echo [ECHO//XP] Building and launching game...
cargo run -p echo-xp
if errorlevel 1 (
    echo [ECHO//XP] Application exited with error.
    pause
    exit /b 1
)
