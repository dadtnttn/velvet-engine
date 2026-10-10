# Informe: sistema de cuevas/dungeons de Demon Queen (2026-09-25)

Inspección de solo lectura. Se leyó `demos/demon-queen/AGENTS.md` (único `AGENTS.md` del repo; trata solo del render a 1920×1080) y `git status`, y se siguieron las llamadas reales desde la seed hasta el render.

**Estado de git:** todo el sistema de dungeon es trabajo sin commit.
- `crates/velvet-dungeon/`, `src/dungeon/` y `src/bin/dungeon_lab.rs` no están versionados.
- `src/dungeon.rs` está añadido al índice (`AM`) sin commit.
- No hay historial git de ninguna de estas partes.

**Escrituras durante la inspección:** solo se recompiló `target/debug/dungeon-lab.exe` (artefacto de build, ver §4). El juego no estaba en ejecución y no se lanzó.

---

## 1. Arquitectura encontrada

### Generador lógico: `crates/velvet-dungeon`
Sin GPU ni lectura de archivos en tiempo de ejecución.

| Archivo | Responsabilidad |
|---|---|
| `src/lib.rs` | Tipos (`Config`, `Catalog`, `Room`, `Connection`, `Blueprint`) y toda la colisión, que es analítica: `surface`, `capsule_floor`, `line_of_sight`, `route`, `room_at`, `region_at`. |
| `src/generator.rs` | `generate(seed, config, catalog, difficulty)`: grafo → embebido en rejilla → salas → conexiones → spawns → validación. |
| `src/rng.rs` | Streams SplitMix64 por dominio y hash FNV-1a. |
| `src/gameplay.rs` | `Difficulty`, presupuestos y `place()` (enemigos, cofres, sellos, trampas). |
| `src/validation.rs` | Validador físico y de grafo; si falla, `generate` devuelve `Err`. |
| `src/controller.rs` | Controlador cinemático tipo cápsula y `traverse()` para pruebas. |
| `src/navigation.rs` | A* dentro de una sala (rejilla 0,65 m) para la IA enemiga. |
| `src/diagnostics.rs` | SVG del blueprint (solo lo usa `dungeon-check`). |
| `src/bin/dungeon-check.rs` | Herramienta por lotes; escribe SVG/JSON en `--out`. |
| `data/caverns.json` | Catálogo embebido: 4 plantillas, 3 biomas, 5 enemigos, 1 botín. |
| `data/compact.json`, `expedition.json`, `stress-large.json` | Configs alternativas; el juego no las usa (solo `dungeon-lab --config`). |

### Adaptador del juego: `demos/demon-queen/src/dungeon.rs` + `src/dungeon/`
Incluido con `#[path]` en `exploration3d.rs:30` y en `dungeon_lab.rs`.

| Archivo | Responsabilidad |
|---|---|
| `dungeon.rs` | `Dungeon`: estado de partida, IA y combate, espada, interacciones (`F`), teclas, regeneración por seed (`F2`). |
| `scene.rs` | Render: portales, caché de chunks, luces, cofres, identidad de salas, enemigos, armas, antorcha de mano, puerta sellada. |
| `visual.rs` | Mallas de sala y corredor/escalera, arcos de puerta, props procedurales. |
| `surfaces.rs` + `surface_styles.json` | Mallador de piedra Voronoi (paredes, techo, losas, túneles, pilares, muro secreto); paleta por bioma. |
| `dressing.rs` | Formaciones deterministas y `wall_torch()`. |
| `assets.rs` + `assets.json` | Carga de GLB/glTF, LOD por distancia, bisagra de la tapa del cofre. |
| `enemies.rs` | Esqueletos KayKit: roles → modelos, animaciones, puntos de mano, fracturas. |
| `finish.rs` | Posproceso: bloom a ¼ de resolución y viñeta. |
| `hud.rs` + `hud_skin.rs` | HUD 1920×1080, mapa (`M`), depuración (`F3`), entrada de seed (`F2`). |
| `runes.rs` | Puzzle de runas. |
| `save.rs` | `Snapshot` versionado con `restore()` validado. |
| `replay.rs` | Partida automática; `#[allow(dead_code)]` en el juego, solo la usan `dungeon-lab` y tests. |

