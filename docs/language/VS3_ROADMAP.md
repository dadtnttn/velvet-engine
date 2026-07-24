# Roadmap de VS3 como lenguaje general

Estado: propuesta activa, sin fechas fijas. El orden expresa dependencias, no
calendario.

Documentos normativos relacionados:

- [Contrato actual de VS3](VELVET_SCRIPT_3.md).
- [Matemáticas avanzadas](VS3_MATH_SPEC.md).
- [Reglas de integración con VelvetEngine](../architecture/VS3_ENGINE_RULES.md).
- [ADR 0008: VS3 como lenguaje general oficial](../adr/0008-velvet-script-3-official.md).

## Objetivo

Convertir VS3 en un lenguaje general, portable y embebible. VelvetEngine será
un host importante, no una parte implícita de su sintaxis o semántica.

La dirección es:

```text
programa VS3
    -> lenguaje + biblioteca estándar portable
    -> registro de módulos externos tipados
    -> ABI de host con capacidades
    -> VelvetEngine, otra aplicación o un host de pruebas
```

No se añadirán `scene`, `sprite`, `entity`, `audio` ni conceptos equivalentes
como palabras reservadas o nativas globales del núcleo.

## Punto de partida

Ya disponible:

- Compilador, bytecode, VM limitada, análisis semántico y diagnósticos.
- Tipos generales, colecciones y biblioteca matemática avanzada.
- Estado persistente, paquetes administrados por el host y tareas cooperativas.
- ABI genérica `Vs3Host`, solicitudes por servicio y política de capacidades.
- CLI, formatter, LSP, extensión VS Code, muestras y pruebas de integración.

Deuda conocida:

- Ya existen imports compartidos, módulos nominales con alias, llamadas `modulo.funcion()`, exportaciones explícitas, estado privado, resolución recursiva y detección de ciclos.
- Las firmas de servicios externos no forman todavía un registro tipado común.
- `velvet-script-vs3` contiene un puente directo hacia `velvet-math`.
- Las nativas antiguas `present_*`, `set_bg` y `ui_flag*` siguen disponibles
  como adaptador heredado.
- `yield([service, payload])` es portable, pero poco descubrible sin esquemas.

Estas excepciones no deben expandirse.

## Orden de implementación

| Fase | Resultado | Depende de |
|---|---|---|
| 0 | Frontera del lenguaje congelada y comprobable | Estado actual |
| 1 | Sistema de módulos de fuente | Fase 0 |
| 2 | ABI externa tipada y registrable | Fases 0-1 |
| 3 | Núcleo libre de VelvetEngine | Fase 2 |
| 4 | Módulos opcionales de VelvetEngine | Fases 2-3 |
| 5 | Ciclo de ejecución y tareas de juego | Fase 4 |
| 6 | Paquetes, portabilidad y herramientas completas | Fases 1-5 |
| 7 | Endurecimiento para producción | Todas |

## Fase 0 — Congelar fronteras

Entregables:

- Adoptar `VS3_ENGINE_RULES.md` como revisión obligatoria de integraciones.
- Inventariar cada dependencia y símbolo específico de VelvetEngine dentro de
  los crates `velvet-script-*`.
- Clasificar APIs como núcleo, biblioteca estándar, módulo externo o legado.
- Guardar pruebas de compatibilidad para bytecode, IDs nativos y ABI de host.
- Añadir una comprobación de dependencias prohibidas en CI.

Criterio de salida:

- Una integración nueva no puede entrar al núcleo sin que CI o revisión
  detecte la violación.
- Toda excepción actual tiene propietario, destino y estrategia de migración.

## Fase 1 — Módulos del lenguaje

Entregables:

- [x] Imports nominales con alias y llamadas `modulo.funcion()`.
- [x] Resolución de nombres entre archivos, estado privado y detección de ciclos.
- [x] Identidad interna determinista independiente de nombres globales del host.
- [x] Exportaciones explícitas de funciones con helpers privados y migración compatible.
- [x] Identidad pública `paquete.modulo` e imports estables entre paquetes.
- [x] Manifest con versión de lenguaje, módulos y dependencias locales con semver.
- [x] `velvet.lock` canónico con versiones exactas, grafo y SHA-256.
- [ ] Registro remoto, descarga, firmas y caché global verificable.
- [ ] Compilación incremental y caché por hash de fuente y dependencias.

Ejemplo objetivo:

```velvet
// @edition 3
import "game/rules.vel" as rules

export function score(input: map) {
    return rules.calculate(input)
}
```

La sintaxis anterior ya funciona, incluido `export function`. Los tipos nominales,
manifest de paquete y resolución sin rutas relativas pertenecen a la siguiente extensión.

Criterio de salida:

- El mismo paquete compila desde CLI, Rust API, editor y un host mínimo.
- Imports ausentes, ambiguos o cíclicos producen diagnósticos con ruta de
  resolución y acción sugerida.

## Fase 2 — ABI externa tipada

Entregables:

- Descriptor único para módulos, funciones, parámetros, resultados y errores.
- Metadatos de pureza, sincronía, coste, capacidades y versión.
- Registro dinámico de módulos antes de compilar o ejecutar.
- Conversión validada entre `Value` y tipos del host.
- Respuestas inmediatas, pendientes, canceladas y fallidas con contrato estable.
- Un host simulado oficial para probar módulos sin VelvetEngine.

