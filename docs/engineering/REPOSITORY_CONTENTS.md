# Contenido del repositorio

El workspace publicado contiene el motor, ejemplos, templates, samples y siete
demos. `Cargo.toml`, `.gitignore` y `tools/repository-policy.json` deben coincidir.

| Demo | Uso |
|---|---|
| `cellular-arena` | Materiales y simulación celular |
| `fx-forge` | Efectos procedurales |
| `ui-gallery` | Componentes y animación de interfaz |
| `terraflow` | Terreno y líquidos; imágenes de salida locales |
| `third-person` | Escenas 3D y control de personaje |
| `rpg-quest-vs3` | Host de referencia para proyectos `.vel` |
| `demon-queen` | Juego de referencia: solo código (historia, datos y arte privados) |

Las demás demos y los prototipos Three.js quedan en disco, ignorados y fuera de
los miembros de Cargo. Los ejemplos bajo `examples/` siguen disponibles.
El test de estilos del casino usa una fixture dentro de `velvet-style`, sin
depender de una demo excluida.

## Demon Queen

Desde el 2026-10-15 se publica solo el código: `src/`, `dungeon/`, `war/`, `build.rs`, el
esquema de la historia (`story/schema.json`), los planos de las zonas y los estilos
que el código lee. La historia y sus textos (catálogos de idioma, diálogos,
misiones, locales), el arte (modelos, iconos, escenas de carga, mapas, fuentes),
las herramientas y los documentos internos quedan en la máquina de desarrollo,
ignorados por `.gitignore`, y también se retiraron del historial publicado.
`tools/repository-policy.json` marca la demo como `source_only_demos`.

Por eso el juego no se ejecuta solo con este repositorio: CI comprueba, prueba y
documenta el motor y excluye los paquetes `demon-queen` y `demon-queen-dungeon`,
cuya compilación y QA dependen de los datos privados. `demon-queen-war` sí entra en CI:
sus reglas, simulación y mapas son datos en código; los dos tests que leen sus modelos
están marcados `#[ignore]` y se ejecutan en local con `--ignored`.

La corte del castillo (`data/castillo/`: audiencias, lecciones, citas) es texto de la
historia y también queda privada.

## Artefactos locales

Logs, `temp/`, `fx_export/`, capturas, builds, guardados, experimentos de arte y
configuración del equipo no se publican. Retirar un archivo del índice con
`git rm --cached` conserva el archivo de trabajo. La configuración personal de
Arch Linux y las antiguas salidas de FX se guardan bajo `.local/`.
FX Forge escribe las nuevas exportaciones en `demos/fx-forge/output/`, sin
dejar imágenes en la raíz del motor.

```powershell
python tools/check_repository.py
```

La comprobación usa el índice de Git: rechaza archivos rastreados que están
ignorados, demos excluidas, medios no revisados y miembros de Cargo ausentes.
Se ejecuta también en CI.

Los commits locales anteriores a la limpieza de octubre contienen medios privados y
datos personales. Ese historial solo vive en ramas locales de respaldo, que no se
publican; el historial de GitHub se reescribió (14 y 15 de octubre de 2026) para
retirar del pasado la historia, los textos, las herramientas, el arte y esos datos.
