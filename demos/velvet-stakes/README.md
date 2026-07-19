# Velvet Stakes — **demo** (no es una tool del engine)

Ejemplo jugable estilo Balatro que **compone** herramientas (`velvet-cards`
zones + evaluación de manos en este binario). El API del engine son las tools,
no este loop de ciegas.

Fan / pre-alpha — no afiliado a LocalThunk ni a Balatro.

## Pantallas

| Pantalla | |
|----------|--|
| Title | Nueva run · Cómo jugar · Salir |
| Blind info | Target, manos, descartes |
| Play | Selección de cartas, preview de mano, score |
| Pause | Continuar · Menú |
| Result | Siguiente ciega / reintentar / menú |

## Run

```bash
cargo run -p velvet-stakes --release
cargo run -p velvet-stakes -- --headless
```

## Controles (juego)

| Tecla | Acción |
|-------|--------|
| 1–8 | Seleccionar / deseleccionar carta |
| P / Enter | Jugar mano (máx. 5 cartas) |
| D | Descartar selección |
| Esc | Pausa |

## Ciegas

1. Small Blind — 300 chips  
2. Big Blind — 800  
3. Boss Blind — 1600  

Manos de póker: High Card → Royal Flush (tabla base simplificada + valor de caras).
