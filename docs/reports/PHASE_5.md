```text
FASE COMPLETADA
Objetivo: Velvet Story — flujo VN completo; demo jugable; save/load; decisiones alteran rutas
Cambios:
  - velvet-story: Character, StoryProgram IR, load from Velvet Script AST
  - StoryPlayer: dialogue, choices, jumps, vars, history, prefs, events
  - SaveGame versioned JSON + checksum + SaveStore (slots, backup)
  - StoryPlugin ticks play time
  - examples/visual-novel (auto + interactive CLI)
  - examples/branching-story (multi-ending paths + mid save/load)
Crates afectados: velvet-story, examples/visual-novel, examples/branching-story
Pruebas: 8 unit tests in velvet-story; demos assert routes/endings
Comandos:
  cargo test -p velvet-story --all-features
  cargo run -p visual-novel -- --auto --choices 0,0,0 --save-dir …
  cargo run -p branching-story
Resultados: demos OK; aria_trust=3 on kind path; branching endings threshold/timeless
Estado:
  - VN playable start→end: implementado (CLI/headless)
  - save/load: implementado (versioned DTO)
  - decisions change routes: implementado
  - rollback / gallery / voice wait: planificado / parcial
  - graphical portraits/GPU present: parcial (IR emits Show/Background events)
Problemas conocidos:
  - no typewriter GPU text yet (velvet-text still thin)
  - ending_gate does not gate on has_key (demo simplified)
Siguiente fase: Velvet Play (maps, collisions, camera)
```
