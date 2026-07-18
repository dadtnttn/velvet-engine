# Orden de implementación (post-auditoría)

Alineado con el roadmap del producto y la prioridad real del brief.

| Fase | Nombre | Criterio de salida medible |
|------|--------|----------------------------|
| **0** | Auditoría | Este conjunto de informes + build/test demos verdes |
| **1** | Estabilización núcleo | clippy/fmt gateables; warnings críticos limpios; demos OK |
| **2** | Formato común + round-trip | Editar región visual, guardar, conservar bloque advanced |
| **3** | Proyectos + plantillas | Crear desde plantilla en ≤5 acciones CLI/Studio; run demo |
| **4** | Diseñador UI visual | Mover botón, cambiar texto, acciones; persistir |
| **5** | Editor narrativo bloques | Crear diálogo/decisión visual → .vel |
| **6** | Grafo narrativo | Nodos/aristas sync con saltos |
| **7** | Modo avanzado Studio | Jerarquía, inspector, script buffer, consola |
| **8** | LSP stdio | initialize/didOpen/diagnostics; VS Code config |
| **9** | Editor 2D | Colocar entidad/tile básico |
| **10** | Render/text/audio polish | Benchmarks reales documentados |
| **11** | Story completo | Template VN jugable E2E |
| **12** | Play/RPG/Action polish | Templates top-down/action E2E |
| **13** | Export desktop | Package ejecutable fuera del repo |
| **14** | Hybrid project | Un proyecto muestra todo |
| **15** | Endurecimiento | CI, deny, migraciones |
| **16** | Release candidate | Criterios §31 del brief |

## Prioridad si el tiempo es corto

```text
1. Fase 1 (no romper)
2. Fase 2 (round-trip)  ← bloqueante para Studio visual
3. Fase 3 (plantillas/flujo)
4. Fase 4 (UI designer mínimo)
5. Fase 5 (narrativa visual mínima)
… resto en paralelo solo si no rompe 1–5
```

## No hacer todavía

- Motor 3D, networking, Steam, Live2D.
- Inflar LOC.
- Sustituir ECS/script/VM que ya pasan tests sin benchmark.
