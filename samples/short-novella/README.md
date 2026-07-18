# Short Novella — Velvet sample (S8)

Multi-scene visual novel sample for Ren’Py-parity track.

| Metric | Count |
|--------|------:|
| Scenes | 8 (`intro` … `ending_*`) |
| Decision points | 5 `choice` blocks |
| Named endings | 3 (`Free Ride`, `Locker Light`, `Shared Rain`) |
| Approx. playtime | ~5–12 min reading (not 30–60 commercial) |

## Play

```bash
velvet play samples/short-novella --choice 0
velvet play samples/short-novella --choice 1
velvet play samples/short-novella --choice 2
```

Spanish (after `localization extract-story` + edits under `tl/es/`):

```bash
velvet play samples/short-novella --lang es --choice 0
```

## Gallery

Unlocks are exercised in unit tests via `Gallery` API (`velvet-story`) with ids:
`cg_station`, `cg_train`, `cg_rooftop`.

## Launch flow

```bash
velvet launch samples/short-novella --choice 0 --export-out dist/short-novella --binary hello-velvet
```
