# Planificación y Desglose de Tareas: Motor de Ajedrez (Rust)

**Base:** `requisitos_motor_de_ajedrez_en_rust_v6.md` (Fases 1 a 6, RF-01 a RF-64, RNF-01 a RNF-26, D-01 a D-42).
**Objetivo de este documento:** convertir los requisitos en un plan ejecutable: orden de trabajo, hitos con compuertas de calidad, tareas con trazabilidad a los requisitos y ruta crítica.

---

## 0. Convenciones del plan

- **ID de tarea:** `T<fase>.<n>` (por ejemplo, `T3.4`). `T0.x` es preparación del proyecto.
- **Estimación relativa:** **S** = hasta medio día, **M** = 1 a 2 días, **L** = 3 días o más (con tests incluidos). No hay fechas porque el documento aún no define fecha límite (Fase 1, pregunta 5). Cuando se conozca, se convierte la suma de tallas en calendario.
- **Prioridad:** la heredada del requisito (Must / Should / Could).
- **Dep.:** tareas que deben estar terminadas antes.
- **Hecho cuando:** criterio verificable, tomado de los criterios de aceptación del documento de requisitos.
- **Compuerta de calidad (aplica a TODA tarea de código):** `cargo test`, `cargo clippy` y `cargo fmt` sin errores ni advertencias, sin `unsafe` injustificado y sin `Vec`/`Box` en el camino caliente (RNF-03, 04, 05, 08, 12, 15, 20, 26).

## 1. Decisiones ya tomadas que condicionan el plan

| # | Decisión transversal | Efecto en el plan |
|---|---|---|
| 1 | Dependencias: solo un crate de FEN + dev-deps (por ejemplo, `criterion`); el resto con la biblioteca estándar. | Evaluar y fijar el crate de FEN al inicio (`T0.3`); benchmarks con `criterion`. |
| 2 | Repeticiones: en el árbol de búsqueda, 1 repetición = empate; triple repetición solo para el historial real. | `T4.7` expone dos consultas distintas; `T5.8` usa la de 1 repetición. |
| 3 | Capturas y quietud incluyen captura al paso y promoción a dama; la búsqueda principal genera las 4 promociones. | `T3.8` clasifica la promoción a dama como "táctica"; `T5.6` y `T5.7` lo respetan. |
| 4 | Generación **legal** en la búsqueda en v1; filtrado lazy como optimización futura medida. | `T3.9` es la ruta principal; `T7.2` (opcional) queda para optimizar. |
| 5 | Historial fijo; en UCI solo se guardan hashes desde la última jugada irreversible; si desborda: error por `stderr` y se conserva la posición anterior. | `T4.5` y `T6.4` implementan esa política. |
| 6 | Si llega `go` durante otra búsqueda, detener y emitir primero el `bestmove` anterior; `position` cambia solo el estado actual y no la copia ya entregada al buscador. | `T6.6` y `T6.8` implementan esta política. |
| 6 | El hash incluye la columna al paso solo si existe una captura legal; la detección del resultado de partida vive en `rules.rs`, no en `PositionState`. | `T4.3` compara la legalidad de la captura; `T4.11` expone `game_status`. |

**Pendientes sin decidir** (no bloquean el inicio, pero hay que cerrarlos antes de la fase indicada):

| Pregunta | Se necesita antes de | Valor por defecto propuesto |
|---|---|---|
| Fuerza de juego esperada (transversal 7) | Fase 5 | Nivel aficionado fuerte; sin tabla de transposición en v1. |
| Metas de rendimiento (perft nps, profundidad por tiempo) (transversal 6) | `T3.11`, `T5.10` | Medir primero y fijar la meta con la línea base. |
| Reglas de 75 movimientos / quíntuple repetición (Fase 4, preg. 6) | `T4.7` | No en v1. |
| Cómo manejar `go` con búsqueda activa (Fase 6, preg. 4) | `T6.6` | Detener la anterior (emitiendo su `bestmove`) y arrancar la nueva. |
| Opciones UCI, `d` y `go perft`, GUI de validación (Fase 6, preg. 3, 5, 6) | `T6.9`, `T6.11` | Sin opciones; incluir `d` y `go perft`; validar con Cute Chess. |

