# Roadmap de UI + animación — crear interfaces y movimiento

Estado: fases 1–9 cerradas. El orden expresaba dependencias, no calendario.
Ajuste honesto: la galería es un tour headless con asserts (`TOUR OK`),
no una ventana interactiva — el pintado por ventana sigue del lado del
host (ver `third-person` para el camino wgpu).

Documentos normativos relacionados:

- [Contrato de VCSS](VELVET_STYLE.md) y [roadmaps VCSS](VCSS_ROADMAP.md),
  [parte 2](VCSS_ROADMAP_2.md).
- [Animación como toolbox](VELVET_ANIM.md).
- Crates: `velvet-ui`, `velvet-anim` · Demos: `velvet-stakes` (principal
  consumidora de UI real).

## Objetivo

Que crear una interfaz de juego completa (menús, HUD, diálogos, listas,
opciones) y animarla (tweens, timelines, transiciones de lista, springs)
sea componer herramientas probadas, no reinventar widgets. La dirección es:

```text
widgets probados (dropdown, tabs, tooltip, listas)
    -> layouts que contienen (wrap, scroll, anchors)
    -> temas conmutables (dark/light/densidad)
    -> movimiento con mezcla (crossfade, eventos, springs, paths)
    -> transiciones de layout (FLIP, enter/exit)
    -> galería jugable que lo demuestra todo
```

## Punto de partida

Ya disponible (verificado en código y tests):

- Widgets: `Panel/Label/Button/Image/ProgressBar/Slider/Toggle/TextField`
  + `ScrollView` aislado + foco teclado/gamepad + hit-test + `DialogueBox`.
- Layouts: `Stack/Row/Column/Grid{columns}`, `padding/margin/gap`,
  `flex_grow`, `min/max-size`, `anchor_bottom`.
- Temas: `Theme::velvet_dark/light` (estructurales, sin registro).
- Anim 2D: `FloatTween`, `Timeline/ChannelTrack` con loop, `AnimDirector`
  multitarget, `AnimPose/Pose3D` + proyección, 8 `EffectKind`, recetas
  (`flip/emerge`), `UiTween/UiAnimator`, puente `.vcss`
  (`timeline_from_plan`, `vanim_to_vcss`).
- Puentes: `vcss_style[_with_media|_auto]`, `background_gradient`,
  `box_shadows`, `@media` por flags.

Deuda conocida:

- Sin `dropdown/select`, `tabs`, `tooltip`, lista virtualizada, datagrid,
  menú contextual ni drag&drop.
- Sin `flex-wrap`, sin scroll-container integrado (`ScrollView` no es
  `LayoutType`), anchors solo abajo-centro.
- Sin registro de temas (un `Theme` activo a mano) ni densidad.
- Sin mezcla entre timelines, sin eventos por keyframe, sin springs
  (easings cerrados), sin paths Bézier (lerp por canal), sin animación
  de layout (FLIP) ni enter/exit de listas, sin partículas ligadas a UI.

## Orden de implementación

| Fase | Resultado | Toca | Criterio de salida | Esfuerzo |
|---|---|---|---|---|
| 1 | Widgets `Dropdown`, `Tabs`, `Tooltip` (retained, con foco/teclado) | `velvet-ui`: `widget.rs`, `tree.rs` (interacción), tests | Abrir/elegir/cerrar con ratón y teclado; sin fugas de foco | M |
| 2 | `VirtualList` (ventana sobre N items, callback de fila) | `velvet-ui`: nuevo `list.rs` | 10k items a 60fps lógicos; scroll+teclado; test de ventana | M |
| 3 | Layouts: `flex-wrap` en Row/Column, `ScrollView` como contenedor con layout propio, anchors generales (esquinas/bordes + offsets) | `velvet-ui`: `layout.rs`, `scroll.rs` | Wrap medido en tests; scroll+layout integrados; anchors en 9 puntos | M |
| 4 | Temas: `ThemeRegistry` (registro, activo, dark/light/auto por `@media`), densidad (`compact/comfortable` escala `gap/padding/font`) | `velvet-ui`: `theme.rs`, puente `vcss.rs` | Conmutar tema sin rebuild; test de densidad | S |
| 5 | Mezcla en `velvet-anim`: crossfade entre timelines, eventos/marcadores por keyframe (`on t → callback/log`), `UiAnimator` multicanal | `velvet-anim`: `track.rs`, `director.rs`; `velvet-ui`: `animation.rs` | Crossfade con valor exacto a mitad; eventos disparan una vez | M |
| 6 | Springs + paths: `SpringTween` (stiffness/damping, estable), movimiento por Bézier/Catmull-Rom en `director.move_to` y `EffectKind` | `velvet-anim`, `velvet-math` (curvas ya existen) | Spring converge sin oscilación eterna; path samplea la curva | M |
| 7 | Animación de layout: FLIP (registrar antes/después, animar delta) + enter/exit en listas (fade+slide por defecto) | `velvet-ui`: nuevo `transition.rs`, `VirtualList` | Reordenar anima posiciones; añadir/quitar anima alpha | M |
| 8 | Partículas UI: emisor 2D ligado a nodos (spawn en rect, tick con UI, upload a paint) reutilizando `velvet-render` particles | `velvet-ui`, `velvet-render` | Burst en click de demo; conteo estable en test | S |
| 9 | Galería jugable + docs: demo `ui-gallery` (cada widget + cada animación visible y operable), capítulo en docs, `check_test_quality` verde | `demos/ui-gallery`, docs | Un humano recorre todo sin leer código; gate completo verde | M |

## Explícitamente fuera (near-term)

- Editor visual WYSIWYG / canvas GPU de diseño (Studio sigue data-first).
- Esquelético 2D tipo Spine, huesos IK, mallas deformables.
- Drag&drop general, datagrid editable, menús contextuales anidados
  (revaluar solo con demo real que los pida).
- Paridad con navegadores (CSS grid auto-placement, container queries).

## Definition of Done (por fase)

1. `cargo check --workspace` y `cargo clippy --workspace --all-targets -- -D warnings`.
2. Tests con valores exactos y comportamiento (foco, teclado, ventana,
   convergencia); `tools/check_test_quality.py` verde.
3. `cargo fmt --all --check` limpio en archivos tocados.
4. Docs de la fase (`VELVET_STYLE.md` si toca VCSS, capítulo UI en docs)
   + la galería lo demuestra cuando aplica.
