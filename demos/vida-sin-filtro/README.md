# Vida Sin Filtro

Simulador de vida para adultos, al estilo de BitLife, hecho completamente en
**Velvet Script 3** sobre el runtime declarativo `velvet-vs3-app`. No hay código
Rust propio: el motor pone la ventana, el input y el dibujo 2D, y toda la lógica,
el contenido y la interfaz viven en los `.vel` de esta carpeta.

> +18: contiene sexo (sin escenas explícitas), drogas, alcohol, crimen y malas decisiones.

## Jugar

Desde la raíz del repositorio:

```powershell
cargo run -p velvet-cli --release -- app run demos/vida-sin-filtro
```

Controles:

| Tecla / ratón | Acción |
|---|---|
| Espacio o Enter | Cumplir un año más |
| 1 – 5 | Elegir una opción en un evento |
| Enter / Espacio / Esc | Cerrar un mensaje |
| Esc | Cerrar un menú |
| Rueda del ratón | Desplazar la historia de tu vida |

Las fuentes se cargan del sistema (`segoeui.ttf` y `segoeuib.ttf`, con Arial
como respaldo), así que en Windows funciona sin instalar nada.

## Qué hay en el juego

- **Vida completa**: nacimiento, infancia, escuela, universidad, trabajo, vejez y muerte.
- **Estadísticas**: felicidad, salud, inteligencia y atractivo, más karma oculto.
- **Ocupación**: escuela, secundaria abierta, universidad con préstamo, 16 trabajos
  (incluye bailarín/a exótico/a, modelo de lencería y creador/a de contenido +18,
  cuyo sueldo depende del atractivo), aumentos, despidos y prisión.
- **Relaciones**: padres, hermanos, parejas, matrimonio, divorcio (pierdes la mitad),
  amantes, ex, hijos; cercanía con cada persona.
- **Noche +18**: discoteca, bares, app de citas, encuentros casuales con o sin
  protección (ITS y embarazos), striptease, marihuana, cocaína, éxtasis, casino y delitos.
- **Consecuencias**: adicciones que dañan la salud, sobredosis, infecciones que
  hay que tratar, infidelidades descubiertas, hijos inesperados y condenas.
- **30 eventos aleatorios** con decisiones, y una pantalla de obituario al morir.

## Archivos

| Archivo | Contenido |
|---|---|
| `main.vel` | Contrato del runtime (`app_manifest`, `app_init`, `app_update`, `app_action`), teclado y acciones |
| `sim.vel` | Estado de la vida, paso de los años, efectos, muerte |
| `events.vel` | Eventos anuales (datos) y su motor |
| `acts.vel` | Menús: ocupación, relaciones, actividades, noche, citas, casino |
| `data.vel` | Nombres, ciudades, trabajos e infecciones |
| `ui.vel` | Dibujo: portada, pantalla de juego, ventanas y obituario |

## Agregar contenido

Los eventos y las actividades son datos. Para crear un evento nuevo, agrega un
mapa a `EVENTS` en `events.vel`; el comentario al inicio del archivo explica
cada campo (edad, peso, requisitos, resultados y marcadores de texto como
`{x}`, `{a}` o `{p}`). Las actividades simples siguen el mismo formato en
`ACTIVITIES` dentro de `acts.vel`.

**Importante:** el compilador de paquetes VS3 exige un `velvet.lock` al día.
Después de editar cualquier `.vel`, regenera el lock:

```powershell
cargo run -p velvet-cli --release -- vs3 lock demos/vida-sin-filtro
cargo run -p velvet-cli --release -- app check demos/vida-sin-filtro
```

## Limitaciones actuales

- No hay guardado: el runtime VS3 no expone almacenamiento todavía.
- Sin imágenes ni sonido; todo se dibuja con primitivas 2D.
