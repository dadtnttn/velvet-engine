# Velvet Stakes — demo estilo Balatro

Demo **fan / pre-alpha** de poker con puntuación **chips × mult** y **ciegas**
(blinds). No está afiliado a LocalThunk ni a Balatro.

Usa `velvet-cards` para mazo / mano / descarte y ventana softbuffer.

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
