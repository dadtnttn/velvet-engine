# Cellular Arena GIANT — mini juego demo

Sandbox **grande y legible** encima de **velvet-cellular**:
jugador con cabeza, torso, brazos (apuntan al ratón), piernas con animación,
enemigos con ojos, partículas densas (FX + sim), HUD de HP/tiempo/hechizos.

## Controles

| Tecla / ratón | Acción |
|---------------|--------|
| **A / D** o flechas | Mover |
| **W / Espacio** | Saltar |
| **Clic izquierdo** | Cavar hacia el cursor |
| **Clic derecho** | Colocar piedra |
| **F** / clic rueda | Lanzar hechizo |
| **1** | Chispas (`spark_bolt`) |
| **2** | Agua (`water_ball`) |
| **3** | Excavación (`digging_blast`) |
| **R** | Reiniciar |
| **Esc** | Salir |

## Objetivo

Mata a todos los slimes/brutes **o** sobrevive **120 s**. HP a 0 = derrota.

## Lanzar (recomendado en Windows)

Doble clic o desde una consola **en la raíz del repo**:

```bat
demos\cellular-arena\run.bat
```

O:

```bat
cargo run -p cellular-arena --release
```

Si ves `error 0x800700e8` al arrancar desde una terminal integrada, es un fallo de **tubería de la terminal**, no del juego. Usa `run.bat` o:

```bat
start "" target\release\cellular-arena.exe
```

Headless smoke:

```bash
cargo run -p cellular-arena --release -- --headless
```
