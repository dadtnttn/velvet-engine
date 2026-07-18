```text
FASE COMPLETADA (parcial → base Fase 2 + arranque Fase 3)
Objetivo: Render 2D wgpu, audio buses, input actions, Hello Velvet con plugins reales
Cambios:
  - velvet-render: Camera2D, letterbox/scaling profiles, SpriteBatch, TextureAtlas,
    GpuContext (headless + windowed), WGSL sprite pipeline, clear, batching stats
  - velvet-audio: Master/Music/Voice/Effects/Ambient/UI buses, voices, fade, crossfade,
    voice limit, spatial atten, null-device tick (tests without hardware)
  - velvet-input: winit_map for keyboard/mouse; action contexts
  - velvet-app: WindowInit/Frame/Resize hooks for GPU present path
  - hello-velvet: real plugins Input+Assets+Audio+Render; headless demo
Crates afectados: velvet-render, velvet-audio, velvet-input, velvet-app, hello-velvet, velvet-cli
Pruebas añadidas:
  - render: camera, letterbox, batch draw-call reduction, GPU headless (skip if no adapter)
  - audio: play/tick, mute, fade-in, voice steal, spatial
  - input: WASD axis, edges, winit key map
Comandos ejecutados:
  cargo test --workspace --all-features  → exit 0 (log: {SCRATCH}/cargo_build_test.log)
  cargo run -p hello-velvet              → 30 updates, exit 0
  cargo run -p velvet-cli -- doctor
  cargo run -p velvet-cli -- run --headless --frames 5
Resultados: workspace green; GPU headless tests passed on host
Líneas Rust (aprox): ~9200 (rs_lines en scratch line_count.txt)
Estado por feature:
  - sprites/cameras/batching/profiles: implementado (CPU + GPU encode path)
  - post-FX / particles / multi-camera full: planificado
  - audio kira hardware output: planificado (API + null backend implementado)
  - hot-reload notify watcher: planificado
  - windowed GPU present demo: parcialmente (hooks listos; full binary path next)
Problemas conocidos:
  - Pipeline format fixed Bgra8UnormSrgb; may need surface-format match on some GPUs
  - Input not auto-fed from WindowRunner (host uses velvet_input::winit_map)
  - 50k LOC no alcanzado
Deuda técnica:
  - dead_code allows on GpuTexture retention fields
  - Audio null mixer does not write to OS device yet
Siguiente fase: escenas/prefabs (completar Fase 3) + Velvet Script lexer/parser (Fase 4)
```
