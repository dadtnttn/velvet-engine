# Roadmap de VCSS como lenguaje de estilo

Estado: propuesta activa, sin fechas fijas. El orden expresa dependencias, no
calendario.

Documentos normativos relacionados:

- [Contrato actual de VCSS](VELVET_STYLE.md).
- [Reglas de integración con VelvetEngine](../architecture/VS3_ENGINE_RULES.md).
- Crate: `crates/velvet-style` · CLI: `velvet style check|dump`.

## Objetivo

Convertir VCSS en un lenguaje de estilo predecible y bien diagnosticado para
UI de juegos: valores con matemática real, selectores suficientes, lint que
atrape errores de autor antes del play, y un `@script` capaz de orquestar
movimiento sin salir al host. La dirección es:

```text
hoja .vcss
    -> valores expresivos (calc, mezcla de color)
    -> selectores suficientes (descendiente + hijo)
    -> lint profundo (keyframes, vars, duplicados)
    -> @script completo para orquestación (strings, math, arrays, debug)
    -> tooling CLI (run, eval, fmt)
```

No se añadirá un motor de layout (flex/grid viven en `velvet-ui`), ni paridad
total con CSS de navegador.

## Punto de partida

Ya disponible (verificado en código y tests):

- Cascada por especificidad `(ids, clases/estados, tipos)` + selectores
  descendientes (`resolve.rs`).
- `var(--x)` y `var(--x, fallback)` recursivos, solo `:root` participa en
  resolución de elementos.
- Colores `#rgb/#rrggbb/#rrggbbaa`, `rgb()/rgba()`, `hsl()/hsla()`,
  nombrados + alias de juego; tiempos `s/ms`; `url()/svg()`.
- `@keyframes`, `animation:`/`transition:` con planes hacia `velvet-anim`.
- `@script` con `let/fn/for/if/return`, `play/animate/wait/emit/on`,
  ~25 builtins (`str/num/len/min/max/abs/floor/ceil/sin/cos/clamp/lerp`,
  `upper/lower/trim/split/join/range`, `rand/pick`, `set/query/get`).
- `lint_stylesheet` (props desconocidas), `check_stylesheet` para CLI,
  `@import` con límite de profundidad, `expand_box_shorthands`.

Deuda conocida:

- Los valores son tokens únicos: no hay `calc()/min()/max()/clamp()`.
- Sin mezcla de color (`color-mix`, `alpha()`); sin sintaxis `rgb(... / a)`.
- Selectores solo descendientes (espacio); `>` no existe.
- El lint no ve referencias rotas: `animation-name` sin `@keyframes`,
  `var(--x)` sin fallback a prop indefinida.
- `@script` sin `replace/includes/startsWith/endsWith/repeat`,
  `round/sqrt/pow`, `push/keys`, ni `print` de depuración.
- CLI sin `run`/`eval`; sin formateador.

## Orden de implementación

| Fase | Resultado | Toca | Criterio de salida | Esfuerzo |
|---|---|---|---|---|
| 1 | Valores con matemática: `calc()`, `min()`, `max()`, `clamp()` en declaraciones (unidades `px`/números/tiempos, anidables) | `value.rs` (+ tests) | `parse_value("calc(100% - 8px)")` evalúa con contexto; error claro si mezcla unidades incompatibles | M |
| 2 | Color expresivo: `color-mix(in srgb, A 60%, B)`, `alpha(c, 0.5)`, sintaxis `rgb(r g b / a)` | `value.rs` | Mezclas y alfas con valores exactos en tests; `as_color()` los resuelve | M |
| 3 | Combinador hijo `A > B` (más especificidad por compuesto, como hoy) | `resolve.rs` | `A > B` solo casa hijos directos; descendiente intacto; tests de ambas | S |
| 4 | Lint profundo: `animation-name` sin `@keyframes`, `var(--x)` sin fallback ni definición, reglas duplicadas idénticas (aviso) | `lib.rs` lint + `style_cmd.rs` | `check` reporta cada caso con selector/línea; cero falsos positivos en la plantilla | S |
| 5 | Builtins `@script`: `replace/includes/startsWith/endsWith/repeat`, `round/sqrt/pow`, `push`, `keys`, `print` (a log de `ScriptRun`, no stdout) | `script.rs` | Cada builtin con test de valor + test de error de aridad/tipo | M |
| 6 | Tooling: `velvet style run <hoja> <fn>`, `velvet style eval <expr>`, ejemplo comentado en plantilla | `style_cmd.rs`, `main.rs`, plantilla | Comandos con tests CLI; docs con ejemplo | S |

## Explícitamente fuera (near-term)

- `!important` (cambia la semántica de cascada; reevaluar solo con caso de uso real documentado).
- `@media` / container queries / anidado `&` (sin layout responsive propio no pagan su complejidad).
- Paridad total CSS (grid, flex, Houdini, custom layout): el layout vive en `velvet-ui`.
- Layout/paint nuevos en el renderer por cuenta de VCSS.

## Definition of Done (por fase)

1. `cargo check --workspace` y `cargo clippy --workspace --all-targets -- -D warnings`.
2. Tests con valores exactos (no solo `is_ok()`); `tools/check_test_quality.py` verde.
3. `cargo fmt --all --check` limpio en archivos tocados.
4. Docs de la fase en `docs/language/VELVET_STYLE.md` + ejemplo que corre.