### Integración con el mundo 3D (`exploration3d.rs`)
- `App::new`: genera o restaura el dungeon (`:9119–9142`).
- `enter_dungeon` / `leave_dungeon` / `bank_dungeon_loot`: `:9357–9444`.
- `tick`: `:10072`. Render delegado: `:7468`. Teclas: `:10533`, `:10656`.
- Persistencia: `exploration3d_state.rs:48` (`dungeon: Option<Snapshot>`, `dungeon_active`).

### Herramientas, prototipos y rutas no activas en el juego
- **`dungeon_lab.rs`**: mismo `Dungeon` y `Scene`. Modos: ventana, `--capture` (PNG/JSON), `--art-gallery`, `--replay` (headless; escribe solo con `--report`).
- **`tools/import_dungeon_art.py`, `tools/articulate_runestone.py`**: importación de arte Tripo con Blender.
- **Arte Tripo** (`cave_wall_rock`, `ruins_archway`, `ancient_runestone`, …): no está en `assets.json`; `data/assets/3d/dungeon_tripo/` solo tiene `import-report.json`; no hay GLB en `_archive`. La sección "Seven supplied assets" de `DUNGEON_TOON_ART.md` no corresponde al código. Solo queda el nombre del stream `tripo_dressing_v1`; las formaciones son procedurales (`visual::append_feature`).
- **Recursos de reserva**: `visual::enemy()` y cubos para cofre/antorcha/estandarte, solo si falta el asset.
- **`diagnostics.rs`**: solo `dungeon-check`.

---

## 2. Flujo de ejecución

### Seed
- `App::new` calcula la seed con `DefaultHasher(world_id, spawn_id, SystemTime nanos)` → `Dungeon::generate(seed)` (`exploration3d.rs:9111–9119`), siempre con `Config::default()`, `Difficulty::default()`, `Catalog::bundled()`. Cada sesión sin dungeon guardado tiene una seed distinta basada en el reloj.
- Dados `(seed, config, catálogo)` el blueprint es determinista. Se reproduce con `F2` (decimal o `0x…`, `dungeon.rs:1363`), con el snapshot guardado o con `dungeon-lab --seed`.
- `Dungeon::generate` usa `.expect(...)` (`dungeon.rs:534`): una seed que no valide provoca pánico al crear la `App`.
- Solo `App::new` y `F2` generan un dungeon nuevo. Tras ganar, `G` vuelve a la misma partida (`won = true`).

### Generación (`generator.rs`)
1. **Roles** por índice en la ruta crítica (12 por defecto): Entrance 0, Normal 1, Encounter 2, Puzzle 3, Event n/2, Elite n/2±1, Rest n−4, Miniboss n−3, Boss n−2, Exit n−1; el resto Rest/Event/Encounter/Normal por módulo.
2. **Columna vertebral**: camino auto-evitante con backtracking (presupuesto 25 000).
3. **Niveles**: ±1 nivel (3 m) con probabilidad `verticality`, limitado a −2..3.
4. **Expedición apilada** (`verticality > 0.35`): 4 salas a +1, +2, +3, +3; la última 9 m sobre el ancla.
5. **Ramas opcionales**: `round(n·branch_factor·(0.5+0.5·room_density))`; la 1ª es Secret (si `secret_density > 0`), la 2ª Treasure.
6. **Bucles** entre celdas adyacentes con |Δnivel| ≤ 1; si no hay, "square detour".
7. **Salas**: plantilla por peso (`room_geometry_v2`); Boss/Miniboss → Arena (`heart_chamber` tiene peso 0); Entrance/Puzzle/Rest/apiladas → Vault; **Event → Bridge** si `verticality > 0.1` (se cumple en las 4 configs). Octógono convexo con chaflanes. Centro = celda × `spacing` (28 m); y = nivel × 3 m. Bioma por tercio de progreso.
8. **Conexiones**: segmento recto borde a borde. Tipo: `Secret` si destino Secret; con desnivel `Stairs` (48 %) o `Ramp`; si no `Shortcut` (bucle), `Bridge` (18 %) o `Tunnel`. Ancho 3,4 m crítico / 2,8 m opcional; alto 3,4 m. Crea un `Socket` por sala.
9. **Obstáculos**: 4 pilares 0,88×0,88 m en Vault y Arena.
10. **Spawns** (`gameplay::place`) y **validación**.

