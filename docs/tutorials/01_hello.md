# Tutorial 01 — Hello Velvet

This tutorial gets a minimal project on disk, checks a script, and runs the engine headless.

## Prerequisites

- Rust 1.80+ toolchain (`rustup`)
- This repository checked out and building:

```bash
cargo build -p velvet-cli
```

The `velvet` binary is produced as the CLI package.

## Create a project

```bash
velvet new hello --template visual-novel --out .
cd hello
```

You should see:

- `velvet.project` — RON project document
- `scripts/main.vel` — starter dialogue
- `assets/` — empty content folder
- `README.md`

## Inspect the project

```bash
velvet project info --validate
```

Expected: name/modules printed; possible **warnings** if `assets/` is empty or
`entry_scene` paths are placeholders. Warnings are OK for a fresh template.

## Check the script

```bash
velvet script check scripts/main.vel
```

This parses and compiles the file, printing diagnostics if any.

Format in place:

```bash
velvet script fmt scripts/main.vel
```

## Run the engine smoke loop

```bash
velvet run --headless --frames 30
```

This does **not** yet load your project path into a full game session; it verifies
the app runner and plugin stack. Project-aware play is evolving — use the
workspace examples for end-to-end demos:

```bash
cargo run -p visual-novel
cargo run -p hello-velvet
```

## Doctor

From the engine workspace root:

```bash
velvet doctor
```

Checks rustc/cargo, workspace crates, and template directories.

## Studio (optional)

```bash
velvet-studio open .
```

Shell commands: `hierarchy`, `check`, `assets`, `inspect`, `fmt`, `new-scene`, `quit`.

## What you learned

1. How projects are represented (`velvet.project`)
2. How to validate and check scripts from the CLI
3. That headless `velvet run` is an engine smoke test, not full game load (yet)

Next: [02_visual_novel.md](./02_visual_novel.md)
