# Roadmap de VS3 como lenguaje general

Estado: propuesta activa, sin fechas fijas. El orden expresa dependencias, no
calendario. Cada fase indica su estado real verificado contra el código
(`[hecho]`, `[parcial]`, `[pendiente]`).

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

## Punto de partida (verificado)

Ya disponible:

- Compilador, bytecode, VM con límites, análisis semántico y diagnósticos
  (`crates/velvet-script-vs3`: `lib.rs`, `semantic.rs`, `bundle.rs`).
- Módulos de fuente: imports nominales con alias, llamadas `modulo.funcion()`,
  `export function`, estado privado, resolución entre archivos y detección de
  ciclos (`bundle.rs`: `cyclic import`, `is private; declare export function`).
- Paquetes: manifest con versión de lenguaje y dependencias semver, resolución
  offline y `velvet.lock` canónico (`manifest.rs`, `package_resolver.rs`,
  `lockfile.rs`; CLI `velvet vs3 lock`).
- ABI externa tipada: `Vs3Host`, `ModuleDescriptor` / `FunctionDescriptor`
  con pureza (`Purity`), sincronía (`SyncKind`), coste (`CostClass`),
  capacidades y versión, más registro consultable antes de compilar
  (`abi.rs`; `registry.function(…)`).
- Protocolo portable `yield([service, payload])` con `resume` / `resume_with`
  y tareas cooperativas (`lib.rs`: `cooperative tasks`, `generic host ABI`).
- Núcleo libre de `velvet-math`: las conversiones viven en el adaptador
  opcional `velvet-script-vs3-engine` (`from_engine_vec2`, …).
- Primeros módulos de host registrados: `velvet.time` (`now_ms`,
  `delta_secs`) y `velvet.input` (`pressed`, `axis`) (`host_modules.rs`).
- Runtime declarativo de aplicaciones: paquete con `app_manifest`,
  `app_init`, `app_update`, `app_action` (`crates/velvet-vs3-app`).
- CLI (`velvet vs3 lock/check/run/fmt/natives`), formatter que preserva
  edición y comentarios, LSP con diagnósticos y completado para fuentes
  `// @edition 3`, y extensión VS Code.

Deuda conocida:

- Las nativas antiguas `present_show`, `set_bg`, `present_hide`, `ui_flag*`
  siguen disponibles como adaptador de compatibilidad dentro de la VM
  (`lib.rs`: `compatibility adapter`); su extracción a módulo registrable
  está pendiente.
- El LSP diagnostica y completa, pero aún no navega imports ni expone firmas
  externas con permisos y documentación.
- `yield([service, payload])` es portable, pero poco descubrible sin esquemas
  publicados desde los descriptores.

Estas excepciones no deben expandirse.

## Orden de implementación

| Fase | Resultado | Estado | Depende de |
|---|---|---|---|
| 0 | Frontera del lenguaje congelada y comprobable | `[hecho]` | Estado actual |
| 1 | Sistema de módulos de fuente | `[hecho]` salvo registro remoto e incremental | Fase 0 |
| 2 | ABI externa tipada y registrable | `[hecho]` | Fases 0-1 |
| 3 | Núcleo libre de VelvetEngine | `[parcial]` (`velvet.compat` hecho; falta separar intrínsecos) | Fase 2 |
| 4 | Módulos opcionales de VelvetEngine | `[parcial]` (`time`, `input` listos) | Fases 2-3 |
| 5 | Ciclo de ejecución y tareas de juego | `[parcial]` | Fase 4 |
| 6 | Paquetes, portabilidad y herramientas completas | `[parcial]` | Fases 1-5 |
| 7 | Endurecimiento para producción | `[pendiente]` | Todas |

## Fase 0 — Congelar fronteras `[hecho]`

Entregables:

- [x] Adoptar `VS3_ENGINE_RULES.md` como revisión obligatoria de integraciones.
- [x] Clasificar APIs como núcleo, biblioteca estándar, módulo externo o legado
  (núcleo en `velvet-script-vs3`, adaptador en `velvet-script-vs3-engine`).
- [x] Pruebas de compatibilidad para bytecode, IDs nativos y ABI de host.
- [x] Comprobación de dependencias prohibidas en CI
  (`tools/check_vs3_boundaries.py`, paso `VS3 boundaries`): el núcleo no
  depende de subsistemas del motor, el adaptador solo de `velvet-math`, y las
  nativas heredadas viven confinadas.
- [x] Propietario, destino y estrategia de migración escritos para cada
  excepción (tabla siguiente).

