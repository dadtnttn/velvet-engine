#!/usr/bin/env python3
import sys
import os
import zipfile
import shutil

DEFAULT_ZIP_PATH = os.environ.get("ECHO_XP_ASSET_ARCHIVE")

def get_script_dir():
    return os.path.dirname(os.path.abspath(__file__))

def get_output_dir():
    return os.path.abspath(os.path.join(get_script_dir(), "..", "data", "assets"))

# Mapping: original Zip path -> (relative output path, purpose, mandatory)
MAPPING = {
    # Wallpapers
    "WinXp/Wallpapers/Bliss.png": ("wallpapers/bliss.png", "Default XP Bliss desktop background", True),
    "WinXp/Wallpapers/Windows_XP_Professional.png": ("wallpapers/professional.png", "Professional XP desktop background", False),
    "WinXp/Wallpapers/Windows_XP_Home_Edition.png": ("wallpapers/home_edition.png", "Home Edition XP desktop background", False),

    # UI & Logos
    "WinXp/Frame/UI Theme.png": ("ui/xp_theme.png", "XP window frame and UI 9-slice atlas", True),
    "WinXp/Logo/Boot Settings Screen.png": ("ui/boot_screen.png", "Boot screen logo / background", True),
    "WinXp/Logo/WindowsLogo-small.png": ("ui/windows_logo_small.png", "Small Windows XP logo", True),
    "WinXp/Misc/mineswapper.png": ("ui/minesweeper.png", "Minesweeper sprite asset", False),

    # Icons
    "WinXp/Icons/WinIcons_16.png": ("icons/winicons_16.png", "16x16 icon atlas", True),
    "WinXp/Icons/WinIcons_32.png": ("icons/winicons_32.png", "32x32 icon atlas", True),
    "WinXp/Icons/WinIcons_48.png": ("icons/winicons_48.png", "48x48 icon atlas", True),

    # Cursors
    "WinXp/Cursor/default_arrow.png": ("cursors/arrow.png", "Default arrow cursor", True),
    "WinXp/Cursor/default_link.png": ("cursors/link.png", "Link hand cursor", True),
    "WinXp/Cursor/default_ibeam.png": ("cursors/ibeam.png", "I-beam text cursor", True),
    "WinXp/Cursor/default_wait.png": ("cursors/wait.png", "Wait hourglass cursor", True),
    "WinXp/Cursor/default_move.png": ("cursors/move.png", "Move cursor", True),
    "WinXp/Cursor/default_size1.png": ("cursors/size_nwse.png", "Diagonal resize cursor 1", True),
    "WinXp/Cursor/default_size2.png": ("cursors/size_nesw.png", "Diagonal resize cursor 2", True),
    "WinXp/Cursor/default_size3.png": ("cursors/size_ns.png", "Vertical resize cursor", True),
    "WinXp/Cursor/default_size4.png": ("cursors/size_we.png", "Horizontal resize cursor", True),

    # Sounds
    "WinXp/Sounds/Windows XP Startup.wav": ("sounds/startup.wav", "System boot sound", True),
    "WinXp/Sounds/Windows XP Logon Sound.wav": ("sounds/logon.wav", "User login sound", True),
    "WinXp/Sounds/Windows XP Shutdown.wav": ("sounds/shutdown.wav", "System shutdown sound", True),
    "WinXp/Sounds/Windows XP Ding.wav": ("sounds/ding.wav", "System ding notification sound", True),
    "WinXp/Sounds/Windows XP Error.wav": ("sounds/error.wav", "Error dialog sound", True),
    "WinXp/Sounds/Windows XP Exclamation.wav": ("sounds/warning.wav", "Warning sound", True),
    "WinXp/Sounds/Windows XP Critical Stop.wav": ("sounds/critical.wav", "Critical stop error sound", True),
    "WinXp/Sounds/Windows XP Menu Command.wav": ("sounds/menu.wav", "Menu command sound", True),
    "WinXp/Sounds/Windows XP Minimize.wav": ("sounds/minimize.wav", "Window minimize sound", True),
    "WinXp/Sounds/Windows XP Restore.wav": ("sounds/restore.wav", "Window restore sound", True),
    "WinXp/Sounds/Windows XP Notify.wav": ("sounds/notify.wav", "New email/notification sound", True),

    # Fonts
    "WinXp/Fonts/tahoma.ttf": ("fonts/tahoma.ttf", "Primary UI Font", True),
    "WinXp/Fonts/framdit.ttf": ("fonts/framdit.ttf", "Secondary UI Font", False),

    # Clip Sheets
    "WinXp/Clip/sheets/clip (idle)_sheet.png": ("clip/idle.png", "Clippy idle animation sheet", True),
    "WinXp/Clip/sheets/clip (think)_sheet.png": ("clip/think.png", "Clippy thinking animation sheet", True),
    "WinXp/Clip/sheets/clip (read)_sheet.png": ("clip/read.png", "Clippy reading animation sheet", True),
    "WinXp/Clip/sheets/clip (weird)_sheet.png": ("clip/weird.png", "Clippy weird/distorted animation sheet", True),
    "WinXp/Clip/sheets/clip (eyes)_sheet.png": ("clip/eyes.png", "Clippy eyes watching animation sheet", True),
    "WinXp/Clip/sheets/clip (listen)_sheet.png": ("clip/listen.png", "Clippy listening animation sheet", True),
    "WinXp/Clip/sheets/clip (noted)_sheet.png": ("clip/noted.png", "Clippy noted animation sheet", True),
}

