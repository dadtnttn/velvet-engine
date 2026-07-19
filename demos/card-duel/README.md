# Card Duel — demo de menús + duelo de cartas

Demo **con ventana** sobre Velvet Engine y las herramientas `velvet-cards`
(catálogo, mazo, zonas library/hand/discard). No es un TCG completo: es un
esqueleto jugable con pantallas de menú.

## Pantallas

| Pantalla | Contenido |
|----------|-----------|
| **Title** | Iniciar duelo · Cómo jugar · Salir |
| **How to Play** | Controles y reglas básicas |
| **Battle** | HP, energía, mano, log, zonas |
| **Pause** | Continuar · Reiniciar · Menú |
| **Result** | Victoria / derrota · Revancha · Menú |

## Run

```bash
cargo run -p card-duel --release
cargo run -p card-duel --release -- --headless
```

O `run.bat` desde la raíz del repo.

## Controles

| Input | Acción |
|-------|--------|
| ↑↓ / W S | Mover selección en menús |
| Enter / Space | Confirmar |
| 1–6 | Jugar carta de la mano |
| E | Fin de turno (enemigo ataca) |
| Esc | Pausa / volver / salir (según pantalla) |

## Datos

- `data/catalog.json` — definiciones de cartas  
- `data/deck.json` — mazo inicial (validado con `validate_deck`)

La partida usa `CardZones::shuffle_library`, `draw`, `discard_from_hand`.
