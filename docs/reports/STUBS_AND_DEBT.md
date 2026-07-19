# Stubs, scaffolds y deuda técnica

## Stubs / scaffolds explícitos

| Ubicación | Qué es | Acción planeada |
|-----------|--------|-----------------|
| `velvet-script-hir` | Solo `crate_name`/`version` | Implementar HIR real o fusionar en compiler |
| `velvet-script-types` | Idem | Type checker real o documentar “structural typing only” |
| Studio “scene stub” | Genera `.vel` con TODO en **contenido** | Aceptable; no es stub del motor |
| `velvet run --path` (si se usa path de juego windowed) | Puede “note” path sin cargar ventana | Wire project load windowed |

## Cerrado en continue pass (ya no stub)

| Antes | Ahora |
|-------|--------|
| Export archive “not yet implemented” | `velvet-build::write_directory_zip` + archive en `export_desktop`; CLI lista entries |
| Play solo con path `.vel` | `velvet play <project_dir>` resuelve `velvet.project` → `entry_scene` |
| Solo CLI document patch | Studio/editor `patch-visual` llama las mismas APIs de `velvet-document` |

## Deuda de calidad

| Ítem | Severidad |
|------|-----------|
| Warnings `dead_code` residuales en paneles Studio | Baja |
| Cross-export real (no dry-run) sin toolchains locales | Media (entorno) |
| Zip no firmado / no multi-OS installer | Producto (fuera de scope) |
| **Historial GitHub:** scripts Python `scripts/gen_vs2_lang*` / cleanup de padding **ya borrados del tree actual**, pero siguen en commits antiguos de `origin/main` | Baja (hacer **luego**: rewrite/`git filter-repo` o BFG al publicar limpio — **no reescribir historial hasta decidir push**) |

## No implementado (visión producto)

- Modos Studio simplificado/avanzado **GUI** con docking egui
- Diseñador UI drag-drop GPU
- Editor 2D tilemap interactivo en Studio
- Installers firmados / Steam / Android / WASM producto

## Política

Un struct con `todo!()` en path de producto se clasifica **roto**.  
Un scaffold con docs “Phase N” se clasifica **stub**.  
Un sistema con tests y demos pero sin GUI se clasifica **funcional limitado**.