def import_assets(zip_path):
    if not os.path.exists(zip_path):
        print(f"ERROR: Zip file not found at '{zip_path}'", file=sys.stderr)
        sys.exit(1)

    out_dir = get_output_dir()
    os.makedirs(out_dir, exist_ok=True)

    print(f"Importing assets from '{zip_path}' to '{out_dir}'...")

    extracted_count = 0
    manifest_entries = []

    with zipfile.ZipFile(zip_path, 'r') as z:
        zip_names = z.namelist()

        for original_path, (rel_out, purpose, mandatory) in MAPPING.items():
            if original_path not in zip_names:
                if mandatory:
                    print(f"WARNING: Mandatory asset '{original_path}' missing in ZIP!")
                continue

            target_path = os.path.join(out_dir, rel_out)
            os.makedirs(os.path.dirname(target_path), exist_ok=True)

            with z.open(original_path) as src, open(target_path, 'wb') as dst:
                shutil.copyfileobj(src, dst)
            extracted_count += 1

            manifest_entries.append((original_path, rel_out, purpose, "Mandatory" if mandatory else "Optional"))

    # Write ASSET_MANIFEST.md
    manifest_path = os.path.join(out_dir, "ASSET_MANIFEST.md")
    with open(manifest_path, 'w', encoding='utf-8') as f:
        f.write("# ECHO//XP Asset Manifest\n\n")
        f.write("| Original File in WinXp.zip | Normalized Target Path | Purpose | Required |\n")
        f.write("| --- | --- | --- | --- |\n")
        for orig, rel, purpose, req in manifest_entries:
            f.write(f"| `{orig}` | `{rel}` | {purpose} | {req} |\n")

    print(f"Successfully imported {extracted_count} assets.")
    print(f"Asset manifest generated at '{manifest_path}'.")

def main():
    zip_path = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_ZIP_PATH
    if not zip_path:
        print(
            "Usage: python import_winxp_assets.py <path-to-your-licensed-archive>",
            file=sys.stderr,
        )
        print(
            "Alternatively set ECHO_XP_ASSET_ARCHIVE. No media is bundled with the demo.",
            file=sys.stderr,
        )
        sys.exit(2)
    import_assets(zip_path)

if __name__ == "__main__":
    main()
