# Roadmap de interacciones — botones animados y bonitos, y todo lo que se siente

Estado: fases 1–7 cerradas. El orden expresaba dependencias, no calendario.

Documentos relacionados: [UI_ANIM_ROADMAP](./UI_ANIM_ROADMAP.md) (fases 1–9
cerradas), [reporte UI+anim](../reports/UI_ANIM.md).

## Objetivo

Que cada interacción se *sienta* bien con herramientas, no a mano: botones
que crecen al hover, se aplastan al pulsar y hacen pop al soltar; sonidos
de UI enganchados a eventos; repetición al mantener; focos visibles solo
con teclado; y todo respetando `reduced-motion`. La dirección es:

```text
botón animado declarativo (hover/press/disabled con springs)
    -> feedback de presión (ripple, squash, pop, partículas)
    -> sonido de UI por eventos (la engine emite, el host suena)
    -> repetición al mantener + doble click
    -> busy/spinner + toggles y sliders vivos
    -> foco visible solo con teclado + reduced-motion en todo
    -> galería que lo demuestra
```

## Punto de partida

Ya disponible: `Button { label, pressed, hovered }`, click por
press+release y por `confirm`, `button/button_hovered` en tema, estados
VCSS (`hover/active/focus/disabled/selected/open`), `UiTween`
(opacity/offset/scale) + `play_fade`, `Spring`, `EffectKind::Punch/Shake`,
bursts `UiBurst::click/sparkle`, `AppearTracker`, `arrows_consumed`.

Deuda conocida: el hover/press solo pone flags (el host anima a mano o
nada); sin sonidos de UI; sin repetición al mantener; sin doble click;
sin estado busy; toggles/sliders sin animación propia; el anillo de foco
no distingue ratón de teclado; `reduced-motion` no apaga tweens de UI.

## Orden de implementación

| Fase | Resultado | Toca | Criterio de salida | Esfuerzo |
|---|---|---|---|---|
| 1 | `AnimatedButton`: máquina hover/press/disabled/selected que emite tweens (grow al hover, squash al press, pop al release) vía `UiAnimator`, con presets y `reduced-motion` = estados instantáneos | `velvet-ui`: nuevo `button_fx.rs` | Secuencia hover→press→release da samples exactos; con reduced-motion no hay tween | M |
| 2 | Sonido UI por eventos: `UiSoundQueue` (`hover/click/disabled/invalid/pick/select` + nodo) emitida en `process()`, el host la drena a `velvet-audio` | `velvet-ui`: `tree.rs`, nuevo `sound.rs` | Click suena una vez; hover suena al entrar (no cada frame); disabled suena distinto | S |
| 3 | Pack de presión: descriptor `Ripple` (círculo expansivo para paint), squash por escala, pop con `Spring`, confetti en `clicked` vía `UiParticles` | `velvet-ui`: `button_fx.rs`, `particles.rs` | Ripple con radio/alpha exactos a mitad; pop converge | S |
| 4 | Mantener-para-repetir (delay + rate) y doble click (ventana configurable) en botones | `velvet-ui`: `tree.rs`/`button_fx.rs` | Repeat dispara N veces en T segs; doble click distingue de dos simples | S |
| 5 | Busy buttons (spinner + disabled + swap de label, `set_busy`) y toggles/sliders vivos (knob deslizante, glow al drag) | `velvet-ui`: `widget.rs`, `tree.rs` | Busy bloquea clicks; knob llega al extremo | M |
| 6 | Foco visible solo con teclado: modalidad de último input (puntero/teclado), estado `:focus-visible`, anillo solo entonces | `velvet-ui`: `tree.rs`, `vcss.rs` | Ratón nunca muestra anillo; Tab sí; test de ambas | S |
| 7 | Galería + docs: parada de interacciones en `ui-gallery` (cada efecto disparado y afirmado), capítulo en docs, gate verde | `demos/ui-gallery`, docs | `TOUR OK` incluye delicia; `reduced-motion` apaga todo | M |

## Explícitamente fuera (near-term)

- Cursor custom / punteros por widget (lado host).
- Háptica/gamepad rumble (sin ABI de host para ello).
- Física de arrastre con inercia (scroll básico ya existe).
- Sonido procedural en la engine (los clips los pone el juego).

## Definition of Done (por fase)

1. `cargo check --workspace` y `cargo clippy --workspace --all-targets -- -D warnings`.
2. Tests con valores exactos (samples a mitad, conteos de eventos,
  _timing_ con relojes fijos); `tools/check_test_quality.py` verde.
3. `cargo fmt --all --check` limpio en archivos tocados.
4. `reduced-motion` verificado en cada efecto nuevo + docs de la fase.
