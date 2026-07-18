```text
FASE COMPLETADA (base Fase 3)
Objetivo: ECS + escenas + prefabs con load/unload/additive
Cambios:
  - velvet-ecs: World, components, commands, query2 (previo)
  - velvet-scene: SceneManager, SceneBlueprint, Prefab RON, hierarchy
    (Name/Parent/Children), Persistent marker, exclusive + additive load
Pruebas:
  - load_unload_scene, additive_load_keeps_previous, exclusive_load_replaces
  - prefab ron_roundtrip, instantiate_with_child
  - evidencia: {SCRATCH}/scene_tests.log
Comandos: cargo test -p velvet-scene --all-features → exit 0
Estado:
  - scene load/update/unload: implementado (update = entities live in World)
  - visual editor scenes: planificado
  - velscene script syntax: planificado (Fase 4)
Siguiente: Fase 4 Velvet Script lexer/parser/VM
```