El descriptor será la fuente única para compilador, VM, LSP, CLI y generación
de documentación. No se mantendrán listas manuales distintas por herramienta.

Criterio de salida:

- Una aplicación externa puede registrar un módulo propio sin modificar los
  crates del compilador o la VM.
- Aridad, tipos y permisos se diagnostican antes de ejecutar cuando la
  información está disponible.

## Fase 3 — Purificar el núcleo

Entregables:

- Mover conversiones `velvet-math` a un crate adaptador externo.
- Mover presentación heredada a un módulo opcional de compatibilidad.
- Separar intrínsecos de VM de módulos de biblioteca estándar registrables.
- Probar el lenguaje sin enlazar ECS, escenas, render, audio ni gameplay.
- Publicar ventanas y mensajes de obsolescencia para los adaptadores antiguos.

Criterio de salida:

- Los crates del núcleo VS3 compilan y pasan pruebas en un workspace mínimo sin
  depender de ningún subsistema de VelvetEngine.
- No quedan tipos de escena, ECS, render, audio o UI en su API pública.

## Fase 4 — Módulos opcionales de VelvetEngine

Entregables sugeridos:

| Módulo de host | Responsabilidad |
|---|---|
| `velvet.time` | Relojes, delta y temporizadores otorgados por el host |
| `velvet.input` | Acciones y dispositivos normalizados |
| `velvet.ecs` | Handles, consultas y comandos diferidos |
| `velvet.assets` | Identificadores y carga asíncrona de recursos |
| `velvet.scene` | Operaciones de escena como biblioteca opcional |
| `velvet.render` | Cámara, material y presentación |
| `velvet.audio` | Reproducción y control de buses |
| `velvet.ui` | Árbol y eventos de interfaz |
| `velvet.physics` | Consultas, contactos y comandos físicos |

Cada módulo vive en un adaptador separado, declara capacidades y puede
excluirse del producto final.

Criterio de salida:

- Un juego habilita únicamente los módulos que necesita.
- Un script que no importa un módulo no obtiene sus nombres ni permisos.
- Las pruebas de lenguaje usan hosts simulados; las pruebas de adaptador validan
  el comportamiento real del motor.

## Fase 5 — Ciclo de ejecución general

Entregables:

- Contextos de actualización proporcionados por el host, no globales ocultos.
- Eventos tipados y colas deterministas.
- Comandos diferidos para evitar mutación insegura durante consultas.
- Handles opacos con generación y errores claros para recursos expirados.
- Cancelación, plazos y presupuestos por tarea y por frame.
- Recarga en caliente con migración explícita de estado versionado.
- Reproducción determinista cuando todos los módulos utilizados lo permiten.

Criterio de salida:

- Un módulo de lógica puede ejecutarse, suspenderse, cancelarse y recargarse sin
  bloquear el hilo principal ni acceder a memoria interna del motor.
- Los excesos de CPU, memoria o solicitudes terminan con errores controlados.

## Fase 6 — Paquetes, portabilidad y experiencia de desarrollo

Entregables:

- [x] Resolución offline y lockfile de paquetes VS3 reproducibles.
- [x] Versionado semántico y restricciones de compatibilidad por paquete.
- [ ] Registro remoto, descarga autenticada y procedencia firmada.
- SDK documentado para hosts Rust y, después, ABI estable para otros lenguajes.
- Runner independiente para validar que la biblioteca estándar es portable.
- LSP con imports, firmas externas, permisos, documentación y navegación.
- CLI para listar módulos, firmas, capacidades y costes.
- Posible backend WebAssembly evaluado mediante pruebas, no asumido por diseño.

Criterio de salida:

- Un paquete de lógica no dependiente del motor produce el mismo resultado en
  el runner independiente y en VelvetEngine.
- Editor y CLI obtienen información desde los mismos descriptores que runtime.

## Fase 7 — Producción

Entregables:

- Depurador de tareas, inspector de estado y profiler por función/módulo.
- Fuzzing de parser, bytecode, deserialización y frontera de host.
- Matriz de compatibilidad de versiones de lenguaje, bytecode y módulos.
- Benchmarks de llamadas externas, tareas, colecciones y módulos matemáticos.
- Auditoría de capacidades, rutas, red, serialización y agotamiento de recursos.
- Guía de migración y política formal de obsolescencia.

Criterio de salida:

- Cero pánicos del host ante entradas de script no confiables en la suite de
  seguridad.
- Presupuestos y regresiones de rendimiento medidos en CI.
- Actualizaciones compatibles demostradas con corpus de versiones anteriores.

## Prioridad inmediata

Orden recomendado para la próxima implementación:

1. Descriptor y registro de módulos externos.
2. Imports y resolución de módulos en fuente.
3. Host simulado y pruebas de conformidad.
4. Extraer `velvet-math` y presentación fuera del núcleo.
5. Crear primero `velvet.time` y `velvet.input`; después ECS y recursos.

Esto desbloquea integraciones reales sin contaminar el lenguaje con conceptos
del motor.

## Fuera de alcance

- Añadir palabras reservadas por cada subsistema del motor.
- Exponer punteros, referencias Rust o componentes ECS internos.
- Permitir I/O, reloj, entropía, red o archivos como autoridad ambiental.
- Crear una segunda VM especial para VelvetEngine.
- Prometer compatibilidad mediante nombres sin versionar ni pruebas.