Streams: `layout_v2` (secuencial, grafo y tipos de conexión), `room_geometry_v2`, `landmark_v2`, `gameplay_v2`, `loot_v2`, `decoration_v2`, `tripo_dressing_v1`, `procedural_arch_v1`, `room_identity_v1`, `near_detail`, `chest_herbs_v1`.

### Geometría renderizada
- Un chunk por sala y por conexión, generados solo en miss de caché (`scene.rs:781`).
- **Sala**: losas; en Bridge, foso a −4 m con cruz de ±1,8 m y paredes de una cara; techo, paredes Voronoi con hueco del tamaño exacto del socket, envejecimiento, arcos procedurales, marcas hacia sockets críticos, pilares, rocas, cristales y formaciones de `dressing::room`.
- **Conexión**: `Stairs` con peldaños de 0,15 m y contrahuellas en los límites de la colisión; el resto con suelo lineal; carcasa en tiras de ~3,2 m (paredes hasta 2,65 m, bóveda hasta 3,4 m) con sellado en extremos.
- Solo `Stairs` tiene geometría distinta; `Bridge`, `Tunnel`, `Ramp`, `Shortcut` se dibujan igual (el mapa colorea `Shortcut`/`Secret`, `hud_skin.rs:769–771`).
- `cull_mode: Back` (`mesh3d.rs:2275`): superficies de una sola cara.

### Gameplay por frame (`Dungeon::update`, `dungeon.rs:669`)
1. Controlador con sub-pasos de 8 ms.
2. Bloqueo de secretos cerrados y de la puerta del jefe mientras falte `seal && crest`.
3. Primera visita a una región → aviso de emboscada y retardos de ataque.
4. Caída al foso (y < suelo − 2,5 m) → punto de control, −12 HP.
5. IA solo en la sala actual del jugador.
6. Muerte: HP 100, oro ÷ 2, punto de control; los enemigos no se reinician.

### Render (`Scene::render`, `scene.rs:829`)
1. Visibilidad por portales BFS (profundidad < 3, 85 m, descarte trasero, línea de visión desde prof. 2).
2. Caché LRU de 48 chunks con `replace_mesh`.
3. Luces: antorcha por socket, cristal guía en Entrance/Normal, identidad (< 32 m), detalle, puertas, enemigos, antorcha y espada en 1ª persona. Se ordenan y recortan a **16** (`MAX_POINT_LIGHTS_3D`), sin sombras.
4. Niebla y ambiente del bioma actual.
5. Toon específico del dungeon, restaurado después; `Scene::new` activa `ShadingStyle3D::Toon` en el renderer compartido.
6. `finish` y HUD sobre `target_view` fijo 1920×1080 (`exploration3d.rs:56`).

---

## 3. Datos y contenido

### Plantillas (`caverns.json`, rev. 1)

