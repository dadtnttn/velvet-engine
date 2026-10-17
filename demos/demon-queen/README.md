# Aquella Vez que un Infarto me Reencarnó como la Reina Demonio más Temida del Continente Oscuro

Aventura narrativa 18+ hecha con Velvet Engine, el juego de referencia del motor. Empieza como
novela visual (el prólogo) y sigue en un mundo 3D en el mismo proceso y la misma ventana: el
castillo y su corte, la villa, el Bosque Oscuro, las termas, el burdel, el Peñascal de Varrak, la
mina (un roguelike en primera persona) y la mesa de guerra, con batallas en tiempo real y una
campaña por turnos.

Desarrollado por **Velvet Weaver**. *Woven with Velvet Engine.*

> Hi F95zone staff! I'm **Velvet Weaver**, the developer of this game
> ([thread 312490](https://f95zone.to/threads/312490/)). This is its source code, built on my own
> engine. I added this note on 2026-10-15 as proof for my Game Developer badge ticket.

## Qué hay en este repositorio

Solo el código. La historia y sus textos (guiones, catálogos de idioma, misiones, la corte), el
arte (modelos, CG, vídeos, música, fuentes) y las herramientas de producción viven en la máquina de
desarrollo y están ignorados; ver
[contenido del repositorio](../../docs/engineering/REPOSITORY_CONTENTS.md). Por eso el juego no se
ejecuta desde un clon: compila, pero necesita el pack privado bajo `data/`.

| Ruta | Qué es |
|---|---|
| `src/main.rs` | El anfitrión: título, novela visual, menús (pausa, ajustes, galería, mochila), guardados y el paso al mundo 3D |
| `src/exploration3d.rs` | El mundo 3D: estado, render y bucle; sus partes en `src/explore/` (entrada, cámara, la Reina, la corte, las citas, la guía…) y `src/zones/` (una por lugar: castillo, bosque, villa, termas, burdel, Peñascal, mina, combate con Varrak, guerra) |
| `src/overlay/` | El HUD por GPU: mapa, diálogos, escenas, la corte, la guerra y la pintura común de interfaz (`ui_paint.rs`, `persona.rs`) |
| `src/bin/` | Solo binarios extra: `demon-queen-3d` (el mundo 3D solo, para QA y capturas) y `dungeon-lab` |
| `dungeon/` | El crate de la mina (`demon-queen-dungeon`) |
| `war/` | El crate de la guerra (`demon-queen-war`): simulación de batalla, IA, reglas de campaña, campos y escenografía |
| `story/schema.json` | El esquema de las historias JSON |
| `build.rs` | Recursos de Windows y versión |

## Reglas de render

Todo se diseña sobre un lienzo lógico de **1920×1080** y se dibuja a la resolución real de la
pantalla (Ajustes → Pantalla → «Resolución nativa»). La ventana se redimensiona con letterbox 16:9,
sin deformar, y el ratón invierte exactamente ese encuadre. El hover y el foco cambian luz, opacidad
o color, nunca la geometría.

## Idiomas

Quince: español (el original), inglés, portugués de Brasil, alemán, francés, italiano, polaco, ruso,
chino simplificado y tradicional, japonés, coreano, hebreo, hindi y tailandés. El texto se traduce
al dibujarse a partir de catálogos (`data/locales/text/<idioma>/*.json`, privados).

## Compilar y probar

```powershell
# Desde la raíz del motor
cargo check -p demon-queen --all-targets
cargo test -p demon-queen-war                 # reglas, simulación y mapas de la guerra
cargo test -p demon-queen-war -- --ignored    # sus modelos (necesita el pack privado)

# Con el pack privado
cargo build --profile fast-release -p demon-queen
cargo test --profile fast-release -p demon-queen --bin demon-queen
cargo test --profile fast-release -p demon-queen-dungeon
```

Capturas de QA del mundo 3D (desde `demos/demon-queen`, con guardados aislados):

```powershell
$env:DEMON_QUEEN_SAVES = "temp/qa-saves"
../../target/fast-release/demon-queen-3d.exe --capture-toon temp/qa --war table      # o vado, campaign…
```

`VELVET_LANG=<idioma>` fuerza un idioma; `VELVET_RENDER_SIZE=2560x1440` fuerza el tamaño de las
capturas.

## Guardados

Partidas, galería y preferencias viven en `%APPDATA%\VelvetEngine\DemonQueen`
(`DEMON_QUEEN_SAVES` lo cambia para pruebas), así que mover o actualizar el ejecutable no borra el
progreso.