---

## 2. Estructura de repositorio propuesta

```
chess-engine/
├── Cargo.toml / Cargo.lock
├── build.rs                  # magics (D-16) y tabla Zobrist (D-22), semilla fija
├── src/
│   ├── lib.rs
│   ├── types.rs              # Color, PieceType, Square (+ TryFrom<u8>)
│   ├── board.rs              # Board (12 + 3 bitboards)
│   ├── state.rs              # PositionState, derechos de enroque
│   ├── fen.rs                # adaptador al crate de FEN + exportación
│   ├── moves.rs              # Move, MoveFlag, Display UCI
│   ├── makemove.rs           # make/unmake, UndoMove
│   ├── attacks/              # saltadoras, rayos (referencia), magics
│   ├── movegen.rs            # MoveList, pseudo-legal, legal, perft
│   ├── zobrist.rs            # claves const + hash
│   ├── history.rs            # historial [u64; 512], repeticiones
│   ├── rules.rs              # 50 movs, material insuficiente, mate/ahogado
│   ├── eval.rs               # material + PST
│   ├── search.rs             # negamax, ID, quietud, MVV-LVA
│   ├── time.rs               # gestor de tiempo
│   └── uci.rs                # bucle UCI, hilos
├── src/main.rs               # arranca el bucle UCI
├── tests/                    # perft, UCI, simetría, equivalencias
└── benches/                  # perft nps y búsqueda (criterion)
```

---

## 3. Hitos y compuertas

| Hito | Contenido | Compuerta de salida (criterio global) |
|---|---|---|
| **M0** | Preparación del proyecto | Repo compila, CI local (`test`+`clippy`+`fmt`) en verde, crate de FEN elegido. |
| **M1** | Fase 1: tablero y estado | FEN ida y vuelta; mapeo A1=0…H8=63 verificado; interfaz de ataques deslizantes con rayos. |
| **M2** | Fase 2: movimiento y make/unmake | `unmake(make(p)) == p` para todos los tipos de bandera; `Display` UCI correcto. |
| **M3** | Fase 3: generador y perft | **Perft del anexo B.1 coincide** en las 5 posiciones; 218 movimientos en la posición de máximo. |
| **M4** | Fase 4: hash, historial y reglas | Hash incremental == hash desde cero durante perft; triple repetición, 50 movs, mate/ahogado correctos. |
| **M5** | Fase 5: evaluación y búsqueda | Posiciones del anexo B.3 resueltas; alfa-beta == minimax sin poda; respuesta legal con tiempo mínimo. |
| **M6** | Fase 6: UCI y tiempo | Sesiones del anexo B.4 pasan; una partida completa en una GUI sin jugadas ilegales ni pérdidas por tiempo. |
| **M7** | Endurecimiento | Pruebas de estrés de concurrencia, benchmarks documentados, lista de dependencias, README. |

> **Regla de oro del plan:** no se empieza una fase sin haber cerrado la compuerta de la anterior. Un error de reglas descubierto en la Fase 5 cuesta mucho más que uno detectado por perft en la Fase 3.

---

## 4. Desglose de tareas

### M0: Preparación

| ID | Tarea | Req. | Dep. | Talla | Hecho cuando |
|---|---|---|---|---|---|
| T0.1 | Crear el proyecto Cargo (biblioteca + binario) con la estructura de carpetas anterior. | RNF-25 | n/d | S | `cargo build` compila. |
| T0.2 | Script local de calidad (`test` + `clippy -D warnings` + `fmt --check`) y, si se usa, CI. | RNF-05 | T0.1 | S | Un solo comando ejecuta las tres comprobaciones. |
| T0.3 | **Evaluar y elegir el crate de FEN**: licencia, mantenimiento, si permite exportar FEN, encaje con `Board`/`PositionState`. Documentar y fijar versión en `Cargo.lock`. Plan B: parser propio. | RF-06, RF-57, D-38, RNF-25 | T0.1 | M | Existe `DEPENDENCIAS.md` con nombre, versión, licencia y motivo. |
| T0.4 | Descargar y archivar los valores de referencia del anexo B (perft, posiciones de prueba) **verificándolos contra la fuente**, como archivo de datos de tests. | Anexo B | T0.1 | S | Archivo `tests/data/` con las FEN y conteos verificados. |
| T0.5 | Definir convenciones de documentación y de nombres (`from()`/`to()` sin `get_`). | Fase 2, preg. 4 | n/d | S | Nota en el README. |

