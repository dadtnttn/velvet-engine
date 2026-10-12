# Traducción de Vida Sin Filtro

El juego está escrito en español. El motor traduce cada texto al dibujarlo con
el catálogo del idioma elegido, `../../assets/i18n/<idioma>.json`
(`{"lang": "de", "msgs": {"texto en español": "traducción"}}`). Cómo busca, qué
significan los huecos (`{0}`, `{0!}`, `{0|a=..|*=..}`, `{0|#one=..}`,
`{@g|m=..|f=..}`) y cómo parte las líneas está en
`docs/architecture/VS3_APP_RUNTIME.md`, sección «Traducción».

Los catálogos se generan con estas herramientas; no se editan a mano.

| Archivo | Para qué |
|---|---|
| `i18n_prep.py` | Convierte la salida de `velvet vs3 strings` en claves con huecos tipados (nombre, género, número, dinero, texto) |
| `tx.py` | Lotes de traducción por idioma: ver, ingresar, validar, construir los catálogos |
| `i18n_check.py` | Valida una traducción contra sus huecos (lo usa `tx.py put`) |
| `STYLE.md` | Guía para quien traduce (tono, huecos, género del jugador, etiquetas cortas) |
| `gender_candidates.py` | Lista líneas que le hablan al jugador y cuya traducción tiene formas con género sin `{@g|m=..|f=..}` (ru, pl, he, hi, fr, pt-BR, it) |
| `subset_fonts.py` | Recorta las fuentes Noto de respaldo a los caracteres que usa cada idioma |

El trabajo (claves, lotes, traducciones por idioma) vive en `trabajo/`, que
no se sube: se reconstruye desde los catálogos. Otra carpeta: `VSF_I18N=...`.

## Traducir lo nuevo

Desde la raíz del repositorio:

```bash
H=demos/vida-sin-filtro/herramientas/traduccion
cargo run -p velvet-cli --release -- vs3 strings demos/vida-sin-filtro --out /tmp/strings.json
python3 $H/i18n_prep.py /tmp/strings.json /tmp/prep

# Primera vez: arma trabajo/ y toma lo que los catálogos ya traducen.
python3 $H/tx.py init /tmp/prep
# Las siguientes: lo nuevo va en lotes aparte (los viejos no se mueven).
python3 $H/tx.py delta /tmp/prep/keys.json

python3 $H/tx.py stats            # avance por idioma
python3 $H/tx.py todo de          # lotes con algo pendiente en alemán: "lote:faltan"
python3 $H/tx.py show 87 de       # las líneas pendientes del lote 87
# Escribe "n|traducción" por línea en un archivo (⏎ = salto de línea, n|= deja
# el original) y súbelo; lo que no pase la validación se informa y no entra.
python3 $H/tx.py put de 87 /tmp/de_87.txt

python3 $H/tx.py build            # catálogos + i18n_data.vel
cargo run -p velvet-cli --release -- vs3 lock demos/vida-sin-filtro
cargo run -p velvet-cli --release -- vs3 check demos/vida-sin-filtro
```

`build` escribe en inglés lo que falte en otro idioma (y lo anota en
`"fallback"` del catálogo para que `init` no lo tome por traducido), quita las
plantillas que el código ya no arma y agrega las derivadas: precios como
hueco (`Te quitó {0} y el celular.`) y el singular de las frases con
`plural()` (`Hace {0} año`).

## Fuentes

Chino, japonés, coreano, hindi, tailandés y hebreo necesitan las fuentes Noto
(SC, TC, JP, KR, Devanagari, Thai, Hebrew) en `../../assets/fonts/`. No se
suben completas (pesan 23 MB): se recortan a lo que usa el juego.

```bash
python3 -I $H/subset_fonts.py <carpeta con las Noto completas> demos/vida-sin-filtro/assets/fonts
```
