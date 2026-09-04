# Roadmap de VCSS, parte 2 — nuevas funciones

Estado: fases 7–13 cerradas. Continúa
[VCSS_ROADMAP.md](./VCSS_ROADMAP.md) (fases 1–6 ya cerradas); el orden
expresaba dependencias, no calendario.

Ajustes honestos respecto al plan: `map`/`filter` no se implementaron
(sin lambdas de primera clase en JS-lite — documentado en `VELVET_STYLE.md`
si llegan); `var()` dentro de argumentos de función (`color-mix`, …) queda
como límite conocido.

## Punto de partida (tras la parte 1)

- Valores: `calc()/min()/max()/clamp()` con chequeo de unidades, `%` con
  base de layout, `Percent`/`Calc` como variantes, plegado en `resolve()`.
- Color: `color-mix(in srgb)`, `alpha()`, `rgb(... / a)` y canales `%`.
- Selectores: descendiente + hijo `>` con matching derecha→izquierda.
- Lint: props desconocidas, `@keyframes` huérfanos, `var()` sin
  definición, `transition-property` desconocida.
- `@script`: strings (`replace/includes/startsWith/endsWith/repeat`),
  math (`round/sqrt/pow`), `push`/`keys`/`typeof`, `print` → `ScriptRun.logs`.
- CLI: `style check|dump|run|eval`; plantilla `styles/ui.vcss`.

## Orden de implementación

| Fase | Resultado | Toca | Criterio de salida | Esfuerzo |
|---|---|---|---|---|
| 7 | Selectores `:not()` y `:is()` (`:is()` = especificidad del arg más específico) | `resolve.rs` | `:not(.x)` niega, `:is(a, b)` une; anidados simples; tests de especificidad | S |
| 8 | `@media` de juego: `(prefers-reduced-motion)`, `(prefers-contrast: more)`, `(orientation: portrait/landscape)`, `(min-width: Npx)` evaluadas contra flags del `StyleQuery` | `parse.rs`, `resolve.rs` | Reglas dentro solo aplican si cumple; query sin flags = comportamiento actual | M |
| 9 | Gradientes: `linear-gradient(dir, stops)` (+ `radial-gradient()` si cabe) a estructura con paradas; helper de fallback a color sólido | `value.rs` | Parse exacto de paradas `%`/colores; hosts pintan o degradan | M |
| 10 | `box-shadow` estructurado (`x y blur spread color [inset]`) en vez de keyword opaco + helper de pintura | `value.rs` | Valores exactos por componente; lo viejo (keyword) sigue pasando | S |
| 11 | `@script` v2: `slice/charAt/padStart/padEnd/indexOf`, `map/filter/slice/sort/reverse`, `entries/values/has/merge`, `JSON.parse/stringify`, `atan2/hypot/sign`, `time()` (reloj del runtime, 0 por defecto = determinista), `once(event, fn)` | `script.rs` | Cada builtin con test de valor + error de aridad/tipo | M |
| 12 | Tooling: `velvet style fmt` (formateador `.vcss` idempotente) y `velvet style snapshot` (resolve hoja+query → JSON esperado, para CI de temas) | `style_cmd.rs`, `main.rs`, nuevo `fmt.rs` si hace falta | Roundtrip `fmt(fmt(x)) == fmt(x)`; snapshot falla si cambia el resuelto | M |
| 13 | Integración UI: puente `velvet-ui` para `>` y `@media` (flags desde prefs/accesibilidad), `var()`+`calc()` documentados en paint | `velvet-ui`, docs | Demo/plantilla los ejerce; sin regresiones en tests UI | S |

## Explícitamente fuera (también en parte 2)

- `!important` (sigue sin caso de uso documentado; cambia la cascada).
- Layout engine (flex/grid), container queries, nesting `&`, Houdini.
- Paridad total CSS: VCSS es estilo de UI de juegos, no de navegador.

## Definition of Done (por fase)

1. `cargo check --workspace` y `cargo clippy --workspace --all-targets -- -D warnings`.
2. Tests con valores exactos; `tools/check_test_quality.py` verde.
3. `cargo fmt --all --check` limpio en archivos tocados.
4. Docs de la fase en `docs/language/VELVET_STYLE.md` + ejemplo que corre
   (`velvet style check` sin warnings).
