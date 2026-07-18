# Flujo crear → jugar (medible)

## Product path (S1–S5)

Uses **`VnSession`** (Say + Choice + presentation + BGM intents) via `velvet play`.

```bash
velvet template install author_full --template visual-novel --out $SCRATCH
velvet document patch $SCRATCH/author_full/scripts/main_menu.vel button.start text "Continuar historia"
velvet document patch $SCRATCH/author_full/scripts/main_menu.vel button.start position "(55%, 58%)"
# advanced on_pressed { game.new() } conservado
velvet narrative edit $SCRATCH/author_full/scripts/main.vel \
  --scene main --speaker hero --text "Una linea nueva del editor." \
  --choice-a "Ir a warm" --jump-a warm \
  --choice-b "Ir a cool" --jump-b cool
velvet script check $SCRATCH/author_full/scripts/main.vel
# errors print as path:line:column: message
velvet recheck-replay $SCRATCH/author_full --choice 0 --max-steps 80
# or:
velvet play $SCRATCH/author_full --choice 0 --max-steps 80 --windowed
# Ending: Warm Lights  (product [say] / [choice] lines in log)
velvet export --binary hello-velvet --out $SCRATCH/export_out --build --release
```

## Windowed

```bash
velvet play $SCRATCH/author_full --windowed --choice 0
```

- Attempts an App host tick (HeadlessRunner multi-frame as portable proof when no display).
- If a real GPU window is unavailable, log records the fallback; **headless product path** still reaches the named ending.

## Resultado esperado

| Paso | Resultado |
|------|-----------|
| Create template | OK |
| Visual patch menu | text + position; `game.new()` advanced intact |
| Narrative edit | dialogue + decision; advanced/state preserved |
| script check | file:line diagnostics on errors |
| recheck-replay / play product | `[say]` lines, choices, `Ending: Warm Lights` |
| export | zip contains host binary; launch EXIT 0 |

Language subset: `docs/reports/VN_LANGUAGE_SUBSET.md`  
Parity checklist: `docs/reports/RENPY_PARITY.md`

## Comandos

- `velvet play <project|file.vel> [--choice N] [--max-steps N] [--windowed]`
- `velvet recheck-replay <project|file.vel> [--choice N]`
- `velvet document patch <path> <region> <key> <value>`
- `velvet narrative edit …`
- `velvet export --binary <name> --out <dir> --build --release`