Excepciones vivas (únicas permitidas):

| Excepción | Propietario | Destino | Migración |
|---|---|---|---|
| Nativas `present_show` / `set_bg` / `present_hide` / `ui_flag` / `ui_flag_get` en la VM | Módulo compat `velvet.compat` | Servicios `yield` registrables + aviso de obsolescencia | Fase 3 |
| Conversiones `velvet-math` y hosts de sim pura (`velvet-terrain`, `velvet-liquid`) en `velvet-script-vs3-engine` | Adaptador engine | Se queda (adaptador designado) | Ninguna: es la frontera |
| `velvet-crypto` como dep del núcleo (lockfile SHA-256) | Pipeline de paquetes | Se queda (utilidad, no subsistema de juego) | Ninguna |

Criterio de salida:

- Una integración nueva no puede entrar al núcleo sin que CI o revisión
  detecte la violación.
- Toda excepción actual tiene propietario, destino y estrategia de migración.

## Fase 1 — Módulos del lenguaje `[hecho]`*

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

\* Hecho salvo los dos últimos puntos.

Ejemplo objetivo (ya funciona, incluido `export function`):

```velvet
// @edition 3
import "game/rules.vel" as rules

export function score(input: map) {
    return rules.calculate(input)
}
```

Criterio de salida:

- El mismo paquete compila desde CLI, Rust API, editor y un host mínimo.
- Imports ausentes, ambiguos o cíclicos producen diagnósticos con ruta de
  resolución y acción sugerida.

## Fase 2 — ABI externa tipada `[hecho]`

Entregables:

- [x] Descriptor único para módulos, funciones, parámetros, resultados y errores
  (`ModuleDescriptor`, `FunctionDescriptor`).
- [x] Metadatos de pureza, sincronía, coste, capacidades y versión.
- [x] Registro dinámico de módulos antes de compilar o ejecutar.
- [x] Conversión validada entre `Value` y tipos del host (adaptador engine).
- [x] Respuestas inmediatas, pendientes, canceladas y fallidas con contrato estable.
- [x] Un host simulado oficial para probar módulos sin VelvetEngine
  (`SimulatedHost`) más suite de conformidad (`conformance.rs`: invariantes
  de registro + drives de protocolo con políticas y presupuestos) y comando
  `velvet vs3 conform`.

El descriptor será la fuente única para compilador, VM, LSP, CLI y generación
de documentación. No se mantendrán listas manuales distintas por herramienta
(`velvet vs3 natives` ya lee del registro).

Criterio de salida:

- Una aplicación externa puede registrar un módulo propio sin modificar los
  crates del compilador o la VM.
- Aridad, tipos y permisos se diagnostican antes de ejecutar cuando la
  información está disponible.

## Fase 3 — Purificar el núcleo `[parcial]`

Entregables:

- [x] Mover conversiones `velvet-math` a un crate adaptador externo.
- [x] Mover presentación heredada (`present_show`, `set_bg`, `present_hide`,
  `ui_flag*`) a un módulo opcional de compatibilidad: `velvet.compat`
  (`velvet-script-vs3-engine/src/compat.rs`, `CompatHost` con el mismo
  estado y coerciones — test de equivalencia incluido).
- [ ] Separar intrínsecos de VM de módulos de biblioteca estándar registrables.
- [x] Probar el lenguaje sin enlazar ECS, escenas, render, audio ni gameplay.
- [x] Publicar ventanas y mensajes de obsolescencia para los adaptadores antiguos:
  warning de compilador (una vez por nombre y función, sin romper el build),
  visible en `velvet vs3 check` como `warn:`, y marca `[deprecated]` en
  `velvet vs3 natives`.

Criterio de salida:

- Los crates del núcleo VS3 compilan y pasan pruebas en un workspace mínimo sin
  depender de ningún subsistema de VelvetEngine.
- No quedan tipos de escena, ECS, render, audio o UI en su API pública.

## Fase 4 — Módulos opcionales de VelvetEngine `[parcial]`

Estado por módulo:

| Módulo de host | Responsabilidad | Estado |
|---|---|---|
| `velvet.time` | Relojes, delta y temporizadores otorgados por el host | [hecho] |
| `velvet.input` | Acciones y dispositivos normalizados | [hecho] |
| `velvet.ecs` | Handles opacos con generación, consultas y comandos diferidos (`flush`) | [hecho] |
| `velvet.assets` | Carga asíncrona (`Pending` + tickets, `status`/`bytes`) | [hecho] |
| `velvet.scene` | Operaciones de escena como biblioteca opcional | [pendiente] |
| `velvet.render` | Cámara, material y presentación | [pendiente] |
| `velvet.audio` | Reproducción y control de buses | [pendiente] |
| `velvet.ui` | Árbol y eventos de interfaz | [pendiente] |
| `velvet.physics` | Consultas, contactos y comandos físicos | [pendiente] |

