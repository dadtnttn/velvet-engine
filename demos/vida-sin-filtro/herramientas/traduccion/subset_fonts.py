"""Recorta las fuentes Noto de respaldo a los caracteres que el juego usa.

Uso: python3 -I subset_fonts.py <carpeta con las fuentes completas> <carpeta de salida>

Cada fuente CJK/coreana se queda con los caracteres de su propio catálogo, de
todos los .vel del juego (nombres de personas y países, el selector de idioma)
y la puntuación común. Las de hindi, tailandés y hebreo ya son pequeñas y se
copian completas (sus reglas de forma de letras quedan intactas).
"""
import json
import os
import shutil
import subprocess
import sys

GAME = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SUBSET = {
    "NotoSansSC-Regular.otf": "zh-Hans",
    "NotoSansTC-Regular.otf": "zh-Hant",
    "NotoSansJP-Regular.otf": "ja",
    "NotoSansKR-Regular.otf": "ko",
}
COPY = ["NotoSansDevanagari-Regular.ttf", "NotoSansThai-Regular.ttf", "NotoSansHebrew-Regular.ttf"]


def catalog_chars(lang):
    with open(os.path.join(GAME, "assets", "i18n", lang + ".json"), encoding="utf-8") as f:
        msgs = json.load(f)["msgs"]
    out = set()
    for v in msgs.values():
        out.update(v)
    return out


def source_chars():
    out = set()
    for name in sorted(os.listdir(GAME)):
        if name.endswith(".vel") and name != "debug.vel":
            with open(os.path.join(GAME, name), encoding="utf-8") as f:
                out.update(f.read())
    return out


def common_chars():
    out = set(chr(c) for c in range(0x20, 0x7F))
    out.update(chr(c) for c in range(0x3000, 0x3040))  # puntuación CJK
    out.update(chr(c) for c in range(0xFF00, 0xFFF0))  # formas de ancho completo
    out.update("·•…–—“”‘’«»¡¿°×÷€£¥₩₹")
    return out


def main():
    src, dst = sys.argv[1], sys.argv[2]
    os.makedirs(dst, exist_ok=True)
    base = source_chars() | common_chars()
    for font, lang in SUBSET.items():
        chars = base | catalog_chars(lang)
        text_file = os.path.join(dst, f".{lang}.txt")
        with open(text_file, "w", encoding="utf-8") as f:
            f.write("".join(sorted(c for c in chars if not c.isspace() or c == " ")))
        subprocess.run(
            [
                "pyftsubset", os.path.join(src, font),
                f"--text-file={text_file}",
                f"--output-file={os.path.join(dst, font)}",
                "--layout-features=*",
                "--no-hinting",
                "--notdef-outline",
                "--drop-tables+=DSIG",
            ],
            check=True,
        )
        os.remove(text_file)
        size = os.path.getsize(os.path.join(dst, font))
        print(f"{font}: {len(chars)} caracteres -> {size // 1024} KB")
    for font in COPY:
        shutil.copyfile(os.path.join(src, font), os.path.join(dst, font))
        print(f"{font}: copia completa {os.path.getsize(os.path.join(dst, font)) // 1024} KB")


if __name__ == "__main__":
    main()
