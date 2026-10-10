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
| Ratón | Todo es clicable; los menús reaccionan al pasar el cursor |
| Espacio o Enter | Cumplir un año más |
| 1 – 5 | Elegir una opción en un evento |
| Enter / Espacio / Esc | Cerrar un mensaje |
| Esc | Volver o cerrar un menú |
| Rueda del ratón | Desplazar la historia de tu vida |
| Teclado (creador) | Escribir el nombre; Retroceso borra |

Las fuentes se cargan del sistema (`segoeui.ttf`, `segoeuib.ttf` e `impact.ttf`,
con Arial como respaldo), así que en Windows funciona sin instalar nada.

## Estilo

Inspirado en la interfaz de Persona 5: rojo, negro y blanco, paneles inclinados
con sombra, semitonos, estallidos de cómic, títulos ladeados con contorno grueso
y el logo en letras de "nota de rescate". Todo el arte es SVG generado por el
script (`art.vel`) y dibujado con la primitiva `svg` del runtime: no hay imágenes
binarias en el juego.

## Qué hay en el juego

- **Creador de personaje**: nombre (escrito o al azar), sexo, a quién le atrae,
  tono de piel, 8 peinados, 9 colores de pelo, ojos, ropa, accesorios (lentes,
  aretes, piercing, barba) y personalidad. El retrato se ve en vivo.
- **Retratos SVG con expresiones**: tu cara cambia con tu felicidad (eufórica,
  contenta, tranquila, triste, deprimida), envejece con canas y arrugas, y cada
  persona de tu vida tiene su propio retrato.
- **Personalidad**: Carismático/a, Rebelde, Cerebrito, Fiestero/a, Romántico/a o
  Ambicioso/a. Cambia tus estadísticas iniciales y te da ventajas en citas,
  crimen, estudio, fiestas, vínculos o trabajo.
- **Rasgos sociales al estilo Persona**: Conocimiento, Valor, Destreza,
  Amabilidad y Encanto, cada uno con 5 rangos con nombre. Suben con lo que haces
  y desbloquean opciones exclusivas en los eventos (por ejemplo, `[VALOR 3]`).
- **Confidentes**: cada vínculo tiene un arcano (Los Amantes, La Emperatriz, La
  Torre...) y un rango del 1 al 10 que sube con la cercanía.
- **Vida completa**: escuela, universidad con préstamo, aumentos, despidos,
  prisión con opción de fuga, matrimonio, divorcio, hijos y muerte.
- **150 carreras** (`jobs.vel`): 126 contratables en 15 campos (Servicio,
  Oficio, Oficina, Tecnología, Arte, Espectáculo, Salud, Ley, Ciencia, Negocios,
  Deporte, Educación, Moda, +18 y Calle) y 24 que solo se consiguen fusionando.
  Todas tienen 5 rangos, acciones propias de su campo y requisitos.
  - **Especializaciones** (354, de ellas 75 con versión +18) al llegar al rango 3:
    +25% de sueldo. Un ilustrador puede hacer arte erótico, un artista 3D modelos
    eróticos, un fotógrafo boudoir, un escritor novela erótica...
  - **Dos carreras a la vez**: la segunda es de media jornada (mitad de sueldo y
    experiencia) y tiene su propio turno.
  - **Fusiones** (42 combinaciones): con ambas carreras en rango 3 las combinas
    en una nueva que empieza en rango 3. Programador + Artista 3D = Desarrollador
    de juegos para adultos; Ilustrador + Escritor = Mangaka erótico; Médico +
    Psicólogo = Sexólogo; Actor porno + Actor = Director de cine para adultos;
    Corredor de bolsa + Emprendedor = Magnate, y muchas más.
- **Actividades por categorías**: Cuerpo, Mente y Lujos (viajes, lotería y
  bienes como motos, autos, departamento, mansión o yate).