| Plantilla | Forma | Medio ancho × medio fondo (m) | Alto (m) | Peso |
|---|---|---|---|---|
| broken_vault | vault | 5–6,8 × 5–7 | 4,8 | 3 |
| eroded_grotto | cavern | 5,5–7,5 × 5–7,5 | 5,8 | 5 |
| rift_crossing | bridge | 6–7,5 × 6–7,5 | 6,2 | 1 |
| heart_chamber | arena | 7,5–8 × 7,5–8 | 7,5 | 0 |

### Biomas

| Bioma | Pool de enemigos | En ruta crítica (sin `optional_only`) |
|---|---|---|
| `forgotten_masonry` | lurker, bone_scout, bone_guard | lurker |
| `luminous_grotto` | lurker, sentinel, bone_scout, grave_mage | lurker, sentinel |
| `ember_sanctum` | sentinel, bone_guard, grave_mage | sentinel |

- Los campos `audio`, `particles` y `decoration` del bioma no se usan en el código.
- Botín: solo `relic_cache` (12–30 oro); su campo `heal` no se usa.

### Spawns (`gameplay.rs`)
- **Encounter**: 2 `bone_scout` en flancos (también en ruta crítica); en un solo Encounter crítico (el más cercano a progreso 0,48) un `sentinel` Miniboss; más 4 enemigos (crítico) o 4–5 (opcional).
- **Elite / Miniboss / Boss**: 1 enemigo (×1,6 / ×2,5 / ×4,2). Elite añade trampa `ember_vent` (9 daño).
- **Interactivos**: cofre en Treasure/Secret; 3 runas en Puzzle; Rest, Memory (Event), Entrance, Exit.

### Combate (`dungeon.rs:123`)

| Estilo | Quién | Ataque |
|---|---|---|
| Quick | lurker | Hit_A, alcance 1,55 |
| Heavy | sentinel, bone_guard, todo Miniboss | Hit_B, alcance 2,0 |
| Mage | grave_mage y **todo Boss** (sea cual sea su definición) | Throw, área en el suelo |
| Ranged | bone_scout | Throw y proyectil |

Espada: 30 daño, impacto a 0,17 s de un golpe de 0,44 s, enfriamiento 0,52 s, cono frontal + línea de visión. Poción +45 HP, hierbas +20 HP.

### Modelos de enemigos (`enemies/kaykit_skeletons/manifest.json`)
- Modelos minion, warrior, rogue, mage; dos sets de animación compartidos.
- Roles: Boss → mage; Miniboss → warrior; **Elite → rogue** siempre; resto por definición. Un Elite `sentinel` lleva hacha y escudo (Heavy) sobre modelo rogue.
- Armas KayKit ancladas al `handslot`: Blade (Quick), Axe + Shield (Heavy), Crossbow (Ranged), Staff (Mage). Solo el mago tiene reserva procedural.
- Los enemigos muertos desaparecen al instante (`scene.rs:1066`); `Death_A/B` solo se usa en cadáveres decorativos.
- Daño visible: marcas de fractura y esquirlas.

### Assets activos (`assets.json`)
- `kaykit_dungeon` (cofre con tapa articulada, antorcha, monedas, estandarte, reliquias), `stylized_nature_test` (roca, champiñón), `items` (`healing_herbs`, `wooden_sword`), armas KayKit.
- `tools/build_portable.ps1` **no copia `data/assets/3d/items/`**: en un portable faltarían la espada (sin reserva) y las hierbas. Deducido del script, no verificado.

### Identidad por sala (`room_identity_plan`, `scene.rs:241`)

| Rol | Decoración |
|---|---|
| Treasure / Secret | cadáveres, corona, escudo, pilas de monedas, gemas, monedas sueltas |
| Rest | hierbas y gemas verdes |
| Encounter | espada, hacha y escudo en el suelo |
| Elite | estandarte y reliquia dorada |
| Miniboss / Boss | 2 estandartes, gemas rojas, reliquia rota |
| Normal / Entrance / Exit / Puzzle | nada |
| Cualquier sala Bridge | nada |

