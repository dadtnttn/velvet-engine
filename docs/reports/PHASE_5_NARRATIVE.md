# FASE 5 — Editor narrativo por bloques

## Objetivo

Crear una escena narrativa completa de forma estructurada (bloques) y emitir `.vel`.

## Implementación

`velvet_document::NarrativeDocument` / `NarrativeBlock`:

- Fondo, música, show/hide, diálogo, narración, pensamiento
- Decisiones con brazos y saltos
- Variables, condiciones, jump/call, ending
- Bloque Advanced / Comment
- `to_source()` / `from_source()` / `validate()` (saltos rotos)

## Criterio de salida

- [x] Construir escena apartment + decisión + dos finales en API  
- [x] Emitir source con `choice` y `jump`  
- [x] Validar saltos faltantes  
- [x] Tests unitarios en `narrative::tests`  

## Evidencia

`{SCRATCH}/narrative_tests.log`

## Limitaciones

- Parser de vuelta es subconjunto (no CST completo).
- GUI de timeline/bloques visuales aún no; el **modelo y el emit** son el path real de Studio simplified.
