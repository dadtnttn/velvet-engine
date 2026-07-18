# FASE 13 — Exportación de escritorio verificable

## Objetivo

Paquete que se ejecuta **fuera del árbol del motor**.

## Cambios

- `export_desktop` con `--build` falla si no encuentra el binario (antes solo log).
- Escribe `README.md`, `run.bat`, `run.sh`.
- CLI: `velvet export --build --release --binary hello-velvet --out <dir>`

## Prueba real (2026-07-17)

```text
copied binary target\release\hello-velvet.exe
export ready at …/export_out_real
running …/export_out_real/hello-velvet.exe
Hello Velvet finished: 30 update(s), exit 0
EXPORT_RUN_EXIT=0
```

Log: `{SCRATCH}/export_run.log`

## Limitaciones

- Cross-compile no probado sin toolchains.
- Empaquetado zip/installer no implementado.
- Assets vacíos en este demo (hello-velvet no requiere assets).
