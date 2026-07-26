# RPG Quest: La liberación de Solaria

> [!IMPORTANT]
> Los recursos gráficos no se distribuyen con el repositorio. Coloca un pack propio y
> debidamente licenciado en `assets/`; sin él, la demo conserva sus fallbacks procedurales.
> Más adelante se podrá documentar un pack redistribuible independiente.

Demo RPG 2D top-down construida en Rust sobre **Velvet Engine**. Combina exploración, diálogos, misión por etapas, inventario, progresión y combate en tiempo real usando los sistemas de `velvet-play` y `velvet-rpg`.

## Contenido jugable

La misión principal avanza en cinco etapas:

1. Hablar con Mira la Anciana.
2. Visitar a Kael para recibir un escudo y pociones.
3. Derrotar a tres exploradores que ocupan los caminos.
4. Vencer al capitán frente a la fortaleza roja.
5. Regresar con Mira para liberar Solaria.

La demo incluye:

- pantalla de título, victoria y derrota;
- mapa top-down con colisiones en edificios y límites;
- tres enemigos menores y un jefe con vida, persecución y ataques;
- ataque del jugador con alcance, orientación, retroceso y cooldown;
- daño, invulnerabilidad temporal, pociones y game over;
- inventario y equipo mediante `velvet-rpg`;
- oro, experiencia, subida de nivel y aumento de estadísticas;
- cofre opcional con recompensa única;
- diálogos contextuales e indicador de interacción;
- GUI pixel-art temática con paneles, barras, botones, ranuras e iconos;
- escalado con letterboxing y modo pantalla completa;
- capturas deterministas y prueba headless.

## Interfaz Runewood

La interfaz activa utiliza el tema **Runewood** de **Complete UI Essential Pack v2.4**. El diseño integra:

- marcos distintos para HUD, perfil, misión, diálogos, inventario y pantallas finales;
- barras Runewood con rellenos rojo, verde y dorado;
- ranuras normales, seleccionadas y bloqueadas;
- iconos temáticos de vida, oro, experiencia, aceptar, cancelar, jugar, reiniciar y volver al inicio;
- teclas visuales para explicar los controles;
- fuente pixel **Toriko** para títulos, botones y etiquetas;
- fuente del sistema para párrafos largos y mayor legibilidad.

Los marcos y botones se amplían mediante **nine-slice**, mientras que barras y títulos horizontales usan **three-slice**. De esta forma las esquinas, runas y remates del pixel art conservan su forma al cambiar de tamaño.

Si el usuario instala los recursos alternativos Wood y Paper en `assets/ui/`, puede
adaptarlos sin cambiar la lógica de la demo. La configuración actual espera Runewood.

## Controles

| Tecla | Acción |
|---|---|
| `WASD` / flechas | Mover a Valen |
| `E` | Hablar, abrir cofres o continuar diálogos |
| `Espacio` | Atacar o continuar diálogos |
| `H` | Beber una Poción de Vida |
| `I` | Abrir o cerrar el inventario |
| `F11` | Alternar pantalla completa |
| `R` | Reintentar después de victoria o derrota |
| `Esc` | Cerrar panel, volver al título o salir |

## Ejecución

Desde la raíz de Velvet Engine:

```bash
cargo run -p rpg-quest --release
```

También se puede iniciar con:

```text
demos/rpg-quest/run.bat
```

## Validación

```bash
cargo fmt --all -- --check
cargo check -p rpg-quest
cargo test -p rpg-quest
cargo clippy -p rpg-quest --all-targets -- -D warnings
cargo run -p rpg-quest -- --headless
```

## Capturas deterministas

```bash
cargo run -p rpg-quest -- --capture-screen gameplay artifacts/rpg-quest/gameplay.png
cargo run -p rpg-quest -- --capture-screen boss artifacts/rpg-quest/boss.png
```

Pantallas disponibles: `title`, `gameplay`, `dialogue`, `inventory`, `boss`, `victory` y `gameover`.

## Recursos gráficos y licencia

La demo utiliza:

- **Tiny Swords (Free Pack)**;
- **Icons Essential**;
- **Complete UI Essential Pack v2.4**, creado por Crusenho Agus Hennihuno.

El tema Runewood, sus alternativas y la fuente Toriko se distribuyen por sus autores bajo
**Creative Commons Attribution 4.0 International (CC BY 4.0)**. Estos binarios no forman
parte del repositorio. Al añadirlos localmente, conserva su licencia en:

```text
assets/ui/LICENSE-CC-BY-4.0.txt
```

Conserva también las licencias originales de cualquier pack propio junto a sus recursos.