### M1: Fase 1, Representación del tablero

| ID | Tarea | Req. | Dep. | Talla | Hecho cuando |
|---|---|---|---|---|---|
| T1.1 | Tipos `Color`, `PieceType`, `Square` con `TryFrom<u8>` / `Into<u8>`. | RF-04, D-04 | T0.1 | S | `Square::try_from(63)` = `Ok(H8)`; `try_from(64)` = `Err`. |
| T1.2 | Mapeo Little-Endian Rank-File y constantes de casillas, filas y columnas. | RF-03, D-02 | T1.1 | S | Test: A1=0, H1=7, A8=56, H8=63. |
| T1.3 | Utilidades de bitboard (consultar, colocar, retirar, contar, iterar bits). Sin asignaciones. | RNF-01 | T1.2 | S | Operaciones en tiempo constante, sin `Vec`/`Box`. |
| T1.4 | `Board`: 12 bitboards + 3 auxiliares; `Copy` + `PartialEq`; colocar/retirar pieza manteniendo auxiliares. | RF-01, RF-02, RNF-02 | T1.3 | M | Posición inicial correcta; test de consistencia de auxiliares tras secuencias arbitrarias. |
| T1.5 | `PositionState` básico (turno, enroque como `u8`, captura al paso `Option<Square>`, reloj 50, número de jugada); el hash se añade en M4. | RF-05, D-03, D-23 | T1.1 | S | `Board` no contiene esos campos; `PositionState` sí. |
| T1.6 | Adaptador del crate de FEN → `Board` + `PositionState`, y **exportador a FEN**. | RF-06, D-38 | T0.3, T1.4, T1.5 | M | `exportar(cargar(fen)) == fen` en el conjunto de prueba. |
| T1.7 | Rechazo de FEN inválidos con error descriptivo (campos faltantes, piezas desconocidas, filas incompletas, turno inválido). | RF-07 | T1.6 | S | Todas las cadenas inválidas devuelven `Err`, sin pánico. |
| T1.8 | Posición inicial estándar. | RF-08 | T1.6 | S | Equivale al FEN estándar de inicio. |
| T1.9 | Impresión ASCII del tablero (fila 8 arriba, columna A a la izquierda). | RF-09 | T1.4 | S | Orientación correcta en un test de texto. |
| T1.10 | Interfaz de ataques deslizantes `rook_attacks(sq, occ)` / `bishop_attacks(sq, occ)` / `queen_attacks`, **implementada con rayos** (referencia). | RF-10, D-05 | T1.3 | M | Resultados correctos con tablero vacío y con bloqueos. |
| T1.11 | Auditoría de la fase: un test por cada RF, `size_of::<Board>() == 120`. | RNF-02, RNF-04 | todas | S | Compuerta M1 en verde. |

### M2: Fase 2, Movimiento

