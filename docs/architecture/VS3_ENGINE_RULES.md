# Reglas de integración entre VelvetEngine y VS3

Estado: normativa propuesta para código nuevo. Las excepciones existentes se
mantienen sólo durante su migración documentada.

## Propósito

VelvetEngine puede alojar y extender VS3. No puede redefinir su núcleo. Estas
reglas impiden que un lenguaje general termine acoplado a escenas, ECS, render,
audio o cualquier género de juego.

Las palabras **DEBE**, **NO DEBE**, **DEBERÍA** y **PUEDE** expresan prioridad
normativa.

## Modelo de dependencias

```text
aplicación / juego
    -> adaptadores VS3 de VelvetEngine
        -> ABI pública de VS3
            -> compilador, bytecode, VM y biblioteca estándar
```

La dependencia siempre apunta hacia el lenguaje. El núcleo VS3 no depende de
adaptadores del motor.

## 1. Frontera del núcleo

1. El núcleo DEBE contener sólo sintaxis, semántica, bytecode, VM, valores,
   módulos, errores y biblioteca estándar general.
2. El núcleo NO DEBE conocer escenas, entidades, componentes, sprites,
   cámaras, audio, físicas, assets, UI, cartas, novelas ni géneros.
3. Un concepto de VelvetEngine NO DEBE convertirse en palabra reservada,
   opcode o `NativeId` global.
4. Los tipos matemáticos generales pueden pertenecer a la biblioteca estándar.
   La conversión a tipos concretos del motor pertenece a un adaptador.
5. Todo crate adaptador DEBE poder eliminarse sin impedir compilar y probar el
   núcleo del lenguaje.

## 2. Módulos externos, no sintaxis especial

1. Toda capacidad del motor DEBE publicarse como módulo externo con namespace,
   por ejemplo `velvet.input` o `velvet.scene`.
2. El namespace `velvet.*` identifica bibliotecas del host; no otorga autoridad
   por sí mismo.
3. Los scripts DEBEN importar o recibir explícitamente las APIs que usan.
4. Los nombres globales específicos del motor están prohibidos.
5. Módulos propios y de terceros DEBEN usar el mismo mecanismo de registro que
   los módulos oficiales.

Ejemplo aceptable:

```velvet
import velvet.input

export function wants_jump() {
    return input.action_pressed("jump")
}
```

Ejemplo prohibido:

```velvet
scene Main
spawn Player
play_sound("jump")
```

El segundo ejemplo obliga al parser y al núcleo a conocer VelvetEngine.

## 3. Descriptor único de API

Cada módulo externo DEBE declarar, en datos inspeccionables:

- nombre canónico y versión;
- funciones, parámetros y resultados;
- errores posibles;
- pureza y determinismo;
- operación síncrona o asíncrona;
- capacidades requeridas;
- coste base y regla de coste dinámico;
- documentación y estado de obsolescencia.

El descriptor DEBE ser la única fuente para runtime, análisis semántico, LSP,
CLI y documentación. Duplicar firmas manualmente entre herramientas está
prohibido.

## 4. Capacidades y autoridad

1. Toda sesión DEBE empezar sin permisos externos.
2. El host DEBE conceder capacidades exactas o namespaces explícitos.
3. Importar un módulo NO DEBE conceder automáticamente permiso para ejecutarlo.
4. Archivos, red, reloj, entropía, portapapeles, procesos y variables de entorno
   NO DEBEN estar disponibles como autoridad ambiental.
5. Servicios sensibles DEBEN validar payload, tamaño, frecuencia y resultado.
6. El rechazo de permiso DEBE identificar capacidad requerida y forma de
   concederla, sin revelar datos sensibles.

## 5. Datos y handles

1. La frontera DEBE usar valores VS3 serializables o conversiones registradas y
   validadas.
2. Punteros, referencias Rust, índices ECS desnudos y memoria interna NO DEBEN
   cruzar la ABI.
3. Recursos del motor DEBEN representarse mediante handles opacos, tipados y
   con generación o validación equivalente.
4. Un handle expirado DEBE producir error controlado, nunca acceso inválido.
5. Conversiones `f64`/`f32` DEBEN ser explícitas y rechazar valores no finitos o
   fuera de rango cuando el destino lo requiera.
6. El propietario y la duración de cada valor externo DEBEN estar definidos en
   el descriptor o documentación del módulo.

## 6. Ejecución y scheduling

1. El host controla cuándo se llama al script; VS3 NO DEBE adueñarse del loop
   principal ni crear hilos del motor implícitamente.
2. Toda llamada DEBE respetar límites de instrucciones, memoria, pila y
   recursión.
3. Operaciones cuyo coste depende de datos DEBEN cobrar coste dinámico.
4. Trabajo lento DEBE usar tareas pendientes, no bloquear un frame.
5. Toda tarea pendiente DEBE poder cancelarse y terminar al destruir su host.
6. El número de respuestas inmediatas por ciclo DEBE estar limitado para evitar
   bucles host-script sin avance de frame.
