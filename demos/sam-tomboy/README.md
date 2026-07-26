# Sam: Honest Stranger

> [!IMPORTANT]
> Los fondos, personajes, audio y material de entrenamiento son locales y no forman parte
> del repositorio. Puedes usar recursos propios compatibles bajo `data/assets/`; la demo
> mantiene una presentación procedural cuando faltan. Podrá publicarse otro pack más adelante.

Demo narrativa para Velvet Engine que mezcla **novela visual, mapa y supervivencia social ligera**.

Sam es una mujer adulta procedente de un mundo donde las mentiras no existen. Llega sola a una ciudad humana con 6 €, sin documentos, trabajo ni alojamiento. Durante su primer día debe conseguir comida, ganarse la confianza de otras personas y decidir dónde pasará la noche sin traicionar su forma de ser.

## Contenido de la demo

- Menú principal con nueva partida, continuar, extras y salida.
- Introducción dentro de un tren.
- Mapa navegable de la ciudad.
- Cuatro locaciones principales:
  - Plaza central.
  - Café Lucero.
  - Taller Norte.
  - Pensión Azul.
- Estación de tren como quinta locación narrativa.
- Dinero, hambre, energía, confianza, tiempo y refugio.
- Decisiones bloqueadas por recursos o condiciones.
- Guardado automático y opción de continuar.
- Tres cierres:
  - **Mañana hay trabajo**.
  - **Una puerta cerrada**.
  - **La primera noche**.
- Vestidor con poses, ropa y expresiones.
- Galería adulta separada de la historia principal.
- Reproductor de 30 imágenes a aproximadamente 30 FPS.

## Controles

### Ratón

- Pulsa botones, decisiones y nodos del mapa.
- Los botones cambian visualmente al pasar el puntero.

### Teclado

- `↑` / `↓`: cambiar la decisión seleccionada.
- `Enter` o `Espacio`: confirmar.
- `Escape`: volver al mapa o al menú.
- `←` / `→`: navegar en los extras.
- `F11`: alternar pantalla completa.

## Ejecución

Desde la raíz de Velvet Engine:

```bash
cargo run -p sam-tomboy
```

En Windows también se puede usar:

```text
demos\sam-tomboy\run.bat
```

## Prueba headless

La ruta automatizada valida el módulo VS3, completa una partida con trabajo y alojamiento y comprueba el round-trip de serialización:

```bash
cargo run -p sam-tomboy -- --headless
```

Salida esperada:

```text
SAM HONEST STRANGER HEADLESS OK
ending=HonestWork
save_roundtrip=ok
vs3_check=ok
```

## Capturas deterministas

```bash
cargo run -p sam-tomboy -- --capture-screen menu artifacts/sam-tomboy/menu.png
cargo run -p sam-tomboy -- --capture-screen intro artifacts/sam-tomboy/intro.png
cargo run -p sam-tomboy -- --capture-screen map artifacts/sam-tomboy/map.png
cargo run -p sam-tomboy -- --capture-screen cafe artifacts/sam-tomboy/cafe.png
cargo run -p sam-tomboy -- --capture-screen garage artifacts/sam-tomboy/garage.png
cargo run -p sam-tomboy -- --capture-screen boarding artifacts/sam-tomboy/boarding.png
cargo run -p sam-tomboy -- --capture-screen gallery artifacts/sam-tomboy/gallery.png
cargo run -p sam-tomboy -- --capture-screen ending artifacts/sam-tomboy/ending.png
```

Pantallas admitidas:

```text
menu, intro, ticket, map, plaza, cafe, garage, boarding, gallery, ending
```

## Fondos SVG temporales

Los fondos editables están en:

```text
data/assets/backgrounds/svg/
```

Las versiones rasterizadas utilizadas durante la ejecución están en:

```text
data/assets/backgrounds/png/
```

Para regenerar ambas colecciones:

```bash
python demos/sam-tomboy/tools/generate_backgrounds.py
```

Los SVG son fondos temporales diseñados para poder reemplazarse posteriormente sin cambiar la lógica del juego.

## Assets de Sam

El compositor combina en tiempo real:

- ocho poses base;
- expresiones faciales;
- camiseta y hoodie;
- ropa casual por capas;
- ropa de taller por capas;
- bikini y ropa interior dentro de Extras;
- tres ilustraciones con dos variantes cada una;
- treinta frames de animación.

La historia principal utiliza ropa contextual: hoodie en la estación y durante la noche, ropa casual en el café y uniforme de trabajo en el taller.

## Arquitectura

```text
src/main.rs       Ventana, navegación, input, menú, mapa y presentación.
src/model.rs      Estado, escenas, decisiones, economía y finales.
src/assets.rs     Caché, carga y composición por capas.
src/render.rs     Framebuffer, texto TTF, imágenes, botones y HUD.
src/save.rs       Guardado JSON seguro en el perfil del usuario.
story/sam_logic.vel  Módulo VS3 válido con el modelo de supervivencia.
```

La versión actual es híbrida: el host gráfico y el flujo completo viven en Rust, mientras `sam_logic.vel` mantiene una referencia válida del estado y las operaciones que se migrarán a VS3 cuando su capa de UI y eventos esté ampliada.

## Guardado

En Windows se almacena en:

```text
%APPDATA%\VelvetEngine\SamHonestStranger\save.json
```

El guardado conserva recursos, progreso narrativo, locaciones visitadas, trabajo, alojamiento y final.

## Validación de desarrollo

```bash
cargo fmt --all -- --check
cargo check -p sam-tomboy
cargo test -p sam-tomboy
cargo clippy -p sam-tomboy --all-targets -- -D warnings
cargo run -p velvet-cli -- vs3 check demos/sam-tomboy/story/sam_logic.vel
cargo run -p sam-tomboy -- --headless
```

## Próximas ampliaciones naturales

- Sustituir los fondos SVG por ilustraciones definitivas.
- Añadir NPCs visuales y retratos propios.
- Introducir varios días y alquiler recurrente.
- Ampliar empleos, alimentación y reputación por barrio.
- Migrar navegación, escenas y estado autoritativo a VS3 cuando estén disponibles sus nuevas APIs de UI.