| ID | Tarea | Req. | Dep. | Talla | Hecho cuando |
|---|---|---|---|---|---|
| T2.1 | `MoveFlag` como `enum` con los 14 valores explícitos del anexo A (4 bits). | RF-12, D-09 | T1.1 | S | Cada tipo tiene un valor distinto y cabe en 4 bits. |
| T2.2 | `Move` como `struct` (origen, destino, bandera) con accesos `from()`, `to()`, `is_capture()`, `is_promotion()`, `promotion_piece()` que devuelven enums. | RF-11, RF-13, D-08, D-10 | T2.1 | M | `promotion_piece()` devuelve `Option<PieceType>`; la API no expone `u8`/`u16`. |
| T2.3 | Verificar `Move: Copy + PartialEq` y `size_of::<Move>() <= 4`. | RNF-06 | T2.2 | S | Test de tamaño. |
| T2.4 | `Display` UCI de `Move`. | RF-17, D-13 | T2.2 | S | `e2e4`, `e7e8q`, `e1g1`, `e5d6`. |
| T2.5 | `UndoMove` / `StateInfo` (pieza capturada + estado previo; el hash llega en M4). | RF-14, D-11 | T1.5, T2.2 | S | `Move` no contiene esos campos. |
| T2.6 | **make move**: silencioso, captura, avance doble, captura al paso, enroques y promociones (con y sin captura); actualiza bitboards, turno, enroque, captura al paso y contadores. | RF-15, RF-30 a RF-32, D-12 | T1.4, T1.5, T2.5 | L | Tests a mano para cada tipo de bandera. |
| T2.7 | **unmake move** usando solo `Move` + `UndoMove`. | RF-16 | T2.6 | M | `unmake(make(p)) == p` para todos los casos de T2.6. |
| T2.8 | Revisión de que make/unmake no asignan memoria dinámica. | RNF-07 | T2.7 | S | Revisión de código sin `Vec`/`Box`. |

> Las reglas de pérdida de enroque (RF-30), casilla de captura al paso (RF-31) y reloj (RF-32) pertenecen formalmente a la Fase 4, pero **make move no puede probarse bien sin ellas**: se implementan aquí y la Fase 4 solo agrega hash e historial.

### M3: Fase 3, Generador de movimientos

| ID | Tarea | Req. | Dep. | Talla | Hecho cuando |
|---|---|---|---|---|---|
| T3.1 | `MoveList` estática de 256 con agregar, contar e iterar; comprobación de desborde en *debug*. | RF-18, RNF-09, D-14 | T2.2 | S | 218 movimientos caben sin desbordar. |
| T3.2 | Tablas `KNIGHT_ATTACKS[64]` y `KING_ATTACKS[64]` (const). | RF-20 | T1.3 | S | Caballo en A1 = 2 destinos; en D4 = 8. |
| T3.3 | Detección de casilla atacada y `is_in_check()` (usa rayos por ahora). | RF-25 | T1.10, T3.2 | M | Jaque por cada pieza, con bloqueo y sin amenaza. |
| T3.4 | Generador de peones: avances simples/dobles, capturas con máscaras de columna A/H, captura al paso. | RF-22 | T3.1 | M | Tests de bordes y de captura al paso solo con casilla en el estado. |
| T3.5 | Expansión de promociones en 4 `Move` (avance y captura). | RF-23, D-18 | T3.4 | S | 4 movimientos por promoción; 4 por cada captura posible. |
| T3.6 | Generador de piezas (caballo, alfil, torre, dama, rey) usando la interfaz RF-10. | RF-26 | T3.2, T1.10 | M | Posición inicial = 20 movimientos pseudo-legales. |
| T3.7 | Generador de enroques con las 5 condiciones. | RF-24 | T3.3 | M | Un test por condición que impide + uno donde sí se genera. |
| T3.8 | Segmentación en capturas y silenciosos (captura al paso y promoción a dama como tácticas, según decisión 3). | RF-19, D-17 | T3.4 a T3.7 | M | La unión es igual al conjunto completo, sin duplicados ni omisiones. |
| T3.9 | **Generación legal** por copia: copiar `Board` + `PositionState`, ejecutar, descartar si el rey queda atacado. | RF-27, D-15 | T2.6, T3.3, T3.8 | M | Clavadas, rey a casilla atacada, resolución de jaque y captura al paso ilegal (anexo B.2). |
| T3.10 | **Perft** con generación legal y make/unmake. | RF-28, D-19 | T3.9, T2.7 | M | **Coincide con anexo B.1 en las 5 posiciones**; 218 en la posición de máximo. |
| T3.11 | Benchmark de perft (nodos/s) con `criterion`; fijar línea base. | RNF-11 | T3.10 | S | Reporte reproducible. |
| T3.12 | **Magic Bitboards con `build.rs`** (semilla fija, tablas incrustadas) detrás de la misma interfaz RF-10. | RF-21, RNF-10, D-16 | T3.10 | L | Coincide con la implementación por rayos en 64 casillas × ocupaciones (vacía, llena, aleatorias con semilla fija); perft vuelve a pasar; compilación determinista. |
| T3.13 | Comparar perft con rayos vs. magics y documentar la mejora. | RNF-11 | T3.12 | S | Tabla de resultados en el README. |