La rama `Role::Event` (gemas azules) es inalcanzable: Event siempre es Bridge con las configs incluidas.

Diferencias entre biomas: paleta de piedra/suelo/musgo, niebla y ambiente, mampostería en bioma 0 o Vault, cristales en bioma 1, acento de los landmarks.

### Progresión, secretos y persistencia
- Sello: runas en `[1,2,3]` (pista en Memory). Llave (`crest`): matar al Miniboss de la sala Miniboss. Puerta del jefe: `seal && crest`. Salida: `won`.
- Secretos: `F` a < 3 m de `c.start`; sin puzzle adicional.
- Rest: cura, fija punto de control, pociones a `max(2)`.
- `G` entra desde cualquier punto del mundo 3D (no hay entrada física) y sale en cualquier momento dentro del dungeon. Al salir, oro al mundo; pociones/hierbas al inventario de la villa (`pocion_pequena`, `hierbas_curativas`).
- Autoguardado cada 1 s también en el dungeon (`persist_3d_state(false)`, `:10834`) en `%APPDATA%…/session.json`.
- `restore` regenera desde la seed y rechaza el snapshot si cambian `GENERATOR_VERSION` o `rules_hash` (incluye todo el catálogo). Si falla, solo `eprintln` y se usa un dungeon nuevo con seed de reloj.
- `enter_dungeon` recoloca siempre al jugador en la entrada, salvo al reanudar sesión.

---

## 4. Validación y pruebas

### Validadores
- `Config::validate`, `Catalog::validate`: rangos, IDs duplicados, pools sin resolver.
- `validation::validate` (cada blueprint): índices finitos, conectividad, holgura en el centro, sockets compatibles, solapes de corredores y salas, pendiente ≤ 0,42, cápsula por cada conexión y del centro a cada puerta, spawns accesibles y sin bloquear puertas, existencia del jefe.
- `save::restore`: valida cada campo.

### Pruebas ejecutadas

| Comando | Resultado |
|---|---|
| `cargo test -p velvet-dungeon` | **13/13 ok** (18,6 s): 256 seeds por defecto, parámetros extremos, cápsula en seed 42. |
| `cargo test -p demon-queen --bin dungeon-lab` | **55/55 ok**: combate, fracturas, animaciones, cofre y bisagra, guardado, superficies, escaleras, arcos, identidad, dressing, replay de seeds 0, 1, 2, 42, 4815. |
| `cargo test -p velvet-render --lib` | **72/72 ok**. |
| Replay con el binario **preexistente** (24-sep 21:10, anterior al código actual) | 195/200; 5 atascos (seeds 31, 104, 106, 150, 168). No representa el código actual. |
| Recompilado + replay default, seeds 0–199 | **169 / 31 fallidas** |
| Replay `compact`, seeds 0–49 | 44 / 6 |
| Replay `expedition`, seeds 0–49 | 27 / 23 |
| Replay `stress-large` | 0–49: cortado a 290 s; 0–2: 0 / 3 |

- Todos los fallos del binario actual: `unresolved encounter … bone_scout`. El bot no mata a un ballestero en 1800 frames; el HP restante (34,x o 4,x) indica 0 o 1 golpes.
- Todas las seeds generaron sin error.
- Solo se sobrescribió `target/debug/dungeon-lab.exe` y dependencias en `target/`.

### Cobertura y límites
- Replay: solo ruta crítica (sin cofres, secretos ni salas opcionales); se cura bajo 60 HP; una muerte no lo detiene.
- Cofre: el test comprueba la matemática del yaw; que el frente del GLB sea +Z es coherente con la bisagra en z −0,56, sin prueba visual.
- `controller_cannot_walk_through_wall` solo comprueba suelo lógico al final, no que siga dentro de la sala.
- `dressing::wall_torch` no tiene tests.
- La parte Event del test de identidad nunca se ejercita.
- No ejecutado: tests de `demon-queen`/`demon-queen-3d` (escriben sesiones en `%TEMP%`), `dungeon-check` (escribe informes), `dungeon-lab --capture`/`--art-gallery` (escriben PNG), el juego.
- `DUNGEON_TOON_ART.md` indica "100 seeds / 0 failures" y "11 tests"; ya no coincide.