Cada módulo vive en un adaptador separado, declara capacidades y puede
excluirse del producto final.

Criterio de salida:

- Un juego habilita únicamente los módulos que necesita.
- Un script que no importa un módulo no obtiene sus nombres ni permisos.
- Las pruebas de lenguaje usan hosts simulados; las pruebas de adaptador validan
  el comportamiento real del motor.

## Fase 5 — Ciclo de ejecución general `[parcial]`

Entregables:

- [x] Tareas cooperativas (`yield` / `resume` / `resume_with`) y VM con límites
  (`call_with_limits`).
- [x] Runtime declarativo de aplicación (`app_manifest`, `app_init`,
  `app_update`, `app_action`) con intérprete 2D determinista por software.
- [x] Comandos diferidos (`velvet.ecs` + `flush`) y handles opacos con
  generación (`e{id}:{gen}`, uso tras `despawn` = error controlado).
- [x] Presupuestos exigidos: `VmLimits` (instrucciones, memoria, recursión,
  pila), tope de solicitudes por drive y aborto controlado de bucles
  infinitos (probado en la suite de conformidad).
- [ ] Contextos de actualización proporcionados por el host, no globales ocultos.
- [ ] Eventos tipados y colas deterministas.
- [ ] Cancelación, plazos y presupuestos por tarea y por frame.
- [ ] Recarga en caliente con migración explícita de estado versionado.
- [ ] Reproducción determinista cuando todos los módulos utilizados lo permiten.

Criterio de salida:

- Un módulo de lógica puede ejecutarse, suspenderse, cancelarse y recargarse sin
  bloquear el hilo principal ni acceder a memoria interna del motor.
- Los excesos de CPU, memoria o solicitudes terminan con errores controlados.

## Fase 6 — Paquetes, portabilidad y experiencia de desarrollo `[parcial]`

Entregables:

- [x] Resolución offline y lockfile de paquetes VS3 reproducibles.
- [x] Versionado semántico y restricciones de compatibilidad por paquete.
- [x] CLI para listar módulos y firmas (`velvet vs3 natives`), resolver
  dependencias (`lock`), comprobar, ejecutar y formatear.
- [x] Runner de conformidad (`velvet vs3 conform`: invariantes de registro +
  protocolo drive + presupuestos, 11 casos).
- [x] LSP con diagnósticos y completado para `// @edition 3`.
- [ ] Registro remoto, descarga autenticada y procedencia firmada.
- [ ] SDK documentado para hosts Rust y, después, ABI estable para otros lenguajes.
- [ ] Runner independiente de conformidad para paquetes (mismo paquete, mismo
  resultado dentro y fuera de VelvetEngine).
- [ ] LSP con imports, firmas externas, permisos, documentación y navegación.
- [ ] Posible backend WebAssembly evaluado mediante pruebas, no asumido por diseño.

Criterio de salida:

- Un paquete de lógica no dependiente del motor produce el mismo resultado en
  el runner independiente y en VelvetEngine.
- Editor y CLI obtienen información desde los mismos descriptores que runtime.

## Fase 7 — Producción `[pendiente]`

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

Estado: los puntos 1–4 están hechos (fases 0–4 al día salvo lo marcado).
Queda, en orden:

1. Registro remoto con firmas + compilación incremental (cierra fases 1 y 6).
2. Resto de módulos de host (`scene`, `render`, `audio`, `ui`, `physics`).
3. Eventos tipados, contextos de actualización del host y recarga en caliente
   (cierra fase 5).
4. LSP con imports/firmas + SDK de hosts (cierra fase 6).
5. Fase 7 completa (debugger, fuzzing, profiler, benchmarks, auditoría).

Esto desbloquea integraciones reales sin contaminar el lenguaje con conceptos
del motor.

## Fuera de alcance

- Añadir palabras reservadas por cada subsistema del motor.
- Exponer punteros, referencias Rust o componentes ECS internos.
- Permitir I/O, reloj, entropía, red o archivos como autoridad ambiental.
- Crear una segunda VM especial para VelvetEngine.
- Prometer compatibilidad mediante nombres sin versionar ni pruebas.