> **Por qué magics va al final de la fase (T3.12):** con rayos se llega antes a perft correcto y la implementación de rayos sirve de oráculo para validar los magics (Riesgo "complejidad de números mágicos").

### M4: Fase 4, Estado histórico y repeticiones

| ID | Tarea | Req. | Dep. | Talla | Hecho cuando |
|---|---|---|---|---|---|
| T4.1 | Completar `PositionState` con `hash: u64`; verificar `Copy`, `PartialEq` y `size_of <= 16`. | RF-29, RNF-14, D-21 | T1.5 | S | Test de tamaño. |
| T4.2 | Tabla Zobrist const generada en `build.rs` (12×64, turno, 16 enroque, 8 columnas), determinista y sin claves repetidas ni nulas. | RF-33, RNF-10, D-22 | T3.12 (reutiliza `build.rs`) | M | Claves distintas y no nulas; dos compilaciones idénticas. |
| T4.3 | Hash desde cero (regla de captura al paso según decisión pendiente). | RF-34 | T4.2, T1.6 | S | Hash almacenado == hash desde cero para todos los FEN de prueba. |
| T4.4 | Actualización **incremental** del hash en make move; `UndoMove` guarda copia completa de `PositionState` (hash incluido). | RF-35, RF-36, D-24, RNF-13 | T4.3, T2.7 | M | Incremental == desde cero para cada bandera; aserción activa en perft. |
| T4.5 | Historial `[u64; 512]` con índice interno; make agrega, unmake retira; política de desborde (error, no pánico). | RF-37, D-25 | T4.4 | M | Tras make/unmake vuelve al estado anterior; sin memoria dinámica. |
| T4.6 | Re-ejecutar perft con aserción de hash en todos los nodos. | RF-35, D-19 | T4.4, T3.10 | S | Perft completo sin discrepancias. |
| T4.7 | Triple repetición (con corte en la última jugada irreversible) y consulta de **1 repetición** para la búsqueda. | RF-38 | T4.5 | M | Secuencia `g1f3 g8f6 f3g1 f6g8` ×2 = empate; con pieza distinta, no. |
| T4.8 | Regla de 50 movimientos (salvo mate en el movimiento 100). | RF-39 | T3.9 | S | FEN del anexo B.2 reporta empate. |
| T4.9 | Material insuficiente (K vs K, K+pieza menor vs K). | RF-40 | T1.4 | S | Anexo B.2 en ambos FEN. |
| T4.10 | Jaque mate y ahogado. | RF-41 | T3.9, T3.3 | S | Anexo B.2 clasificado correctamente. |
| T4.11 | Auditoría de la fase y decisión de dónde vive la detección de fin de partida (en `PositionState` o en una capa `Game`). | Fase 4, preg. 7 | todas | S | Compuerta M4 en verde. |

### M5: Fase 5, Raciocinio y evaluación

