# Tutorial — Your first VN (create → translate → play → export)

Measurable path matching Ren’Py-parity S6–S9.

## 1. Create

```bash
velvet template install my_vn --template visual-novel --out .
cd my_vn
```

## 2. Extract + Spanish scaffold

```bash
velvet localization extract-story scripts/main.vel --project . --lang es
# edit tl/es/strings.json
velvet localization langs .
```

## 3. Play EN / ES

```bash
velvet play . --choice 0
velvet play . --lang es --choice 0
```

## 4. One-command launch (check + play + export)

```bash
velvet launch . --choice 0 --export-out dist --binary hello-velvet
# or Studio:
velvet-studio launch . --choice 0 --export-out dist
```

## 5. Multi-platform dry-run

```bash
velvet export --multi --out dist/multi --binary hello-velvet
```

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `no velvet.project` | Run from project root or pass path to `launch` / `play` |
| `language es: no tl/es/…` | Run `extract-story --lang es` first |
| `script check failed path:line` | Fix syntax at reported line |
| Windowed fails | Headless product play still works; see `s1_windowed` honesty notes |
| Export binary missing | Use `--build --release` and a known package (`hello-velvet`) |
| Cross-platform real build fails | Need rustup target; dry-run still writes manifests |

See also: `docs/reports/VN_LANGUAGE_SUBSET.md`, `docs/reports/CREATE_TO_PLAY.md`, `docs/reports/RENPY_PARITY.md`.