---

## 5. Estado observado

### Hechos confirmados
1. Replay actual falla en 15,5 % de seeds por defecto (31/200) y 46 % en `expedition` (23/50), siempre contra `bone_scout`.
2. Colisión 100 % analítica (polígonos de sala, rectángulos de conexión, AABB de pilares, techo de túnel 2,25 → 3,4 m). Ninguna malla ni asset colisiona; el jugador no choca con enemigos.
3. Escaleras: colisión y render coinciden por peldaño. Rampa/`Bridge`/`Tunnel` sin geometría distinta.
4. Ninguno de los 7 assets Tripo se carga.
5. Una seed de sesión inválida provocaría pánico; solo se probaron exhaustivamente 0–255 más los rangos de replay.
6. Cambios en `caverns.json` o `Config` invalidan expediciones guardadas y se descartan en silencio.

### Inferencias del código (no verificadas visualmente)
- **Antorchas** (`dressing.rs:25–55`): placa trasera a 0,50 m hacia dentro del plano de pared y a `ancho/2 + 0,55` m del centro de la puerta. El arco (`visual.rs:939`) ocupa −0,05…0,29 m de fondo (base hasta 0,345) y el poste llega a `ancho/2 + 0,48`. `ARCH_DEPTH = 0.9` no corresponde al arco actual. La antorcha quedaría ~0,15–0,2 m separada del arco y ~0,5 m de la pared.
- **Superficies de una cara con back-face culling**: invisibles desde detrás; compatible con la sensación de "hueco" si la cámara sale de la envolvente (ojo a 1,62 m, near 0,055, cápsula 0,28 m).
- **Cofres**: miran al primer portal del BFS desde la entrada, que con bucles puede no ser la puerta usada. Runas, altar, Rest y Memory miran a `sockets[0]`.
- **Poca identidad**: Normal, Entrance, Exit, Puzzle y todas las Bridge (incl. todas las Event) no reciben decoración de identidad.
- **Elite** puede tener arma que no casa con el modelo; **Boss** siempre modelo y estilo de mago.
- Enemigos confinados a su sala y solo activos con el jugador dentro.
- Con más de 16 luces cercanas se descartan las lejanas cada frame.
- Mallas de chunk generadas en el hilo de render en cada miss.

### No verificado
- Geometría hueca o atravesable en partida real.
- Alineación real de antorchas con paredes/arcos.
- Frente visual real del cofre.
- Lectura visual de armas y animaciones.
- Si un humano derrota al `bone_scout` donde falla el bot.
- FPS reales (las cifras del documento son render + espera síncrona de otra sesión).
- Letterbox y presentación según `AGENTS.md`.

---

## 6. Dependencias y límites

- `velvet-dungeon` → `velvet-math`, `serde`, `serde_json`.
- Adaptador → `velvet-dungeon`, `velvet-render` (wgpu, toon, `GltfModel`, `Material3D::with_emission` modificado), `winit`.
- `exploration3d.rs` → adaptador, `exploration3d_state`, economía del mundo (oro, inventario, tienda).
- `main.rs` → `exploration3d::App::prepare(gpu, dungeon_seed)` y título según `dungeon_active()`.
- El arte depende del blueprint, nunca al revés; el RNG de layout no depende del arte; `surface_styles.json` no altera `rules_hash`.

**Fuera del sistema de cuevas:** castillo, naturaleza, villa, tienda, mapa del mundo (`map_overlay.rs` no referencia el dungeon), novela visual. No existen en el dungeon: audio, partículas (campo sin uso), sombras de luces puntuales, destrucción de terreno.