| ID | Tarea | Req. | Dep. | Talla | Hecho cuando |
|---|---|---|---|---|---|
| T5.1 | Constantes: valores de pieza (100/300/300/500/900/10000), PSTs `const [i32; 64]` por tipo, `MATE_SCORE`, `INFINITY`, `MAX_PLY` holgados dentro de `i32`. | RF-43, RNF-17, D-28 | T1.4 | S | Declaradas como `const`; negar `INFINITY` no desborda. |
| T5.2 | Evaluación estática desde el punto de vista del bando que mueve (material + PST con espejo vertical para negras). | RF-42, RF-44, D-27 | T5.1 | M | Posición inicial = 0; sin dama negra es positiva con blancas al turno. |
| T5.3 | Test de **simetría** con posiciones espejo (anexo B.3). | RF-44 | T5.2 | S | Evaluación idéntica en A y su espejo B. |
| T5.4 | Puntajes de mate ajustados por distancia. | RF-45, D-30 | T5.1 | S | Mate en 1 > mate en 3; `MATE_SCORE` > cota de evaluación. |
| T5.5 | **Minimax sin poda** (solo como oráculo de tests) y **negamax alfa-beta** a profundidad fija. | RF-46, D-29 | T5.2, T4.10 | M | Mismo puntaje con y sin poda en las posiciones de prueba. |
| T5.6 | Ordenamiento **MVV-LVA** sin memoria dinámica (captura al paso = peón x peón). | RF-51, D-32 | T3.8, T5.1 | M | Peón x dama antes que dama x peón; capturas antes que silenciosos. |
| T5.7 | **Búsqueda de quietud** con *stand pat*, solo capturas tácticas (decisión 3), MVV-LVA, poda y cota de profundidad. | RF-52, D-33, RNF-16 | T5.5, T5.6 | M | Peón envenenado (B.3): con quietud no elige `d2d5`. |
| T5.8 | Fin de partida dentro de la búsqueda: mate con distancia, ahogado = 0, empates = 0 (1 repetición cuenta). | RF-50, RF-39 a 41 | T4.7 a T4.10, T5.4 | M | Mate en uno (B.3) encontrado; ahogado y reloj 100 puntúan 0. |
| T5.9 | **Iterative deepening** con límites de profundidad y tiempo, comprobación cada N nodos y **señal de parada atómica**; resultado `{mejor movimiento, puntaje, profundidad, nodos}`. | RF-47, RF-48, RF-49, D-31 | T5.8 | L | Con tiempo mínimo devuelve movimiento legal de profundidad 1; `stop` termina devolviendo movimiento legal. |
| T5.10 | Benchmark de búsqueda (profundidad fija, nodos, tiempo, nps) y test de determinismo. | RNF-18, RNF-19 | T5.9 | S | Dos ejecuciones idénticas; reporte reproducible. |
| T5.11 | Auditoría de la fase (todo el anexo B.3). | RNF-16, RNF-20 | todas | S | Compuerta M5 en verde. |

### M6: Fase 6, Interfaz y ciclo de juego

| ID | Tarea | Req. | Dep. | Talla | Hecho cuando |
|---|---|---|---|---|---|
| T6.1 | Bucle de lectura de `stdin`, despacho de comandos, ignorar desconocidos y entradas mal formadas. | RF-53 | T0.1 | M | Sesión mixta sin pánico ni cierre. |
| T6.2 | Salida: solo UCI por `stdout` con *flush* inmediato; errores por `stderr`. | RNF-21 | T6.1 | S | Todas las líneas de `stdout` son UCI válido. |
| T6.3 | *Handshake*: `uci`, `isready`, `ucinewgame` (reinicia posición e historial). | RF-54 | T6.1 | S | Anexo B.4 (`uci`, `isready`). |
| T6.4 | Comando `position` (`startpos`/`fen` + `moves`), con conservación de la posición previa ante error y política de historial (decisión 5). | RF-55, RF-57 | T6.3, T1.6, T4.5 | M | `e2e4 e7e5` aplica ambos movimientos y conserva el hash actual desde la última jugada irreversible; `e2e5` informa error y no cambia la posición. |
| T6.5 | Traducción texto UCI → `Move` **buscando entre los legales**. | RF-56, D-39 | T3.9, T2.4 | S | Todos los legales de Kiwipete se recuperan; `e1g1` y `e7e8q` correctos. |
| T6.6 | Arquitectura de **dos hilos** (E/S + búsqueda sobre copia) con señal atómica; política para `go` con búsqueda activa. | RF-62, RNF-22, D-37 | T5.9 | L | `isready` responde durante búsqueda larga; `position` no altera búsqueda en curso. |
| T6.7 | Comando `go` (`wtime`, `btime`, `winc`, `binc`, `movestogo`, `depth`, `movetime`, `infinite`) sin bloquear la lectura. | RF-58 | T6.6 | M | `depth 4`, `movetime 500` e `infinite` se comportan como se espera. |
| T6.8 | `stop`, `quit`, y **un único emisor de `bestmove`** (exactamente uno por `go`; `bestmove 0000` sin legales; `infinite` espera a `stop`). | RF-59, RF-60 | T6.7 | M | Anexo B.4 (mate y ahogado, búsqueda infinita). |
| T6.9 | Salida `info` (profundidad, `score cp`/`score mate`, nodos, tiempo) tras cada iteración. | RF-61 | T5.9, T6.7 | S | Mate en uno emite `score mate 1`. |
| T6.10 | **Gestor de tiempo** (límite blando y duro, margen de seguridad). | RF-63, D-40, D-41 | T6.7 | L | Nunca supera el tiempo restante menos el margen; responde con poco tiempo. |
| T6.11 | Comandos de diagnóstico `d` y `go perft <n>` (Could). | RF-64 | T3.10, T1.9 | S | `go perft 4` = 197.281. |
| T6.12 | Validación con una GUI (por ejemplo, Cute Chess): una partida completa contra otro motor. | RNF-24 | T6.10 | M | Pendiente: Cute Chess y Stockfish no están instalados en el entorno local; se cubre el protocolo con pruebas automatizadas. |

