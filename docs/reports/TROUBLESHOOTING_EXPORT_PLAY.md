# Troubleshooting — play & export

## Play

- **Product path:** `velvet play <project|file.vel> [--choice N] [--lang es] [--windowed]`
- **Recheck:** `velvet recheck-replay <project>`
- **Launch:** `velvet launch <project>` (info + check + play + export)

### Common errors

1. **Parse `file:line:column`** — script check prints locations; fix that line.
2. **No ending** — increase `--max-steps` or fix jumps to ending scenes.
3. **Spanish still English** — ensure `tl/es/strings.json` keys match `extract-story` keys; use `--lang es`.
4. **WindowRunner fail** — logged honestly; continue headless.

## Export

```bash
velvet export --binary hello-velvet --out dist --build --release
velvet export --multi --out dist/platforms --binary hello-velvet   # dry-run multi
```

| Issue | Action |
|-------|--------|
| Binary not found after build | Package name must match a workspace binary (`hello-velvet`) |
| Zip missing | Real export always writes `{project}-{binary}-{platform}.zip` |
| Cross target fails | Install `rustup target add …`; dry-run still OK for CI evidence |

## Sample content

`samples/short-novella` — 8 scenes, 3 endings, gallery.json, EN/ES `tl/`.
