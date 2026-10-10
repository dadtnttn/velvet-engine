# velvet-three-kit

El kit toon de Three.js que comparten los prototipos de Demon Queen
(`demos/demon-queen-*-threejs`) y lo común de sus exportadores al juego en Rust. Antes cada
prototipo llevaba su copia en `src/kit/` (y las termas otra más en `src/vkit/`). Ahora hay una
sola, aquí, y cada prototipo la importa con una ruta relativa.

## Qué hay

| Fichero | Para qué |
|---|---|
| `toon.js` | El sombreado toon (`toonify(root)` sobre una escena, `toonOf(material)`): bandas, borde, brillo de tela, variación lenta de tono, y la ventana que deja ver a la Reina a través de lo que la tapa (`occlusion`; con radio 0 está apagada). `sourceOf` devuelve el material original de uno ya toon. |
| `outline.js` | El contorno de tinta por profundidad (`createOutline(composer, camera)`), que se adelgaza con la distancia (`fade`). |
| `bake.js` | El horneado (`bake(root)`, `mergeChildren(group)`): piezas fundidas por material y oclusión ambiental por vértice. |
| `textures.js`, `relief.js` | Texturas pintadas en canvas sin costuras, con relieve (mapas normales) sacado del color. |
| `geo.js` | Piezas y colocación (`rbox`, `box`, `cyl`, `lathe`, `extrude`, `put`, `at`…) y las luces y brillos de la decoración (`lampAt`, `glowAt`, `poolAt`, `collectDecor`). |
| `village/` | El aspecto pintado de la villa (`village-textures.js`) y su biblioteca de materiales (`materials.js`), que usan la villa, el bosque y las termas. |
| `export/run.mjs` | El exportador de Node: sirve el prototipo con Vite, lo abre en Chrome sin ventana, espera su gancho de depuración y escribe los GLB (con las mallas nombradas como sus nodos, que es lo que lee el juego) y los JSON del plano. |
| `export/gltf.js` | Lado de la página: el GLB binario de una escena, copias planas de las texturas, redondeo para los planos. |
| `export/merged.js` | Para escenas fundidas en una malla por material (castillo, termas). |
| `export/world.js` | Para escenas del mundo escritas pieza a pieza con los nombres de la biblioteca (bosque, villa). |

Lo propio de cada prototipo sigue en él: el burdel conserva su `src/kit/materials.js`, el bosque
sus `forest-materials.js` y `forest-textures.js`, y cada uno su `src/export/game-export.js`
(qué se exporta de su lugar y cómo).

## Cómo se usa

Desde un prototipo, con ruta relativa:

```js
import { toonify, sourceOf } from "../../../../tools/velvet-three-kit/toon.js";
```

El exportador de cada prototipo (`tools/export-game.mjs`) queda en una llamada:

```js
import { exportGame } from "../../../tools/velvet-three-kit/export/run.mjs";
await exportGame({ prototype: new URL("..", import.meta.url), debug: "castleDebug", port: 5259,
                   glb: "data/assets/3d/castle", data: "data/castle" });
```

`VELVET_EXPORT_ROOT=<carpeta>` manda la exportación a otra carpeta en vez de al juego, para
comparar dos exportaciones.

## Dependencias

Los prototipos comparten una sola instalación de `node_modules`: la de
`demon-queen-caves-threejs`, enlazada desde cada uno. El kit hace lo mismo: su `node_modules` es
un enlace (unión de Windows) a esa carpeta, así que `three`, `vite` y `playwright-core` son los
mismos módulos que ven los prototipos. En un clon nuevo, después de instalar los de las cuevas:

```bat
mklink /J tools\velvet-three-kit\node_modules demos\demon-queen-caves-threejs\node_modules
```

Los ficheros del kit están fuera de la raíz de cada prototipo, y Vite los sirve porque cada
`vite.config.js` ya permite todo el repositorio (`server.fs.allow`).
