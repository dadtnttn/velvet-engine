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
| Espacio o Enter | Avanzar un mes |
| Y | Avanzar hasta tu próximo cumpleaños (se detiene si hay una decisión) |
| 1 – 9, flechas, letras, Espacio | Jugar los minijuegos |
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
- **Tiempo mes a mes**: cada mes cobras, trabajas turnos, pagas colegiatura y
  tomas decisiones; los embarazos duran nueve meses. El botón +1 AÑO avanza
  hasta tu cumpleaños y se detiene cuando algo requiere una decisión.
- **Familia desde el nacimiento**: naces en una familia humilde, de clase media o
  adinerada, y cada padre tiene un estilo (amoroso, estricto, relajado, ausente o
  tóxico) que afecta tu felicidad, tus calificaciones y si te pagará la carrera.
- **Escuela y estudios** (`edu.vel`): calificaciones desde la primaria, exámenes
  semestrales (minijuego, copiar o faltar) y 36 carreras para estudiar (20
  universitarias y 16 técnicas) con colegiatura mensual. Pagan tus padres (si los
  convences), una beca según tus calificaciones, un préstamo o tú con tu trabajo.
  Sin pagar, te dan de baja; con mal promedio, pierdes la beca o repites semestre.
- **Títulos y amateurs**: muchas carreras piden título. Las reguladas (médico,
  abogado, policía...) solo con título; en las demás puedes entrar como amateur
  pasando una audición con minijuego: cobras el 60%, avanzas más lento, practicas
  con minijuegos y puedes certificarte en rango 3.
- **100 chambas de medio tiempo** (`gigs.vel`) en 14 lugares (comida, vida
  nocturna, construcción, tiendas, reparto, hogar, cuidado, oficina, hotel,
  eventos, gimnasio, fábrica, belleza y en línea +18). Sin experiencia y
  compatibles con estudiar. Cada turno es un minijuego que decide tus propinas,
  cada mes hay una minimisión con bono, y hay 24 eventos de trabajo: clientes
  imposibles, asaltos, huelgas, ascensos... y a partir de los 18, propuestas
  picantes que siempre puedes aceptar, rechazar o denunciar (nunca en trabajos
  de cuidado).
- **7 minijuegos** (`minigames.vel`): bartender (prepara tragos), memoria de
  pedidos, cambio exacto, precisión, ritmo con flechas, tecleo rápido y examen
  de preguntas.
- **Conversaciones con decisiones** (`dialogue.vel`): diálogos ramificados con
  tus padres, tu pareja, amigos, hermanos y ex. Cada persona tiene un estilo
  (dulce, celoso, espíritu libre, intenso...) que cambia lo que dice y lo fácil
  que es convencerla. Las opciones muestran su probabilidad, algunas piden rango
  social, y pueden terminar en becas, préstamos, bodas, mudarse juntos, una
  relación abierta, confesiones de infidelidad o reconciliaciones.
- **Salud** (`health.vel`): 27 enfermedades. ITS de leves a extremas (ladillas,
  candidiasis, clamidia, gonorrea, VPH, herpes, sífilis, hepatitis B, VIH), con
  contagio persona a persona: cada adulto tiene sus propias infecciones, así que
  con una pareja limpia y fiel no hay riesgo, pero si se acuesta con alguien
  infectado te lo puede pasar. Más gente distinta, más riesgo; el condón lo baja
  mucho (no del todo con herpes, VPH y ladillas). También crónicas y cáncer
  ligados a lo que haces: diabetes, hipertensión, corazón, EPOC, cirrosis,
  obesidad, depresión, Alzheimer, artritis y 7 tipos de cáncer con etapas. Lo
  que no te diagnostican da síntomas; el médico lo detecta y lo tratas (cura,
  control mensual o quimioterapia).
- **El mundo se mueve**: tu pareja puede engañarte (más si es espíritu libre, si
  la relación va mal o si es abierta) y traerte algo; tus ex y amigos se casan,
  la gente se contagia y se cura sin ti.
- **Hábitos** (Actividades › Hábitos): 6 dañinos (fumar, vapear, beber a diario,
  comida chatarra, desvelarte, broncearte sin bloqueador) y 8 buenos (gimnasio,
  correr, comer sano, meditar, leer, dormir 8 horas, condón siempre, chequeo
  anual). Cambian tu riesgo de cada enfermedad; los adictivos cuestan dejarlos.
- **Efectos** (`effects.vel`): ventajas y desventajas que salen de todo lo
  anterior y modifican el resto del juego. Tierno de bebé (+atractivo), acné en
  la adolescencia, cuerpo definido tras un año de gimnasio, aliento a cigarro,
  ojeras, sobrepeso, brote de herpes, quimioterapia, enamorado, corazón roto,
  famoso, antecedentes penales... y una ventaja por cada gusto en la cama (fan
  del sexo oral: más éxito ligando pero más riesgo de ITS; exhibicionista: liga
  más pero cuesta que te contraten). Afectan tu atractivo efectivo, ligues,
  contrataciones, sueldo, calificaciones, placer y felicidad o salud mensual.
