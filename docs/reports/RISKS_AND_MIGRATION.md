# Riesgos y plan de migración

## Riesgos

| ID | Riesgo | Impacto | Mitigación |
|----|--------|---------|------------|
| R1 | Editor visual destruye scripts manuales | Alto | Fase 2: CST/regions @visual/@advanced |
| R2 | Reescribir Studio rompe CLI tools útiles | Medio | Mantener velvet-studio CLI; GUI como capa |
| R3 | tower-lsp vs NDJSON actual | Medio | Nuevo binario o feature; tests de protocolo |
| R4 | HIR/types vacíos engañan a usuarios | Medio | Implementar o eliminar del workspace público |
| R5 | Cambios de save/format | Alto | version + migrate + tests |
| R6 | Dependencia egui/wgpu en editor | Medio | Feature flags; headless CI sin GUI |
| R7 | Scope creep (3D, networking) | Alto | Fuera de alcance 0.2 |

## Plan de migración de formatos

1. **velvet.project** — ya version implícita 0.1; al cambiar campos: `format_version` + `validate` + migrate.
2. **SaveGame** — ya `format_version` + checksum; nuevos campos con `#[serde(default)]`.
3. **UI/scene visual IR** — nuevo archivo versionado (ej. `.velui` / side-car) **o** anotaciones en `.vel`; no dos fuentes de verdad sin sync.
4. **Bytecode** — `BYTECODE_VERSION`; recompilar scripts en load.

## Compatibilidad

- No borrar APIs públicas de story/play sin deprecate + tests.
- Demos existentes deben seguir compilando en cada fase.
- Plantillas: regenerables; documentar breaking en CHANGELOG.

## Decisiones iniciales (Fase 0→1)

| Decisión | Elección | Razón |
|----------|----------|-------|
| Arquitectura | Conservar workspace | Ya modular y verde |
| Editor GUI | egui sobre winit (Fase 3–4) | Maduro, ya en stack Rust tools |
| Round-trip | Anotaciones `@visual`/`@advanced` + CST ligero | Cumple brief sin DOS formatos |
| LSP | tower-lsp stdio en crate existente (Fase 8) | Estándar VS Code |