7. Actualizaciones de ECS o escena durante consultas DEBERÍAN usar comandos
   diferidos para mantener reglas de préstamo y orden determinista.

## 7. Determinismo

1. Funciones declaradas puras DEBEN producir el mismo resultado para las mismas
   entradas y versión.
2. Tiempo, entrada, RNG no determinista y estado del mundo DEBEN llegar como
   argumentos, contexto o servicios explícitos.
3. Un módulo DEBE declarar si rompe determinismo y por qué.
4. Grabación y replay DEBEN registrar respuestas externas necesarias, no sólo
   llamadas de script.
5. Cambiar el algoritmo de una función determinista estable requiere versión o
   estrategia de compatibilidad.

## 8. Errores

1. Entradas de script NO DEBEN provocar `panic` en el host.
2. Errores DEBEN incluir módulo, operación, causa y acción recuperable cuando
   exista.
3. Errores de script DEBEN conservar archivo, línea y stack de VS3.
4. Errores externos DEBEN cruzar la ABI como datos controlados, no como texto
   arbitrario imposible de clasificar.
5. Información secreta, rutas privadas y payloads sensibles NO DEBEN aparecer
   en diagnósticos normales.

## 9. Compatibilidad

1. IDs publicados de bytecode o nativas nunca se renumeran ni reutilizan.
2. Toda ABI de módulo DEBE declarar versión y política de compatibilidad.
3. Una eliminación requiere aviso de obsolescencia, alternativa y ventana de
   migración publicada.
4. Shims heredados DEBEN estar aislados y no aceptar funciones nuevas.
5. El corpus de compatibilidad DEBE ejecutar paquetes de versiones anteriores.
6. Un cambio incompatible requiere nueva versión de lenguaje, bytecode, módulo
   o adaptador según la frontera afectada; nunca detección silenciosa.

## 10. Tooling y descubribilidad

1. Módulos registrados DEBEN aparecer en completions, hover y documentación.
2. LSP DEBE mostrar firma, versión, pureza, coste y capacidades relevantes.
3. CLI DEBE poder listar módulos y validar permisos sin ejecutar el programa.
4. Diagnósticos de imports y bindings DEBEN proponer el módulo o permiso que
   falta.
5. Herramientas NO DEBEN asumir que `velvet.*` siempre está instalado.

## 11. Pruebas obligatorias

Toda integración nueva DEBE incluir:

- prueba del módulo con host simulado;
- prueba de conformidad contra el adaptador real;
- permiso denegado por defecto y permiso concedido explícitamente;
- tipos, aridad, payload inválido y handle expirado;
- límites de instrucciones, memoria o solicitudes aplicables;
- cancelación de tareas pendientes;
- determinismo o declaración probada de no determinismo;
- metadata consumida por runtime y tooling;
- ausencia de pánicos ante entradas inválidas.

Las pruebas del lenguaje NO DEBEN necesitar una ventana, GPU, dispositivo de
audio ni mundo ECS real.

## 12. Organización de crates

Estructura objetivo orientativa:

```text
velvet-script-*              núcleo portable
velvet-script-host-sdk       descriptores y utilidades genéricas
velvet-vs3-host-core         composición del host VelvetEngine
velvet-vs3-host-input        módulo velvet.input
velvet-vs3-host-ecs          módulo velvet.ecs
velvet-vs3-host-scene        módulo velvet.scene
velvet-vs3-host-render       módulo velvet.render
velvet-vs3-host-audio        módulo velvet.audio
```

Los nombres finales pueden cambiar. La dirección de dependencias no.

## Excepciones actuales y migración

### Puente `velvet-math`

Hoy `velvet-script-vs3` depende de `velvet-math` para conversiones `f64`/`f32`.
Es una comodidad transitoria. DEBE moverse a un adaptador externo; los tipos
`vec*`, `mat*` y `quat` continúan como tipos generales de VS3.

### Presentación heredada

`present_show`, `present_hide`, `set_bg`, `ui_flag` y `ui_flag_get` son shims
heredados. NO DEBEN recibir nuevas funciones. Deben migrar a módulos externos
y conservarse sólo durante una ventana de compatibilidad documentada.

### Solicitudes mediante `yield`

`yield([service, payload])` sigue siendo una primitiva general válida. La ABI
tipada debe construir sobre ella o reemplazar su ergonomía sin codificar
servicios de VelvetEngine en el parser.

## Checklist para aprobar una integración

- [ ] ¿Vive fuera del núcleo VS3?
- [ ] ¿Es un módulo registrable, no sintaxis especial?
- [ ] ¿Tiene descriptor único y versionado?
- [ ] ¿Declara tipos, errores, pureza, coste y capacidades?
- [ ] ¿Permisos denegados por defecto?
- [ ] ¿No cruza punteros ni estado interno?
- [ ] ¿Respeta presupuestos y cancelación?
- [ ] ¿Tiene host simulado y prueba del adaptador real?
- [ ] ¿LSP, CLI y documentación consumen el mismo descriptor?
- [ ] ¿Incluye estrategia de compatibilidad y migración?

Si una respuesta es “no”, la integración no está lista para entrar.