- **Noche +18 por categorías**: Fiesta, Ligue, Placer (sex shop, love hotel,
  club swinger, fiesta de disfraces, spa nudista, crucero para adultos),
  Carreras, Vicios y Crimen. Encuentros con o
  sin protección (ITS y embarazos), drogas con adicción y sobredosis, casino y
  delitos, algunos bloqueados hasta tener rango social suficiente.
- **75 eventos aleatorios** con decisiones, opciones bloqueadas por rango y
  personalidades con mejor suerte en ciertas decisiones.
- **Carreras +18 con 5 rangos**: cine para adultos (Amateur → Leyenda +18),
  striptease (Debut → Leyenda del club), escort (Independiente → Agencia propia)
  y creador/a de contenido (100 fans → Un millón de fans). Se entra por casting
  en Noche +18 › Carreras, cada una tiene sus acciones, experiencia, ascensos,
  fama y eventos propios (rodajes, Premios Terciopelo Rojo, clientes VIP,
  operativos policiales, campeonatos de pole dance, transmisiones virales).
- **Gustos en la cama**: elige hasta 3 en el creador (se revelan a los 18).
  Cambian tus ligues, desbloquean opciones `[GUSTO]` en eventos (club de BDSM,
  fiestas atrevidas, cosplay...) y si compartes gustos con alguien la noche es
  mejor. Cada persona adulta tiene los suyos.
- **Misiones**: 26 objetivos con recompensa (graduarte, casarte, primer millón,
  ganar un premio, llevar un confidente a rango 10, vivir 100 años...).
- **Retratos animados y variados**: respiran, parpadean y reaccionan (salto con
  "!", corazón, gota de sudor, enojo o dinero). 3 tipos de cara, 3 de cuerpo,
  12 peinados, marcas (pecas, lunar, cicatriz) y labial.
- **Efectos**: corte de pantalla al cumplir años, entradas animadas de menús y
  diálogos, números que flotan al cambiar una estadística, notificaciones de
  "¡RANGO ARRIBA!" y obituario con tu retrato.

## Archivos

| Archivo | Contenido |
|---|---|
| `main.vel` | Contrato del runtime, puntero/hover, teclado, creador, animaciones |
| `sim.vel` | Estado de la vida, personalidad, rasgos sociales, paso de los años, muerte |
| `events.vel` | Eventos anuales (datos) y su motor |
| `acts.vel` | Menús: ocupación, vínculos, actividades, noche, citas, casino, compras |
| `jobs.vel` | Catálogo de 150 carreras, especializaciones, segunda carrera y fusiones |
| `career.vel` | Acciones de las carreras +18, gustos en la cama y misiones |
| `data.vel` | Nombres, ciudades, trabajos, personalidades, arcanos, paletas, bienes |
| `art.vel` | Generadores SVG: fondo, paneles, estallidos, logo y retratos |
| `ui.vel` | Pantallas, menús, diálogos, notificaciones y efectos |

## Agregar contenido

Los eventos y las actividades son datos. Para crear un evento nuevo, agrega un
mapa a `EVENTS` en `events.vel`; el comentario al inicio del archivo explica
cada campo (edad, peso, requisitos, resultados, opciones bloqueadas por rango y
marcadores de texto como `{x}`, `{a}` o `{p}`). Las actividades siguen el mismo
formato en `ACTIVITIES` dentro de `acts.vel`.

**Importante:** el compilador de paquetes VS3 exige un `velvet.lock` al día.
Después de editar cualquier `.vel`, regenera el lock:

```powershell
cargo run -p velvet-cli --release -- vs3 lock demos/vida-sin-filtro
cargo run -p velvet-cli --release -- app check demos/vida-sin-filtro
```

## Limitaciones actuales

- No hay guardado: el runtime VS3 no expone almacenamiento todavía.
- Sin sonido: el runtime VS3 aún no tiene audio.
- El teclado no escribe acentos en el nombre (los nombres al azar sí los llevan).