- **Vínculos a fondo** (`relations.vel`): el menú de Vínculos se divide en
  Familia, Pareja, Amigos, Otros y Conocer gente, y cada persona tiene su propio
  menú por categorías según el tipo de relación:
  - Pareja: Citas (cena, cine, picnic, concierto, baile, escapada, regalos),
    Intimidad (relaciones con condón, anticonceptivo, sin protección o buscando
    bebé; noche de fantasías según sus gustos; masaje; love hotel; trío),
    Juntos (ejercicio, cocinar, trivia, videojuegos, series, viajes, mascota),
    Hablar y Futuro (boda a elegir: íntima, civil, Las Vegas, playa o la boda
    del año; renovar votos; adopción; tratamiento de fertilidad; divorcio).
  - Padres, hermanos, hijos, nietos, amigos y ex tienen sus propias opciones
    (ayudar en casa, cuidarlos en la vejez, leer un cuento, enseñar a manejar,
    malcriar a los nietos, pijamadas, fiestas, bromas, pedir perdón, bloquear...).
  - Muchas actividades son minijuegos: tu puntaje decide cuánto crece el vínculo.
  - **Fertilidad**: cada persona tiene la suya y baja con la edad (antes en
    mujeres), el cigarro, la obesidad, las drogas y algunas ITS. La probabilidad
    de embarazo depende de la protección y de la fertilidad de los dos.
  - **Amigos de la infancia**: haces amigos en el kínder, la primaria y la
    secundaria que crecen contigo; de adultos pueden confesarte que siempre les
    gustaste, y llegar juntos hasta la boda y la vejez.
  - Eventos de pareja y amigos: aniversarios, conocer a los suegros, celos,
    despidos, discursos de boda y rivalidad entre hermanos.
- **Platicar** (`chat.vel`): con cualquier persona, como en una app de mensajes.
  Cada quien tiene un temperamento (callado, parlanchín, bromista, sensible,
  orgulloso, romántico) y un ánimo que cambia cada mes (feliz, cariñoso, triste,
  enojado, estresado, aburrido). Eliges qué decir (preguntar por su día,
  consolar, contar un chiste, coquetear, recordar algo, chismear, pedir consejo,
  pedir historias a los abuelos...) y responde según los dos: a quien está
  triste no le cuentes chistes; alguien callado y molesto puede negarse a hablar,
  y tú decides si insistir o darle su espacio. Tu pareja también te escribe por
  su cuenta.
- **Familia extendida**: abuelos, tíos y primos desde que naces, con sus propias
  actividades (visitas, comida familiar, historias de los abuelos, «domingo»,
  jugar con los primos) y eventos (Navidad, hospital, fiestas, consejos del tío).
- **La gran carrera**: al buscar un bebé juegas un minijuego en el que guías a
  tu campeón hasta el óvulo esquivando glóbulos blancos y tomando turbos; llegar
  primero aumenta mucho la probabilidad de embarazo.
- **Tríos y algo más**: invitar a otra mujer o a otro hombre, intercambio de
  parejas o una fiesta privada para adultos. Que acepte depende de su carácter,
  sus gustos y su confianza, y puede haber celos después.
- **Más en pareja**: karaoke, ver las estrellas, spa, bañarse juntos, mensajes
  picantes (que se pueden filtrar), juego de rol, hacer las paces, dormir
  abrazados, desayuno en la cama y clases de baile.
- **Hijos y legado** (`family.vel`): tus hijos tienen personalidad (dulce,
  rebelde, genio, flojo), calificaciones y te piden cosas (juguetes, permiso para
  fiestas, el auto, que les pagues la universidad, dinero, la boda). Puedes
  ayudarles con la tarea, castigarlos, echarlos de la casa, hablar con ellos o
  pagarles los estudios; todo cuenta para tu puntaje como padre o madre y para
  lo que sienten por ti. Crecen, trabajan, se casan y te dan nietos; si te
  quieren, te cuidan en la vejez (si no, asilo).
- **Herencia y nueva generación**: nombra a un heredero en vida o, si no, una
  ruleta decide entre tus hijos (quien mejor te trató ocupa más espacio). El
  heredero recibe más o menos herencia según cómo te trató, y puedes seguir
  jugando como esa persona, con tu familia, hermanos y nietos.
- **Vejez y funeral**: jubilación con pensión, cuidados, y un funeral al que va
  más o menos gente según tus vínculos y tu fama.
- **Tu pasado te persigue**: si trabajaste en la industria +18, años después
  pueden reconocerte en la junta de padres, aparecer tus videos en la oficina,
  ofrecerte un documental o memorias, o un amigo adulto de tu hijo confesarte que
  eres su fantasía. Y puedes regresar como estrella madura (MILF/DILF).
- **Vida completa**: aumentos, despidos, prisión con opción de fuga,
  matrimonio, divorcio, hijos y muerte.
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
- **Misiones**: 44 objetivos con recompensa (graduarte, casarte, primer millón,
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
| `sim.vel` | Estado de la vida, personalidad, rasgos sociales, paso de los meses, muerte |
| `events.vel` | Eventos anuales (datos) y su motor |
| `acts.vel` | Menús: ocupación, vínculos, actividades, noche, citas, casino, compras |
| `jobs.vel` | Catálogo de 150 carreras, especializaciones, segunda carrera y fusiones |
| `career.vel` | Acciones de las carreras +18, gustos en la cama y misiones |
| `edu.vel` | Familia, escuela, exámenes, universidades y técnicas, títulos y audiciones |
| `gigs.vel` | 100 chambas de medio tiempo, sus eventos y minimisiones |
| `minigames.vel` | Los 7 minijuegos y su dibujo |
| `dialogue.vel` | Árboles de diálogo con padres, pareja, hijos, amigos y ex |
| `health.vel` | Enfermedades, contagio, mundo de los demás personajes, médico y hábitos |
| `family.vel` | Hijos, peticiones, herencia, ruleta, nueva generación, vejez y eventos de legado |
| `effects.vel` | Efectos positivos y negativos y sus modificadores |
| `relations.vel` | Menús por vínculo, citas, intimidad, tríos, fertilidad, bodas, adopción, amigos de la infancia y eventos familiares |
| `chat.vel` | Conversaciones libres, temperamentos, ánimos, mensajes de tu pareja y familia extendida |
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