### M7: Endurecimiento y cierre

| ID | Tarea | Req. | Dep. | Talla | Hecho cuando |
|---|---|---|---|---|---|
| T7.1 | Prueba de estrés: cientos de `go`/`stop` consecutivos sin bloqueos ni pánicos, un `bestmove` por `go`; medir latencia de `stop`. | RNF-22, RNF-23 | T6.8 | M | Pasa sin bloqueos. |
| T7.2 | *(Opcional, medido)* Optimización del filtrado de legalidad con máscaras de jaque y clavadas. | D-15, decisión 4 | T3.13, T5.10 | L | Solo si los benchmarks lo justifican; perft sigue coincidiendo. |
| T7.3 | Revisión de `unsafe`, dependencias y licencias. | RNF-03, RNF-25 | n/d | S | Sin `unsafe` injustificado; `DEPENDENCIAS.md` al día. |
| T7.4 | README: uso, arquitectura, resultados de benchmarks, decisiones y límites de la v1. | n/d | n/d | M | Documento revisado por el grupo. |

---

## 5. Orden de ejecución y ruta crítica

```
M0 ──► M1 ──► M2 ──► M3 ──► M4 ──► M5 ──► M6 ──► M7
                      │      │             ▲
                      │      └─ build.rs ──┘ (Zobrist reutiliza el de magics)
                      └─ perft (compuerta)
```

**Ruta crítica:** `T1.4 → T2.6 → T2.7 → T3.9 → T3.10 → T4.4 → T4.10 → T5.5 → T5.8 → T5.9 → T6.6 → T6.8 → T6.10 → T6.12`.

**Se puede paralelizar si hay varias personas** (sin romper las compuertas):

| Frente | Tareas independientes entre sí |
|---|---|
| Tablero y FEN | T1.6 a T1.9 (después de T1.4/T1.5) |
| Ataques | T1.10, T3.2, T3.12 (magics, solo necesita la interfaz RF-10) |
| Movimiento | T2.1 a T2.4 mientras otra persona hace T1.x |
| Evaluación | T5.1 a T5.4 pueden empezar en cuanto exista `Board` (M1) |
| Protocolo | T6.1 a T6.3 pueden empezar en cuanto exista el proyecto (M0) con un "motor falso" que responde `bestmove` fijo |
| Documentación | T0.3, T0.4, T7.4 |

Las tareas **no** paralelizables son las de la ruta crítica: dependen del código anterior y de la compuerta de perft.

---

## 6. Estrategia de pruebas (transversal)

