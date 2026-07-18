# Criterios de aceptación medibles (candidato 0.2)

Cada ítem debe poder verificarse con un comando o un checklist de UI.

## Compilación y calidad

- [ ] `cargo build --workspace --all-features` exit 0  
- [ ] `cargo test --workspace --all-features` exit 0  
- [ ] `cargo fmt --all --check` exit 0  
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0  
- [ ] `tokei` report documentado sin conteo de `target/`

## Flujo creador (prioridad)

- [ ] `velvet-studio new` o wizard crea proyecto desde plantilla VN  
- [ ] Abrir proyecto lista scripts/assets  
- [ ] Cambiar fondo / botón (visual o archivo) y guardar  
- [ ] Conectar “Iniciar” → escena intro  
- [ ] Crear diálogo + decisión  
- [ ] `velvet run` / play desde Studio  
- [ ] Guardar y cargar partida  
- [ ] `velvet export` produce carpeta ejecutable  
- [ ] Ejecutar juego **fuera** del árbol del motor  

## Round-trip

- [ ] Archivo con bloque `@advanced` sobrevive edición visual del `@visual`  
- [ ] Test automatizado open→edit→save→reopen  

## LSP

- [ ] Proceso stdio responde `initialize` / `textDocument/publishDiagnostics`  
- [ ] Config VS Code abre `.vel` con diagnósticos  

## Demos

- [ ] visual-novel plantilla: varios finales  
- [ ] top-down-rpg plantilla: quest+diálogo  
- [ ] action plantilla: combate+score  
- [ ] hybrid: narrativa afecta mapa  

## Honestidad

- [ ] `docs/reports/LIMITATIONS.md` actualizado  
- [ ] Ninguna feature “completa” con solo structs vacíos  

## Estado actual vs meta

| Criterio | Hoy |
|----------|-----|
| Build/test workspace | Sí (lib/all-features verificado) |
| Studio visual dual mode | No |
| Round-trip | No |
| LSP stdio | No (NDJSON) |
| 4 plantillas reales ricas | Parcial |
| Export out-of-tree | Parcial |
