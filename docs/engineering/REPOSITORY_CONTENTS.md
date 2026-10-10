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
| `demon-queen` | Juego de referencia: Rust, historia, editor y herramientas |

Las demás demos y los prototipos Three.js quedan en disco, ignorados y fuera de
los miembros de Cargo. Los ejemplos bajo `examples/` siguen disponibles.
El test de estilos del casino usa una fixture dentro de `velvet-style`, sin
depender de una demo excluida.

## Demon Queen

Se publica código y datos narrativos. Los únicos medios públicos son los
archivos enumerados en `tools/repository-policy.json` y el pack KayKit Skeletons
CC0 con su licencia. Se revisaron visualmente iconos, marcos, objetos de tienda
y mapas; también las escenas pixel art de carga y sus referencias. Las CG,
ilustraciones de personajes, referencias de producción, vídeos,
música y demás modelos quedan privados. Los nuevos medios están ignorados por
defecto; para incluirlos hay que revisar contenido y actualizar ambas listas.

El build normal no genera el bundle portable ni necesita las CG privadas.
El build `--features portable` exige el pack privado y falla con un diagnóstico
si falta. CI comprueba el código de Demon Queen sin esa feature. Las pruebas,
Clippy y documentación del motor con todas las features excluyen los paquetes
`demon-queen` y `demon-queen-dungeon`, cuya QA completa depende de recursos locales.

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

Los commits locales anteriores a esta limpieza contienen medios privados y un
ejecutable que supera el límite de GitHub. Para publicar, se conserva ese historial
en las ramas locales de respaldo y se crea un commit con el árbol público completo
sobre `origin/main`. Así se actualiza `main` por avance directo, sin publicar los
artefactos locales históricos ni reescribir el historial remoto.