| Nivel | Qué se prueba | Dónde |
|---|---|---|
| Unitarios | Un test por RF (RNF-04). | Dentro de cada módulo. |
| Ida y vuelta | `exportar(cargar(fen))`, `unmake(make(p))`, `texto → Move → texto`. | `tests/` |
| **Perft** | Conteos del anexo B.1 en todas las posiciones; con aserción de hash (T4.6). | `tests/perft.rs` |
| Equivalencia | Magics vs. rayos (T3.12); alfa-beta vs. minimax sin poda (T5.5). | `tests/` |
| Simetría | Evaluación en posiciones espejo (T5.3). | `tests/` |
| Posiciones tácticas | Mate en uno, pieza colgada, peón envenenado (anexo B.3). | `tests/search.rs` |
| Sesiones UCI | Entradas y salidas del anexo B.4 con el proceso real. | `tests/uci.rs` |
| Estrés | `go`/`stop` repetidos (T7.1). | `tests/stress.rs` |
| Rendimiento | perft nps y búsqueda con `criterion` (T3.11, T5.10). | `benches/` |

> **Aviso del propio documento (anexo B):** los valores de perft y posiciones de prueba son de la comunidad; hay que **verificarlos contra la fuente** antes de fijarlos como tests (`T0.4`).

---

## 7. Riesgos del plan y su tarea de mitigación

| Riesgo | Impacto | Mitigación en el plan |
|---|---|---|
| Error sutil de reglas (enroque, captura al paso, promoción) | Alto | Perft como compuerta de M3 y de M4 (T3.10, T4.6); tests por bandera (T2.6, T2.7). |
| Magics demasiado complejos o lentos de compilar | Medio | Se hace al final de M3 sobre un perft ya correcto; plan B: tablas generadas una vez y guardadas en el repo (T3.12). |
| Hash incremental distinto al hash desde cero | Alto | Aserción durante perft (T4.4, T4.6). |
| Interbloqueos o `bestmove` duplicado/ausente | Alto | Un único emisor de `bestmove` (T6.8) y prueba de estrés (T7.1). |
| Pérdida por tiempo en la GUI | Alto | Margen de seguridad y límites blando/duro (T6.10); validación con relojes cortos (T6.12). |
| El crate de FEN no encaja o no exporta | Medio | Evaluarlo en M0 (T0.3); plan B: parser propio. |
| Errores de signo/perspectiva en la búsqueda | Alto | Simetría (T5.3) y equivalencia con minimax (T5.5). |
| Desborde del historial por una partida larga en UCI | Medio | Política de la decisión 5 (T4.5, T6.4). |
| Alcance que crece (TT, killers, evaluación avanzada) | Medio | Todo eso está **fuera de v1** (ver sección 8). |

---

## 8. Fuera de alcance en v1 (no planificar ni empezar)

Libros de aperturas, tablebases, redes neuronales, búsqueda multihilo, interfaz gráfica propia, Chess960, *ponder*, MultiPV, `go searchmoves`, opciones UCI configurables, tabla de transposición, *killers*/*history*/movimiento nulo/reducciones, evaluación con fases de juego, variante principal, reglas automáticas de 75 movimientos y quíntuple repetición.

> Candidatas naturales para una **v2**, en este orden sugerido: tabla de transposición (el hash ya está), *killers* + *history*, evaluación interpolada medio juego/final, filtrado de legalidad con máscaras (T7.2).

---

## 9. Checklist de seguimiento

- [x] **M0** Preparación (T0.1 – T0.5)
- [x] **M1** Tablero y estado (T1.1 – T1.11)
- [x] **M2** Movimiento y make/unmake (T2.1 – T2.8)
- [x] **M3** Generador + **perft del anexo B.1** (T3.1 – T3.13)
- [x] **M4** Hash, historial y reglas (T4.1 – T4.11)
- [x] **M5** Evaluación y búsqueda (T5.1 – T5.11)
- [ ] **M6** UCI, hilos y tiempo (T6.1 – T6.12; falta validación con GUI)
- [x] **M7** Endurecimiento y cierre (T7.1 – T7.4; optimización opcional T7.2 no justificada)

**Totales:** 8 hitos, 75 tareas.
