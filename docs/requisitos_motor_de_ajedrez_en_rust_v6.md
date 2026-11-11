# Documento de Requisitos y Alcance: Motor de Ajedrez (Rust)

**Estado:** Borrador completo, Fases 1 a 6
**Contenido:** Fase 1 (Representación del Tablero), Fase 2 (Representación del Movimiento), Fase 3 (Generador de Movimientos), Fase 4 (Estado Histórico y Repeticiones), Fase 5 (Raciocinio y Evaluación) y Fase 6 (Interfaz y Ciclo de Juego). Al final se incluye un resumen de identificadores por fase y las preguntas transversales prioritarias.

---

## 0. Convenciones

- **Identificadores:** `RF-xx` = requisito funcional, `RNF-xx` = requisito no funcional, `D-xx` = decisión de diseño. La numeración es continua a lo largo de todo el documento para que cada identificador sea único.
- **Prioridad (MoSCoW):** **Must** (imprescindible), **Should** (importante), **Could** (deseable).
- **Criterio de aceptación:** condición verificable que indica que el requisito está cumplido. Todo requisito debe tener uno.
- **Estructura de cada fase:** alcance, requisitos funcionales, requisitos no funcionales, decisiones de diseño, riesgos y preguntas abiertas.
- Cada fase separa **qué** debe cumplir el sistema (requisitos) de **cómo** se piensa resolver (decisiones de diseño).

---

## 1. Objetivo y contexto

Construir un motor de ajedrez en Rust, orientado a rendimiento y buenas prácticas, que:

- represente posiciones y movimientos de forma eficiente en memoria y CPU, sin perder legibilidad ni seguridad de tipos,
- juegue conforme a las reglas oficiales del ajedrez,
- pueda comunicarse con interfaces gráficas mediante el protocolo estándar **UCI (Universal Chess Interface)**, lo que además permite partidas Jugador vs PC desde cualquier GUI compatible.

---

## 2. Alcance general del proyecto (preliminar, a confirmar con el grupo)

| Incluye (v1) | No incluye (v1) |
|---|---|
| Representación del tablero con bitboards | Libros de aperturas |
| Representación de movimientos y deshacer movimientos | Tablas de finales (tablebases) |
| Generación de movimientos legales | Evaluación con redes neuronales |
| Reglas completas (enroque, captura al paso, promoción, empates, jaque mate y ahogado) | Búsqueda paralela con varios hilos de búsqueda (la v1 usa un hilo de E/S y un hilo de búsqueda) |
| Evaluación estática y búsqueda (alfa-beta) | Interfaz gráfica propia |
| Protocolo UCI, con gestión del tiempo (la consola solo se usa para depuración) | Variantes de ajedrez (por ejemplo, Chess960) |

---

# Fase 1: Representación del Tablero (Estructura de Datos Core)

## 1.1 Alcance de la Fase 1

**Incluye:**
- Tipos de datos base (`Color`, `PieceType`, `Square`).
- Estructura `Board` (disposición física de las piezas).
- Estructura de estado de la posición (`PositionState`), separada del tablero.
- Carga y exportación de posiciones en formato FEN (la carga puede apoyarse en un crate externo, ver Fase 6).
- Definición de la interfaz de ataques para piezas deslizantes (la implementación con Magic Bitboards llega en la Fase 3).

**No incluye (se cubre en fases posteriores):**
- Estructura `Move` y ejecución de movimientos (Fase 2).
- Generación de movimientos pseudo-legales y legales (Fase 3).
- Hash de Zobrist, historial y detección de repeticiones (Fase 4).
- Evaluación y búsqueda (Fase 5).
- Protocolo UCI (Fase 6).

## 1.2 Requisitos funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| RF-01 | El sistema debe representar las piezas con **12 bitboards** (un `u64` por cada combinación de tipo de pieza y color). | Must | Para la posición inicial, cada bitboard contiene exactamente las casillas esperadas (por ejemplo, peones blancos = casillas A2 a H2). |
| RF-02 | El sistema debe mantener **3 bitboards auxiliares**: todas las piezas blancas, todas las piezas negras y todas las casillas ocupadas. | Must | Tras cualquier secuencia de colocaciones y retiradas de piezas, cada auxiliar es igual a la unión (OR) de los bitboards principales correspondientes. |
| RF-03 | El sistema debe usar el mapeo **Little-Endian Rank-File**: A1 = bit 0, B1 = bit 1, ..., H1 = bit 7, A2 = bit 8, ..., H8 = bit 63. | Must | Test unitario que verifica el índice de casillas representativas (A1 = 0, H1 = 7, A8 = 56, H8 = 63). |
| RF-04 | El sistema debe definir los tipos `Color` (2 valores), `PieceType` (6 valores) y `Square` (64 valores) con conversión **segura** desde y hacia `u8`. | Must | `Square::try_from(63)` devuelve `Ok(H8)` y `Square::try_from(64)` devuelve `Err`. |
| RF-05 | El **estado de la posición** (turno, derechos de enroque, casilla de captura al paso, contador de 50 movimientos, número de jugada y, desde la Fase 4, el hash) debe residir en una estructura **independiente** de `Board` (`PositionState`). | Must | `Board` no contiene ninguno de esos campos; `PositionState` sí. |
| RF-06 | El sistema debe **cargar posiciones desde una cadena FEN** (la carga puede delegarse en un crate externo, D-38) y **exportarlas** a FEN. | Must | Para un conjunto de FEN válidos de prueba, `exportar(cargar(fen)) == fen`. |
| RF-07 | El sistema debe **rechazar FEN inválidos** devolviendo un error descriptivo, sin terminar el programa abruptamente. | Should | Cadenas con campos faltantes, piezas desconocidas, filas incompletas o turno inválido devuelven `Err`. |
| RF-08 | El sistema debe poder crear la **posición inicial estándar**. | Must | Cargar la posición inicial equivale a cargar el FEN estándar de inicio. |
| RF-09 | El sistema debe poder **imprimir el tablero en ASCII** para depuración. | Should | La salida muestra las 64 casillas con la orientación correcta (fila 8 arriba, columna A a la izquierda). |
| RF-10 | El cálculo de ataques de **torres, alfiles y reinas** debe estar detrás de una interfaz única que reciba casilla y ocupación y devuelva un bitboard de ataques. | Must | Existe `rook_attacks(sq, occupancy)` y `bishop_attacks(sq, occupancy)`; para casos de prueba (tablero vacío y con bloqueos) devuelven los ataques correctos. |

## 1.3 Requisitos no funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| RNF-01 | Las operaciones básicas sobre bitboards (consultar, colocar y retirar una pieza) deben ser de **tiempo constante** y sin asignaciones dinámicas. | Must | Revisión de código: ninguna de esas operaciones usa `Vec`, `Box` ni similares. |
| RNF-02 | `Board` debe ser **copiable de forma barata** (`Copy`) y comparable (`PartialEq`), porque el filtrado de legalidad de la Fase 3 se basa en copiar el tablero y porque facilita los tests. | Should | `Board` implementa `Copy` y `PartialEq`; su tamaño es de 120 bytes (15 bitboards de 8 bytes) más lo estrictamente necesario. |
| RNF-03 | El código de esta fase no debe usar **`unsafe`**, salvo justificación documentada. | Should | La búsqueda de `unsafe` en el código no devuelve resultados, o cada uso tiene un comentario de justificación. |
| RNF-04 | El código debe estar cubierto por **tests unitarios** para cada requisito funcional. | Must | `cargo test` pasa y existe al menos un test por cada RF. |
| RNF-05 | El código debe compilar sin advertencias bajo `cargo clippy` y estar formateado con `cargo fmt`. | Should | Ambos comandos terminan sin errores ni advertencias. |

## 1.4 Decisiones de diseño

| ID | Decisión | Justificación | Alternativa descartada |
|---|---|---|---|
| D-01 | **Bitboards `u64`** como representación del tablero. | Permiten generar y combinar ataques con operaciones a nivel de bits muy rápidas. | Arreglo de 64 casillas: más simple, pero más lento para generación de movimientos. |
| D-02 | Mapeo **Little-Endian Rank-File** (A1 = 0, H8 = 63). | Es el estándar más extendido, lo que facilita seguir referencias y reutilizar tablas conocidas. | Mapeo Big-Endian: menos habitual, aumenta el riesgo de confusión. |
| D-03 | Separar **`Board`** (piezas) del **estado de la posición**, que reside en la estructura **`PositionState`**. El estado se reparte en estructuras independientes (`PositionState`, estructura de deshacer e historial; ver Fase 4). | Mantiene `Board` pequeño y fácil de copiar; separa responsabilidades. | Una única estructura con todo. |
| D-04 | Conversión de `u8` a `Square` mediante **`TryFrom<u8>`**. | Un cast directo con un valor fuera de rango es comportamiento indefinido en Rust. | `transmute` o cast directo. |
| D-05 | **Magic Bitboards** como objetivo para piezas deslizantes, detrás de la interfaz definida en RF-10. La generación de las tablas se define en la Fase 3 (D-16). | Máxima velocidad en generación de movimientos; la interfaz permite empezar con un método más simple (rayos) y reemplazarlo después. | Rayos clásicos como solución definitiva: más lentos. |
| D-06 | El **hash de Zobrist** vive en `PositionState` (confirmado en la Fase 4, D-21). | Depende del turno, enroque y captura al paso, que están en esa estructura. | Guardarlo en `Board`. |
| D-07 | **UCI se trata como interfaz independiente**, no como parte de la representación del tablero. | Es un requisito de comunicación, no de estructura de datos. | Incluirlo en la capa del tablero. |

## 1.5 Riesgos

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Errores sutiles en el mapeo de casillas (inversión de filas o columnas) | Alto: afecta a todo lo posterior | Tests unitarios del mapeo (RF-03) y de FEN (RF-06) desde el inicio. |
| Complejidad de generar los números mágicos | Medio | Empezar con ataques por rayos detrás de la interfaz RF-10; esa versión sirve después como referencia para probar los Magic Bitboards (RF-21). |
| Inconsistencia entre bitboards principales y auxiliares | Alto | Criterio de aceptación de RF-02 y test de consistencia. |

## 1.6 Preguntas abiertas para el grupo

1. ¿Se priorizan el rendimiento o la claridad del código en esta primera versión?
2. ¿Qué dependencias externas se permiten? La Fase 6 incorpora un crate para FEN; ¿se aceptan otras (por ejemplo, para números aleatorios o tests) o el resto se hace con la biblioteca estándar?
3. ¿Qué fuerza de juego se espera del motor (nivel aproximado)?
4. ¿Se acepta usar ataques por rayos como implementación provisoria hasta tener los Magic Bitboards (Fase 3)?
5. ¿Hay una fecha límite o un hito de entrega que condicione el alcance?

---

# Fase 2: Representación del Movimiento (Input/Output del Generador)

El movimiento debe ser lo suficientemente ligero para generarse y evaluarse millones de veces por segundo, manteniendo la legibilidad y la seguridad de tipos (*type safety*) de Rust.

## 2.1 Alcance de la Fase 2

**Incluye:**
- Estructura `Move` con campos explícitos (origen, destino y banderas).
- Métodos de acceso que devuelven los tipos definidos en la Fase 1 (`Square`, `PieceType`).
- Estructura secundaria de deshacer (`UndoMove` o `StateInfo`).
- Ejecución (*make move*) y reversión (*unmake move*) de un movimiento sobre el tablero y el estado.
- Conversión de un `Move` a texto en formato UCI (`Display`).

**No incluye (se cubre en fases posteriores):**
- Generación de movimientos y verificación de su legalidad (Fase 3). *Make move* asume que recibe un movimiento ya generado por el motor y no valida su legalidad.
- Actualización del hash de Zobrist al ejecutar o deshacer (Fase 4, RF-35 y RF-36).
- *Parsing* de un movimiento escrito en texto UCI hacia un `Move` (Fase 6, RF-56), porque necesita conocer la posición para determinar las banderas.

## 2.2 Requisitos funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| RF-11 | El sistema debe definir `Move` como una **`struct` con campos explícitos**: casilla de origen, casilla de destino y banderas. | Must | Se puede construir un `Move` a partir de dos `Square` y una bandera, y consultar cada dato. |
| RF-12 | Las **banderas** deben codificar, siguiendo el estándar de la industria (4 bits), el tipo de movimiento: silencioso, avance doble de peón, enroque corto, enroque largo, captura, captura al paso y promoción (a caballo, alfil, torre o dama), con y sin captura. | Must | Existe un valor de bandera distinto para cada uno de los 14 tipos de movimiento del anexo A y cada valor cabe en 4 bits. |
| RF-13 | El sistema debe ofrecer **métodos de acceso** para leer el movimiento (por ejemplo, `from()`, `to()`, `is_capture()`, `is_promotion()`, `promotion_piece()`) que devuelvan **enums** y no tipos numéricos crudos. | Must | `from()` y `to()` devuelven `Square`; `promotion_piece()` devuelve `Option<PieceType>`; la API pública de `Move` no expone `u8` ni `u16` para casillas o piezas. |
| RF-14 | La información de **pieza capturada y estado previo irreversible** (derechos de enroque, casilla de captura al paso y contador de 50 movimientos) **no** debe almacenarse en `Move`, sino en una estructura secundaria (`UndoMove` o `StateInfo`). | Must | `Move` no contiene esos campos; la estructura secundaria sí. |
| RF-15 | El sistema debe **ejecutar** un movimiento sobre el tablero y el estado (*make move*), actualizando bitboards, turno, derechos de enroque, casilla de captura al paso y contadores, y **producir** la estructura de deshacer correspondiente. | Must | Tests con movimientos construidos a mano: silencioso, captura, avance doble, captura al paso, ambos enroques y cada promoción (con y sin captura) producen la posición esperada. |
| RF-16 | El sistema debe **deshacer** un movimiento (*unmake move*) usando únicamente el `Move` y la estructura de deshacer, restaurando exactamente la posición anterior. | Must | Para cada caso de prueba de RF-15, `unmake(make(posición))` es igual a la posición original (tablero y estado comparados con `PartialEq`). |
| RF-17 | `Move` debe implementar el trait **`Display`** y producir el formato de texto del protocolo UCI. | Must | Avance doble: `e2e4`. Promoción a dama: `e7e8q`. Enroque corto blanco: `e1g1` (movimiento del rey de dos casillas). Captura al paso: origen y destino (`e5d6`). |

## 2.3 Requisitos no funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| RNF-06 | `Move` debe ser **liviano**: `Copy`, comparable y de tamaño pequeño, para usarlo en listas de movimientos y en el ordenamiento de movimientos (*move ordering*). | Must | `Move` implementa `Copy` y `PartialEq`, y `size_of::<Move>() <= 4` bytes. |
| RNF-07 | *Make move* y *unmake move* no deben realizar **asignaciones dinámicas**. | Must | Revisión de código: ninguna de las dos operaciones usa `Vec`, `Box` ni similares. La estructura de deshacer se devuelve por valor. |
| RNF-08 | El código de esta fase debe cumplir los mismos criterios de calidad que la Fase 1 (RNF-03, RNF-04 y RNF-05). | Must | `cargo test`, `cargo clippy` y `cargo fmt` pasan sin errores ni advertencias, sin `unsafe` injustificado. |

## 2.4 Decisiones de diseño

| ID | Decisión | Justificación | Alternativa descartada |
|---|---|---|---|
| D-08 | **`Move` como `struct` explícita**, sin empaquetar los datos en un entero. | Mantiene la legibilidad y la seguridad de tipos; evita manipulación de bits cruda. | `u16` empaquetado: más compacto, pero más difícil de leer y de depurar. Podría reconsiderarse más adelante si las mediciones lo justifican, sin cambiar la API (RF-13). |
| D-09 | **Banderas de 4 bits** según el estándar de la industria, expresadas como un `enum` con valores explícitos (ver anexo A). | Un único campo codifica todos los tipos de movimiento; es compatible con la bibliografía y con un eventual empaquetado futuro. | Un booleano por cada propiedad: ocupa más y permite combinaciones inválidas. |
| D-10 | Los **métodos de acceso devuelven enums** (`Square`, `PieceType`), nunca tipos numéricos. | Garantiza coherencia con la Fase 1 y seguridad en tiempo de compilación. | Devolver `u8`: más simple, pero habilita errores de índice. |
| D-11 | **Pieza capturada y estado irreversible fuera de `Move`**, en una estructura secundaria creada al ejecutar el movimiento. | Mantiene `Move` lo más liviano posible. Para el ordenamiento de movimientos, la pieza capturada se consulta en `Board` a partir de la casilla de destino. | Guardar todo en `Move`: lo agranda y lo hace más lento de copiar y ordenar. |
| D-12 | **Make/Unmake** como mecanismo principal para recorrer el árbol de búsqueda. | Evita copiar el tablero completo en cada nodo; la estructura de deshacer guarda solo lo irreversible. La copia del tablero se sigue usando para el filtrado de legalidad (D-15). | Copiar el tablero en cada nodo de la búsqueda (*copy-make*): más simple, pero más costoso. |
| D-13 | **`Display` para la salida UCI.** | Permite imprimir el movimiento directamente en el formato del protocolo, sin funciones auxiliares. | Función de conversión separada. |

## 2.5 Riesgos

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Errores al deshacer casos especiales (captura al paso, enroque, promoción con captura) | Alto: corrompe la búsqueda de forma silenciosa | Tests específicos para cada tipo de bandera (RF-15 y RF-16) y perft en la Fase 3 (RF-28). |
| Formato UCI incorrecto en enroque o promoción | Medio: la GUI rechaza o malinterpreta el movimiento | Criterios de aceptación de RF-17 con casos de enroque, promoción y captura al paso. |
| Necesitar la pieza capturada para ordenar movimientos sin tenerla en `Move` | Bajo | Consultarla en `Board` por la casilla de destino (D-11); si resulta lento, reevaluar. |
| La estructura de deshacer no contempla el hash de Zobrist | Medio: rediseño en la Fase 4 | Resuelto en la Fase 4: la estructura de deshacer guarda una copia completa del estado previo, incluido el hash (RF-36). |

## 2.6 Preguntas abiertas para el grupo

1. ¿Las banderas se modelan como un `enum` o como un tipo sobre un `u8` con constantes? (El `enum` es más seguro, pero requiere conversiones.)
2. ¿Se fija un tamaño máximo para `Move` (RNF-06) o se deja como objetivo a medir?
3. ¿La estructura de deshacer se guarda en una pila propia de la búsqueda o se devuelve y administra por quien llama?
4. ¿Se mantiene el prefijo `get_` en los métodos de acceso o se sigue la convención de Rust (`from()`, `to()`)?
5. ¿Se contempla Chess960 en el futuro? Cambiaría el formato UCI del enroque.

---

# Fase 3: Generador de Movimientos (Move Generation)

Es el módulo más crítico para el rendimiento. Determina qué jugadas están disponibles en cada posición y sigue un enfoque de **generación pseudo-legal seguida de un filtrado estricto de legalidad**.

## 3.1 Alcance de la Fase 3

**Incluye:**
- Estructura `MoveList` estática para almacenar movimientos sin usar el *heap*.
- Tablas de ataques de piezas saltadoras (caballo y rey).
- Implementación de Magic Bitboards para piezas deslizantes, con tablas generadas en compilación.
- Generación de movimientos de peones (avances, capturas, captura al paso y promociones).
- Generación de enroques.
- Detección de casillas atacadas y de jaque.
- Generación pseudo-legal y filtrado de legalidad.
- Segmentación de los movimientos en capturas y silenciosos.
- *Perft* como método de validación del generador.

**No incluye (se cubre en fases posteriores):**
- Hash de Zobrist, historial, detección de repeticiones y reglas de fin de partida: empates, jaque mate y ahogado (Fase 4).
- Ordenamiento de movimientos y búsqueda de quietud (*quiescence search*) propiamente dicha (Fase 5). Aquí solo se deja preparada la segmentación.
- Protocolo UCI (Fase 6).
- Optimizaciones del filtrado de legalidad (máscaras de jaque y piezas clavadas), que quedan como mejora futura.

## 3.2 Requisitos funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| RF-18 | El sistema debe almacenar los movimientos generados en una **`MoveList` estática**, respaldada por un arreglo preasignado en el *stack* de **capacidad 256**, con operaciones para agregar, consultar cantidad e iterar. | Must | `MoveList` no usa memoria dinámica; agregar, contar e iterar funcionan correctamente; una posición con 218 movimientos legales (anexo B) se genera sin desbordar la lista. |
| RF-19 | El generador debe poder devolver los movimientos **segmentados en dos grupos**: capturas y movimientos silenciosos. | Must | Para un conjunto de posiciones de prueba, la unión de ambos grupos es igual al conjunto completo de movimientos generados, sin duplicados ni omisiones, y ningún movimiento del grupo de capturas es silencioso. |
| RF-20 | Los ataques de **caballos y reyes** deben resolverse con **tablas precalculadas de tamaño fijo** (por ejemplo, `KNIGHT_ATTACKS[64]` y `KING_ATTACKS[64]`) indexadas por la casilla de la pieza. | Must | Para cada casilla de las 64, la tabla coincide con el cálculo de referencia; un caballo en A1 tiene exactamente 2 destinos y uno en D4 tiene 8. |
| RF-21 | Los ataques de **torres y alfiles** (y, por composición, de la reina) deben calcularse con **Magic Bitboards** que implementen la interfaz definida en RF-10, con tablas y números mágicos generados antes de la ejecución mediante un script `build.rs`. | Must | Para las 64 casillas y un conjunto grande de ocupaciones (vacía, llena y aleatorias con semilla fija), el resultado coincide con el de una implementación de referencia basada en rayos. |
| RF-22 | El generador de **peones** debe producir avances simples y dobles mediante desplazamientos de bits, capturas diagonales (sin cruzar el borde del tablero entre las columnas A y H) y captura al paso. | Must | Un peón en la columna A no captura hacia la columna H y viceversa; el avance doble solo se genera desde la fila inicial y con ambas casillas libres; la captura al paso solo se genera cuando hay casilla de captura al paso en el estado. |
| RF-23 | Cuando un peón alcance la última fila (detectado mediante máscara de bits), el generador debe **expandir automáticamente** el movimiento en 4 `Move` distintos (dama, torre, alfil y caballo), tanto en avances como en capturas. | Must | Un peón en la séptima fila con casilla libre produce 4 movimientos con banderas de promoción sin captura; por cada pieza rival capturable produce 4 movimientos con banderas de promoción con captura. |
| RF-24 | El generador debe producir los **enroques** únicamente cuando se cumplan todas las condiciones: derecho vigente, casillas entre rey y torre vacías, rey que no está en jaque y rey que no pasa por una casilla atacada ni termina en una casilla atacada. | Must | Existe un test por cada condición que impide el enroque (derecho perdido, pieza en medio, rey en jaque, casilla de paso atacada, casilla de destino atacada) y un test donde el enroque sí se genera. |
| RF-25 | El sistema debe ofrecer una función para determinar si una **casilla está atacada por un bando** y una función **`is_in_check()`** para saber si el rey de un bando está en jaque. | Must | Tests con jaque por cada tipo de pieza (peón, caballo, alfil, torre, dama), con una pieza interpuesta que bloquea el ataque y con el rey sin amenazas. |
| RF-26 | El sistema debe ofrecer **generación pseudo-legal**: todos los movimientos según el movimiento físico de cada pieza, ignorando si dejan al propio rey en jaque. | Must | La posición inicial produce 20 movimientos; en posiciones de prueba específicas, un movimiento que deja al rey en jaque aparece en el resultado pseudo-legal. |
| RF-27 | El sistema debe ofrecer **generación legal**: copiar el `Board` y el `PositionState` (sin el historial), ejecutar el movimiento pseudo-legal en la copia y **descartarlo si el rey del bando que movió queda bajo ataque**. | Must | Una pieza clavada no puede abandonar la línea de clavada; el rey no puede moverse a una casilla atacada; en jaque solo se devuelven movimientos que lo resuelven; la captura al paso que dejaría al rey expuesto en la misma fila se descarta (posición del anexo B). |
| RF-28 | El sistema debe implementar **perft**: una función que cuente los nodos hoja a una profundidad dada, usando generación legal y *make/unmake*. | Must | Los conteos coinciden con los valores de referencia del anexo B para todas las posiciones y profundidades listadas. |

## 3.3 Requisitos no funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| RNF-09 | La generación de movimientos no debe realizar **asignaciones dinámicas** (`Vec`, `Box` ni similares); todo se almacena en el *stack*. | Must | Revisión de código: ninguna función del generador asigna memoria dinámica durante la generación. |
| RNF-10 | La generación de tablas con `build.rs` debe ser **determinista** y no depender de fuentes externas de aleatoriedad. | Should | Dos compilaciones limpias producen tablas idénticas. |
| RNF-11 | Debe existir una **medición de rendimiento reproducible** del generador (nodos por segundo en perft). | Should | Existe un *benchmark* que reporta nodos por segundo; la meta numérica se define con el grupo. |
| RNF-12 | El código de esta fase debe cumplir los mismos criterios de calidad que las fases anteriores (RNF-03, RNF-04 y RNF-05). | Must | `cargo test`, `cargo clippy` y `cargo fmt` pasan sin errores ni advertencias, sin `unsafe` injustificado. |

## 3.4 Decisiones de diseño

| ID | Decisión | Justificación | Alternativa descartada |
|---|---|---|---|
| D-14 | **`MoveList` estático de 256 elementos** en el *stack*. | Evita el costo de memoria dinámica en la búsqueda. El máximo conocido de movimientos legales en una posición es 218, así que 256 deja margen. | `Vec<Move>`: más flexible, pero con costo de *heap* en cada nodo. |
| D-15 | **Generación pseudo-legal seguida de filtrado legal por copia** del tablero y del estado (*copy-make*, sin el historial) en la primera versión. | Es simple y correcta: al ejecutar el movimiento sobre la copia se detectan automáticamente clavadas y casos especiales como la captura al paso. `Board` y `PositionState` son `Copy` y pequeños (RNF-02 y RNF-14). | Generación legal directa con máscaras de jaque y piezas clavadas: más rápida, pero más compleja; queda como optimización futura. |
| D-16 | **Magic Bitboards con tablas y números mágicos generados en `build.rs`** e incrustados en el binario. | Elimina el costo de inicialización en tiempo de ejecución y da máxima velocidad de consulta. La implementación basada en rayos sirve como referencia de pruebas. | Generar las tablas al arrancar el programa: más simple, pero con costo en cada inicio. |
| D-17 | **Segmentación en dos grupos** (capturas y silenciosos) desde el generador. | Prepara la búsqueda de quietud, que solo explora capturas. | Generar todo y filtrar después: desperdicia trabajo. |
| D-18 | **Expansión de promociones dentro del generador** en 4 `Move` distintos. | El resto del motor trata cada promoción como un movimiento independiente, sin casos especiales. | Un único movimiento de promoción con la pieza elegida más tarde. |
| D-19 | **Perft como validación obligatoria** del generador y de make/unmake. | Es el método estándar para detectar errores de reglas en la generación; además prueba en conjunto el código de la Fase 2. | Probar solo con casos manuales: no cubre combinaciones raras. |

## 3.5 Riesgos

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Errores sutiles en enroque, captura al paso o promociones | Alto: movimientos ilegales o faltantes | Tests por cada condición (RF-22 a RF-24) y perft contra valores de referencia (RF-28). |
| Cruce de bordes del tablero al desplazar bits de peones o saltadoras | Alto | Máscaras de columnas y tests de bordes (RF-20 y RF-22). |
| Complejidad y tiempo de compilación de `build.rs` para generar los números mágicos | Medio | Semilla fija (RNF-10), pruebas contra la implementación de referencia (RF-21); alternativa: generar las tablas una vez y guardarlas en el repositorio. |
| Costo de copiar el tablero por cada movimiento pseudo-legal | Medio: limita el rendimiento | Medir con perft (RNF-11); si es un problema, pasar a máscaras de jaque y clavadas. |
| Desbordamiento de `MoveList` | Bajo | Capacidad 256 mayor que el máximo conocido (218); comprobación en modo *debug* al agregar. |
| Clasificación inconsistente de promociones y captura al paso entre capturas y silenciosos | Medio | Definirla con el grupo (pregunta 1) y cubrirla con el criterio de RF-19. |

## 3.6 Preguntas abiertas para el grupo

1. En la segmentación, ¿las promociones sin captura se consideran capturas (movimientos "tácticos", como suele hacerse para la búsqueda de quietud) o silenciosos? ¿La captura al paso va en capturas? (Afecta a RF-51 y RF-52 de la Fase 5.)
2. ¿Se mantiene el filtrado por copia en la v1 y se optimiza más tarde, o se usa directamente *make/unmake* para el filtrado?
3. ¿El `build.rs` usa una semilla fija propia o se generan las tablas una vez y se guardan en el repositorio? ¿Se permiten dependencias en el *build*?
4. ¿Cuál es la meta de rendimiento de perft (nodos por segundo)?
5. ¿Qué debe pasar si `MoveList` se desborda: error siempre o solo en modo *debug*?

---

# Fase 4: Estado Histórico y Repeticiones

El estado del juego requiere memoria más allá de la posición actual de las piezas para aplicar las reglas de empate, evaluar el historial y permitir que la IA navegue su árbol de búsqueda.

## 4.1 Alcance de la Fase 4

**Incluye:**
- Estructura `PositionState` completa (turno, derechos de enroque, casilla de captura al paso, reloj de 50 movimientos, número de jugada y hash).
- Zobrist Hashing: tabla de números aleatorios generada en compilación y actualización incremental del hash.
- Ampliación de *make move* y *unmake move* para actualizar el estado completo y el hash, con una estructura de deshacer que guarda una copia del estado previo.
- Historial de posiciones en un arreglo estático.
- Reglas de fin de partida: triple repetición, regla de 50 movimientos, material insuficiente, jaque mate y ahogado.

**No incluye (se cubre en fases posteriores):**
- Evaluación y búsqueda, incluido el tratamiento de las repeticiones dentro del árbol de búsqueda y la tabla de transposición (Fase 5).
- Protocolo UCI (Fase 6).
- Reglas automáticas de 75 movimientos y quíntuple repetición (pendiente de decisión del grupo, pregunta 6).

## 4.2 Requisitos funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| RF-29 | `PositionState` debe contener: **turno** (`Color`), **derechos de enroque**, **casilla de captura al paso** (`Option<Square>`), **reloj de 50 movimientos** (`u8`), **número de jugada** y **hash** (`u64`). | Must | La estructura expone esos campos (o accesos equivalentes) y se carga correctamente desde un FEN y se exporta idéntica (RF-06). |
| RF-30 | Los **derechos de enroque** deben empaquetarse en un único `u8` como máscara de bits: bit 0 = K, bit 1 = Q, bit 2 = k, bit 3 = q. Un derecho debe perderse al mover el rey, al mover una torre desde su casilla inicial y al capturar una torre en su casilla inicial. | Must | Tests unitarios de cada caso de pérdida de derechos, incluida la captura de una torre rival en su casilla inicial. |
| RF-31 | La **casilla de captura al paso** debe establecerse únicamente después de un avance doble de peón y borrarse con cualquier otro movimiento. | Must | Tras `e2e4`, la casilla es `e3`; tras el siguiente movimiento cualquiera, es `None`. |
| RF-32 | El **reloj de 50 movimientos** debe reiniciarse a cero tras una captura o un avance de peón y aumentar en uno en cualquier otro movimiento. | Must | Un movimiento de caballo silencioso incrementa el reloj; una captura y un avance de peón lo reinician. |
| RF-33 | La **tabla de números aleatorios de Zobrist** (`u64`) debe generarse en compilación mediante `build.rs` y almacenarse como un arreglo `const` en el binario. Debe incluir claves para cada pieza en cada casilla, el turno, los derechos de enroque y la columna de captura al paso. | Must | Todas las claves son distintas y distintas de cero; las dimensiones coinciden con lo requerido (12 × 64 piezas-casillas, 1 de turno, 16 de enroque, 8 de columna). Dos compilaciones limpias producen la misma tabla (RNF-10). |
| RF-34 | El sistema debe poder **calcular el hash desde cero** a partir de un tablero y su estado. | Must | Tras cargar cualquier FEN de prueba, el hash almacenado es igual al hash calculado desde cero. |
| RF-35 | Durante *make move*, el hash debe **actualizarse de forma incremental** con operaciones XOR (piezas movidas y capturadas, cambio de turno, cambios de enroque y de captura al paso), sin recorrer el tablero. | Must | Para cada tipo de movimiento del anexo A, el hash incremental es igual al calculado desde cero; esta comprobación se ejecuta también como aserción durante los tests de perft. |
| RF-36 | La **estructura de deshacer** (definida en la Fase 2) debe guardar la **pieza capturada** y una **copia exacta del `PositionState`** previo al movimiento (turno, enroque, captura al paso, reloj, número de jugada y hash), de modo que *unmake move* restaure el estado en tiempo constante. | Must | Para cada caso de prueba, `unmake(make(posición))` devuelve un `Board` y un `PositionState` iguales a los originales, hash incluido. |
| RF-37 | El **historial de posiciones** debe mantenerse en un **arreglo estático de tamaño fijo** (por ejemplo, `[u64; 512]`) con un índice interno, sin usar `Vec`. *Make move* agrega el hash de la posición y *unmake move* lo retira. | Must | Tras una secuencia de *make* y *unmake*, el historial vuelve exactamente a su estado anterior; el código no usa memoria dinámica. |
| RF-38 | El sistema debe **detectar la triple repetición** contando las apariciones del hash actual en el historial, sin mirar más atrás del último movimiento irreversible (captura o avance de peón). | Must | Partiendo de la posición inicial, la secuencia `g1f3 g8f6 f3g1 f6g8 g1f3 g8f6 f3g1 f6g8` hace que la posición inicial aparezca tres veces y se reporte empate; con una pieza movida distinta no se reporta. |
| RF-39 | El sistema debe detectar el empate por la **regla de los 50 movimientos** cuando el reloj llegue a 100 medios movimientos, salvo que el último movimiento haya dado jaque mate. | Must | Una posición con reloj 100 (anexo B) se reporta como empate; si el movimiento que llevó el reloj a 100 dio jaque mate, se reporta jaque mate. |
| RF-40 | El sistema debe detectar el empate por **material insuficiente**. | Should | Rey contra rey y rey con una pieza menor contra rey se reportan como empate; una posición con una torre o un peón no. |
| RF-41 | El sistema debe detectar el **jaque mate** y el **ahogado**: sin movimientos legales, es jaque mate si el rey está en jaque y ahogado en caso contrario. | Must | Las posiciones de jaque mate y ahogado del anexo B se clasifican correctamente. |

## 4.3 Requisitos no funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| RNF-13 | *Make move* y *unmake move*, incluida la actualización del hash y del historial, deben ser de **tiempo constante** y sin asignaciones dinámicas. | Must | Revisión de código: no hay bucles sobre el tablero completo ni uso de `Vec`, `Box` o similares en esas operaciones. |
| RNF-14 | `PositionState` debe ser **liviano**: `Copy`, comparable (`PartialEq`) y de tamaño pequeño, porque se copia en el filtrado de legalidad (D-15) y en cada estructura de deshacer. | Must | `PositionState` implementa `Copy` y `PartialEq`, y `size_of::<PositionState>() <= 16` bytes. |
| RNF-15 | El código de esta fase debe cumplir los mismos criterios de calidad que las fases anteriores (RNF-03, RNF-04 y RNF-05). | Must | `cargo test`, `cargo clippy` y `cargo fmt` pasan sin errores ni advertencias, sin `unsafe` injustificado. |

## 4.4 Decisiones de diseño

| ID | Decisión | Justificación | Alternativa descartada |
|---|---|---|---|
| D-20 | La estructura de estado se llama **`PositionState`** y es independiente de `Board`. | Cierra la elección de nombre pendiente de la Fase 1 (D-03) y mantiene `Board` limitado a la disposición física. | `GameState` o `Position`. |
| D-21 | El **hash de Zobrist vive en `PositionState`** como un campo `u64`, con actualización incremental mediante XOR. | Depende del turno, el enroque y la captura al paso, que ya están en esa estructura; evita recalcular todo el tablero en cada movimiento. | Guardarlo en `Board` o recalcularlo en cada nodo. |
| D-22 | **Tabla de Zobrist generada en compilación** con `build.rs` y almacenada como `const`. | Sin costo de inicialización en tiempo de ejecución y resultados reproducibles. Reutiliza el mecanismo de `build.rs` de la Fase 3 (D-16). | Generarla al arrancar con un generador pseudoaleatorio: más simple, pero con costo en cada inicio. |
| D-23 | **Derechos de enroque como máscara de bits en un `u8`.** | Permite operaciones binarias rápidas y ocupa un solo byte. | Cuatro booleanos: más legibles, pero más grandes y más lentos de combinar. |
| D-24 | La **estructura de deshacer guarda una copia completa del `PositionState`** previo, más la pieza capturada. | *Unmake* en tiempo constante, sin recalcular enroque, captura al paso, reloj ni hash; es simple y robusta. | Revertir cada campo individualmente (por ejemplo, deshacer el hash con XOR): más compacto, pero con más riesgo de errores. |
| D-25 | **Historial en arreglo estático `[u64; N]` fuera de `PositionState`**, con índice interno. | Evita memoria dinámica; mantenerlo fuera del estado evita copiar cientos de bytes en cada copia del filtrado de legalidad (D-15). | `Vec<u64>`: más flexible, pero con peticiones al *heap*. Incluir el arreglo dentro de `PositionState`: haría costosa cada copia. |
| D-26 | **La búsqueda usa make/unmake y la validación de legalidad usa copia**, como se definió en la Fase 2 (D-12) y la Fase 3 (D-15). | Cada mecanismo se usa donde resulta más simple o más rápido; la copia del filtrado no toca el historial. | Usar un solo mecanismo para todo. |

## 4.5 Riesgos

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Hash incremental distinto del hash calculado desde cero | Alto: repeticiones falsas o no detectadas, y errores silenciosos en la tabla de transposición | Aserción que compara ambos hashes durante perft y tests por cada tipo de movimiento (RF-35). |
| Derechos de enroque mal actualizados (por ejemplo, al capturar una torre en su casilla inicial) | Alto: enroques ilegales | Tests específicos (RF-30) y perft de la Fase 3. |
| Tratamiento de la casilla de captura al paso en el hash: FEN la registra tras cada avance doble, aunque no haya captura posible | Medio: repeticiones no detectadas | Definir la regla con el grupo (pregunta 2) y cubrirla con tests. |
| Desbordamiento del historial de tamaño fijo (partidas muy largas o posiciones UCI con muchos movimientos) | Medio | Comprobación al agregar; decidir la política con el grupo (pregunta 3); afecta a RF-55 de la Fase 6. |
| Mate dado en el movimiento 100 del reloj tratado como empate | Bajo | Criterio explícito en RF-39. |
| Significado distinto de repetición en la búsqueda (suele bastar una repetición) y en la partida real (triple) | Medio: decisiones erróneas de la IA | Definirlo con el grupo (pregunta 1); afecta a RF-50 de la Fase 5. |

## 4.6 Preguntas abiertas para el grupo

1. Dentro de la búsqueda (Fase 5, RF-50), ¿una única repetición cuenta como empate (como suelen hacer los motores) o se exige la triple repetición también allí?
2. ¿La casilla de captura al paso entra en el hash siempre que hubo avance doble, o solo cuando existe una captura al paso posible?
3. Si el historial de tamaño fijo se llena (por ejemplo, al recibir por UCI una partida muy larga, RF-55), ¿se considera error, se descartan las posiciones más antiguas o se aumenta el tamaño?
4. ¿La tabla de Zobrist usa una semilla fija propia en `build.rs` o se generan las claves una vez y se guardan en el repositorio?
5. ¿Qué alcance tiene el material insuficiente en la v1: solo rey contra rey y rey con pieza menor contra rey, o también alfiles del mismo color?
6. ¿Se aplican las reglas automáticas de 75 movimientos y quíntuple repetición, o solo las reclamaciones por 50 movimientos y triple repetición?
7. ¿La detección de fin de partida vive en `PositionState` o en una capa superior (por ejemplo, una estructura `Game`)?

---

# Fase 5: Raciocinio y Evaluación (El "Cerebro")

Es el componente encargado de decidir qué movimiento ejecutar. Se enfoca en maximizar la velocidad de cálculo evitando el uso de memoria dinámica.

## 5.1 Alcance de la Fase 5

**Incluye:**
- Función de evaluación estática: valoración material y *Piece-Square Tables* (PSTs).
- Búsqueda Minimax con poda alfa-beta.
- Búsqueda iterativa en profundidad (*iterative deepening*), con límites de profundidad y de tiempo y una señal de parada externa.
- Ordenamiento de movimientos con la heurística MVV-LVA.
- Búsqueda de quietud (*quiescence search*).
- Tratamiento del fin de partida dentro de la búsqueda (mate, ahogado y empates).
- Resultado de la búsqueda (mejor movimiento, puntaje, profundidad y nodos).

**No incluye (se cubre en fases posteriores o queda fuera de la v1):**
- Tabla de transposición (queda fuera del alcance inicial para mantener la simplicidad; el hash de Zobrist de la Fase 4 deja la base preparada).
- Otras heurísticas de búsqueda: movimientos *killer*, *history*, poda por movimiento nulo, reducciones, ventanas de aspiración.
- Evaluación avanzada: distinción entre medio juego y final, seguridad del rey, movilidad, estructura de peones.
- Variante principal (*principal variation*), pendiente de decisión del grupo (pregunta 7).
- Interpretación de los comandos UCI y cálculo del reparto de tiempo a partir de los relojes: se cubren en la Fase 6 (RF-58 y RF-63). Esta fase solo recibe los límites ya calculados.

## 5.2 Requisitos funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| RF-42 | La **evaluación estática** debe devolver un `i32` que represente la ventaja de la posición **desde el punto de vista del bando que mueve**: positivo si ese bando está mejor y negativo si está peor. | Must | Para la posición inicial la evaluación es 0; si se retira la dama negra, la evaluación es positiva con las blancas al turno y negativa con las negras al turno. |
| RF-43 | La **valoración material** debe asignar los puntajes: Peón = 100, Caballo = 300, Alfil = 300, Torre = 500, Dama = 900, Rey = 10000. El valor del rey no debe sumar en la evaluación (ambos bandos tienen un rey); se conserva para el orden de MVV-LVA. | Must | La componente de material de una posición sin la dama blanca vale −900 desde el punto de vista de las blancas; la de la posición inicial vale 0. |
| RF-44 | El sistema debe usar **Piece-Square Tables**: un arreglo estático `const [i32; 64]` por tipo de pieza (una sola tabla por tipo, sin separar medio juego y final). Las piezas negras deben usar la misma tabla con la casilla reflejada verticalmente. | Must | Para un conjunto de posiciones y sus espejos (colores invertidos y tablero reflejado, anexo B), la evaluación desde el punto de vista del bando que mueve es idéntica. |
| RF-45 | El sistema debe definir **puntajes de mate** separados del material, mayores que cualquier evaluación estática posible y **ajustados por distancia**, de modo que se prefiera el mate más cercano y se retrase la derrota más lejana. | Must | `MATE_SCORE` es mayor que la cota máxima de evaluación estática; un mate en 1 puntúa más que un mate en 3; los valores usados como límites iniciales de la búsqueda (infinito) no provocan desbordamiento al negarlos. |
| RF-46 | El sistema debe implementar **Minimax con poda alfa-beta** a profundidad fija. | Must | Para un conjunto de posiciones y una misma profundidad, con la misma función de evaluación en las hojas, el puntaje con poda es igual al de un Minimax sin poda. |
| RF-47 | El sistema debe implementar **búsqueda iterativa en profundidad**: buscar a profundidad 1, luego 2, luego 3, y conservar siempre el mejor movimiento de la última iteración completada. | Must | Con un límite de tiempo muy corto, la búsqueda devuelve igualmente un movimiento legal (el de la profundidad 1 completada) e informa la profundidad alcanzada. |
| RF-48 | La búsqueda debe aceptar **límites** de profundidad máxima y de tiempo máximo, comprobar el tiempo cada cierta cantidad de nodos y poder **detenerse mediante una señal externa**. | Must | Una búsqueda con profundidad 4 termina al completar la profundidad 4; una con límite de tiempo termina dentro de ese límite más un margen definido por el grupo; activar la señal de parada termina la búsqueda y devuelve un movimiento legal. |
| RF-49 | La búsqueda debe devolver un **resultado** con: mejor movimiento (vacío si no hay movimientos legales), puntaje, profundidad completada y cantidad de nodos visitados. | Must | En una posición con movimientos legales el resultado incluye un movimiento legal; en una posición de jaque mate o ahogado el mejor movimiento está vacío. |
| RF-50 | Dentro de la búsqueda, un nodo **sin movimientos legales** debe puntuarse como mate (con el ajuste de distancia de RF-45) si el rey está en jaque, o como 0 si es ahogado. Los nodos de **empate** (regla de 50 movimientos, material insuficiente y repetición según la regla decidida en la Fase 4) deben puntuarse 0. | Must | En la posición de mate en uno del anexo B la búsqueda encuentra el mate y su puntaje es de mate con distancia 1; las posiciones de ahogado y de reloj 100 del anexo B puntúan 0. |
| RF-51 | El **ordenamiento de movimientos** debe aplicar **MVV-LVA**: las capturas se ordenan primero por el valor de la víctima (de mayor a menor) y, a igual víctima, por el valor del atacante (de menor a mayor); todas las capturas se exploran antes que los movimientos silenciosos, aprovechando la segmentación de la Fase 3. La captura al paso se trata como peón que captura peón. | Must | Peón captura dama se ordena antes que dama captura peón; una captura cualquiera se ordena antes que un movimiento silencioso; el ordenamiento no usa memoria dinámica. |
| RF-52 | Al alcanzar la profundidad máxima, la búsqueda debe continuar con **búsqueda de quietud**: evaluar la posición estática como cota inferior (*stand pat*) y explorar **únicamente movimientos de captura**, ordenados con MVV-LVA y con poda alfa-beta, hasta llegar a una posición estable. | Must | En la posición del peón envenenado del anexo B, la búsqueda a profundidad 1 con quietud no elige `d2d5`; sin quietud sí la elegiría. |

## 5.3 Requisitos no funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| RNF-16 | La búsqueda no debe realizar **asignaciones dinámicas**: cada nivel de recursión usa su propia `MoveList` en el *stack* y la profundidad máxima está acotada por una constante. | Must | Revisión de código: no hay uso de `Vec`, `Box` ni similares en la búsqueda; existe una constante de profundidad máxima (`MAX_PLY`) que ninguna rama puede superar. |
| RNF-17 | Las tablas de evaluación (PSTs y valores de piezas) deben ser constantes (`const`) alojadas en el binario, sin cálculo en tiempo de ejecución. | Must | Las tablas están declaradas como `const` y no se modifican durante la ejecución. |
| RNF-18 | Debe existir una **medición de rendimiento reproducible** de la búsqueda: profundidad fija sobre un conjunto de posiciones de referencia, con nodos, tiempo y nodos por segundo. | Should | Existe un *benchmark* que reporta esos tres valores; la meta numérica se define con el grupo. |
| RNF-19 | La búsqueda debe ser **determinista**: la misma posición con el mismo límite de profundidad produce el mismo mejor movimiento y el mismo puntaje. | Should | Dos ejecuciones sucesivas sobre las posiciones del anexo B con límite de profundidad dan resultados idénticos. |
| RNF-20 | El código de esta fase debe cumplir los mismos criterios de calidad que las fases anteriores (RNF-03, RNF-04 y RNF-05). | Must | `cargo test`, `cargo clippy` y `cargo fmt` pasan sin errores ni advertencias, sin `unsafe` injustificado. |

## 5.4 Decisiones de diseño

| ID | Decisión | Justificación | Alternativa descartada |
|---|---|---|---|
| D-27 | **Evaluación simple**: material más una única PST `const` por tipo de pieza, sin separar medio juego y final. | Es fácil de implementar y de probar, y sirve como base para mejorar después. | Evaluación con interpolación entre medio juego y final: más fuerte, pero más compleja. |
| D-28 | **Valor del rey = 10000** solo como referencia para MVV-LVA; **no suma** en la evaluación. | Ambos bandos siempre tienen un rey, por lo que su valor se cancela; usarlo para el mate mezclaría escalas. | Usar el valor del rey como puntaje de mate. |
| D-29 | **Minimax con poda alfa-beta implementado en su formulación negamax** (propuesta), con la evaluación desde el punto de vista del bando que mueve. | Es equivalente a Minimax y evita duplicar el código de maximizar y minimizar. | Dos funciones separadas para máximo y mínimo. |
| D-30 | **Puntaje de mate separado del material y ajustado por distancia.** | Permite preferir el mate más rápido y evitar confundirlo con una gran ventaja material. | Tratar el mate como una captura del rey. |
| D-31 | **Iterative deepening** con límites de profundidad y de tiempo y señal de parada externa. | Siempre hay un mejor movimiento disponible y se puede responder dentro del tiempo. | Búsqueda de profundidad fija sin control de tiempo. |
| D-32 | **Ordenamiento MVV-LVA** apoyado en la segmentación de capturas de la Fase 3 (D-17). | Mejora mucho la poda con poco costo; la pieza capturada y la atacante se consultan en `Board` (D-11). | Sin ordenamiento, o ordenamiento por intercambio estático (SEE), más complejo. |
| D-33 | **Quiescence search con *stand pat*** y solo capturas. | Evita el efecto horizonte; el *stand pat* acota la cantidad de nodos. | Cortar la búsqueda a profundidad fija; incluir también jaques en la quietud (más preciso, más costoso). |
| D-34 | **Sin tabla de transposición ni otras heurísticas avanzadas** en la v1. | Mantiene la fase simple; el hash de Zobrist queda disponible para incorporarla después. | Incluir la tabla de transposición desde el inicio. |
| D-35 | La búsqueda usa **la generación legal de la Fase 3** en la v1 (propuesta). | Es más simple y facilita la detección de mate y ahogado (lista vacía). | Generación pseudo-legal más comprobación de jaque tras *make/unmake*: evita trabajo duplicado, pero complica la detección de fin de partida. |

## 5.5 Riesgos

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Errores de signo o de perspectiva en la evaluación y en la búsqueda | Alto: el motor juega al revés | Test de simetría con posiciones espejo (RF-44) y de equivalencia con Minimax sin poda (RF-46). |
| Desbordamiento de `i32` al negar el valor de "infinito" en la poda | Alto: pánico en modo *debug* o comportamiento errado | Usar límites de búsqueda holgados dentro de `i32` y comprobarlo en RF-45. |
| Confusión entre la escala del material y la del mate | Medio | Puntajes de mate separados y mayores que cualquier evaluación (RF-45, D-30). |
| La PST única es débil en los finales (por ejemplo, el rey se queda en la esquina) | Medio: juego pobre en finales | Aceptado para la v1; reevaluar con evaluación interpolada (pregunta 6). |
| Explosión de nodos en la búsqueda de quietud | Medio: respuestas lentas | *Stand pat*, ordenamiento MVV-LVA y cota de profundidad máxima (RNF-16). |
| Pasarse del límite de tiempo por comprobarlo poco seguido | Medio | Comprobar cada cierta cantidad de nodos y definir el margen aceptable (pregunta 3). |
| Trabajo duplicado entre la generación legal por copia y el *make/unmake* de la búsqueda | Medio: limita el rendimiento | Medir con el *benchmark* (RNF-18) y reconsiderar D-35 (pregunta 4). |
| Reglas de repetición en la búsqueda sin decidir | Medio | Resolver la pregunta 1 de la Fase 4. |

## 5.6 Preguntas abiertas para el grupo

1. ¿La evaluación se expresa desde el punto de vista del bando que mueve (como propone el documento con negamax) o siempre desde las blancas?
2. ¿Qué valores concretos se usan para `MATE_SCORE`, el valor de "infinito" y `MAX_PLY`?
3. ¿Qué meta de rendimiento se persigue (profundidad alcanzada en cierto tiempo) y qué margen de tiempo se tolera al pasarse del límite?
4. ¿La búsqueda usa la generación legal de la Fase 3 o la pseudo-legal con comprobación de jaque tras ejecutar el movimiento?
5. En la búsqueda de quietud, ¿se incluyen las promociones sin captura y la captura al paso? (Depende de la pregunta 1 de la Fase 3.)
6. ¿Se acepta la PST única en la v1 o se prefiere incluir la distinción entre medio juego y final?
7. ¿Se devolverá la variante principal para mostrarla en la interfaz UCI (Fase 6, RF-61)?

---

# Fase 6: Interfaz y Ciclo de Juego (Game Loop)

Es la capa que conecta el motor con el exterior. El motor se comunica exclusivamente mediante el protocolo UCI, escuchando comandos mientras calcula y administrando el tiempo disponible.

## 6.1 Alcance de la Fase 6

**Incluye:**
- Bucle UCI por `stdin` y `stdout`, con los comandos `uci`, `isready`, `ucinewgame`, `position`, `go`, `stop` y `quit`.
- Interpretación de posiciones: `startpos`, FEN (mediante un crate externo) y lista de movimientos.
- Traducción de movimientos en texto UCI a `Move`.
- Arquitectura de dos hilos: uno de entrada y salida (E/S) y otro de búsqueda, con interrupción segura de la búsqueda.
- Gestión del tiempo a partir de `wtime`, `btime`, `winc` y `binc` (y demás parámetros de `go`).
- Salida de `info` y `bestmove`.
- Representación ASCII y otros comandos de diagnóstico, solo para depuración.

**No incluye (queda fuera de la v1 o se evalúa después):**
- Una interfaz de consola propia para jugar partidas (el motor se usa a través de una GUI compatible con UCI).
- *Ponder* (`go ponder` y `ponderhit`), MultiPV, `go searchmoves` y Chess960.
- Opciones UCI configurables (por ejemplo, tamaño de tabla de transposición o cantidad de hilos), salvo lo que se decida en la pregunta 3.
- Búsqueda paralela con varios hilos de búsqueda.

## 6.2 Requisitos funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| RF-53 | El motor debe **leer comandos UCI por `stdin` y responder por `stdout`**. Los comandos desconocidos deben ignorarse y una entrada mal formada no debe terminar el programa. | Must | Una sesión con comandos válidos, desconocidos y mal formados no provoca pánico ni cierre del programa; los desconocidos no generan salida de error de protocolo. |
| RF-54 | El motor debe responder al *handshake*: `uci` con `id name`, `id author` y `uciok`; `isready` con `readyok` (también mientras hay una búsqueda activa); y `ucinewgame` debe reiniciar el estado de la partida (posición e historial). | Must | La respuesta a `uci` termina en `uciok`; `isready` durante una búsqueda larga responde `readyok` sin esperar a que termine; tras `ucinewgame` el historial queda vacío. |
| RF-55 | El comando **`position [startpos \| fen <FEN>] [moves <m1> <m2> ...]`** debe establecer la posición indicada y aplicar los movimientos en orden, actualizando estado, hash e historial. Si la posición o algún movimiento es inválido, debe informarse el error y **conservarse la posición anterior**. | Must | `position startpos moves e2e4 e7e5` deja la posición esperada y un historial de 2 entradas; `position startpos moves e2e5` informa el error y la posición previa no cambia. |
| RF-56 | El sistema debe **convertir un movimiento en texto UCI en un `Move`** de la posición actual, buscándolo entre los movimientos legales por su texto (RF-17). Si ninguno coincide, debe devolver un error. | Must | Para todos los movimientos legales de las posiciones de prueba (incluida Kiwipete), convertir el texto de cada movimiento devuelve el mismo `Move`; el enroque `e1g1` y la promoción `e7e8q` se interpretan correctamente; `e2e5` en la posición inicial devuelve error. |
| RF-57 | La cadena **FEN** recibida por UCI debe interpretarse mediante un **crate externo** de la comunidad de Rust, y el resultado debe convertirse a `Board` y `PositionState`, calculando el hash (RF-34). Un FEN inválido debe devolver un error sin terminar el programa. | Must | Para el conjunto de FEN válidos de prueba, la posición resultante coincide con la esperada; los FEN inválidos de RF-07 devuelven error. |
| RF-58 | El comando **`go`** debe aceptar los parámetros `wtime`, `btime`, `winc`, `binc`, `movestogo`, `depth`, `movetime` e `infinite`, traducirlos a los límites de la búsqueda (RF-48) e iniciar la búsqueda en el hilo de búsqueda **sin bloquear la lectura de comandos**. | Must | `go depth 4` limita la búsqueda a profundidad 4; `go movetime 500` termina cerca de los 500 ms; `go infinite` no termina por sí sola; durante la búsqueda el motor sigue respondiendo a `isready`. |
| RF-59 | El comando **`stop`** debe interrumpir la búsqueda mediante la señal externa (RF-48) y el motor debe responder con el mejor movimiento encontrado hasta ese momento. El comando **`quit`** debe detener la búsqueda y cerrar el programa de forma ordenada, sin dejar hilos activos. | Must | `go infinite` seguido de `stop` produce un `bestmove` legal; tras `quit` el proceso termina sin errores ni hilos pendientes. |
| RF-60 | El motor debe emitir **`bestmove <movimiento>` exactamente una vez por cada `go`**. Si no hay movimientos legales, debe emitir `bestmove 0000`. Con `go infinite`, no debe emitirse hasta recibir `stop`. | Must | En una posición de mate o ahogado (anexo B) la respuesta es `bestmove 0000`; en una secuencia de varios `go` y `stop` hay un `bestmove` por cada `go`, ni más ni menos. |
| RF-61 | Tras cada iteración completa de la búsqueda, el motor debe emitir una línea **`info`** con la profundidad, el puntaje (`score cp <n>` o `score mate <n>`), los nodos y el tiempo. Los puntajes de mate (RF-45) deben convertirse a la cantidad de movimientos hasta el mate. | Should | Para el mate en uno del anexo B se emite `score mate 1`; cada línea `info` de una búsqueda a profundidad 4 tiene profundidad creciente. |
| RF-62 | El sistema debe usar **dos hilos**: un **hilo de E/S** que escucha permanentemente el protocolo y un **hilo de búsqueda** dedicado a la IA, al que el hilo de E/S puede interrumpir de forma segura. El hilo de búsqueda debe trabajar sobre una **copia** de la posición. | Must | Durante una búsqueda larga el hilo de E/S procesa `isready` y `stop`; modificar la posición con `position` no afecta a una búsqueda en curso. |
| RF-63 | El motor debe contar con un **gestor de tiempo** que, a partir de `wtime`, `btime`, `winc`, `binc` (y `movestogo`, si se recibe) del bando que mueve, calcule el tiempo disponible para la jugada, defina los límites de la búsqueda y **envíe la señal de detención** al ciclo de *iterative deepening* al agotarse, de modo que se devuelva la mejor jugada encontrada. | Must | El tiempo asignado nunca supera el tiempo restante menos un margen de seguridad; con muy poco tiempo restante el motor igualmente responde con un movimiento legal; en una partida simulada con relojes cortos no se pierde por tiempo. |
| RF-64 | El sistema debe ofrecer **comandos de diagnóstico no estándar**: `d`, que imprime el tablero en ASCII (RF-09), el FEN y el hash; y `go perft <n>`, que ejecuta perft (RF-28). | Could | `d` en la posición inicial imprime el tablero con la orientación correcta; `go perft 4` en la posición inicial informa 197.281 nodos. |

## 6.3 Requisitos no funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| RNF-21 | Por **`stdout`** solo debe salir texto del protocolo UCI (y, si se implementan, las respuestas de los comandos de diagnóstico); los mensajes de diagnóstico y errores se escriben por `stderr`, y cada línea de salida se vacía (*flush*) inmediatamente. | Must | En una sesión de prueba, todas las líneas de `stdout` son respuestas UCI válidas y llegan sin demora; los errores de RF-55 aparecen en `stderr`. |
| RNF-22 | La concurrencia debe ser **segura**: comunicación entre hilos con tipos de la biblioteca estándar (valores atómicos, canales o *mutex*), sin `unsafe`, sin carreras de datos y sin interbloqueos. | Must | Una prueba de estrés con cientos de secuencias `go` y `stop` consecutivas termina sin bloqueos ni pánicos y con un `bestmove` por cada `go`. |
| RNF-23 | La **latencia de respuesta a `stop`** debe ser baja. | Should | Tras recibir `stop`, el `bestmove` se emite dentro de un margen definido por el grupo (por ejemplo, unas decenas de milisegundos). |
| RNF-24 | El motor debe ser **compatible con una GUI estándar de UCI** (por ejemplo, Cute Chess). | Should | Una partida completa contra otro motor a través de la GUI termina sin movimientos ilegales, sin pérdidas por tiempo atribuibles al motor y sin errores de protocolo. |
| RNF-25 | Las **dependencias externas** deben ser mínimas y estar documentadas (nombre, versión fija en `Cargo.lock`, licencia y motivo). | Should | Existe una lista de dependencias revisada; el crate de FEN tiene licencia compatible y mantenimiento reciente. |
| RNF-26 | El código de esta fase debe cumplir los mismos criterios de calidad que las fases anteriores (RNF-03, RNF-04 y RNF-05). | Must | `cargo test`, `cargo clippy` y `cargo fmt` pasan sin errores ni advertencias, sin `unsafe` injustificado. |

## 6.4 Decisiones de diseño

| ID | Decisión | Justificación | Alternativa descartada |
|---|---|---|---|
| D-36 | **UCI como única interfaz**; la representación ASCII queda como herramienta de depuración. | Una GUI estándar ya ofrece tablero, relojes y partidas; evita mantener una consola propia. | CLI propia con tablero en ASCII y comandos como `e2e4` (fase prevista en el primer borrador). |
| D-37 | **Dos hilos** (E/S y búsqueda) creados con la biblioteca estándar, con parada mediante una **señal atómica** compartida y búsqueda sobre una copia de la posición. | El hilo de E/S puede responder a `isready` y `stop` mientras se calcula; las estructuras son `Copy` o pequeñas, por lo que copiarlas es barato. | Un solo hilo que consulta `stdin` dentro de la búsqueda: más simple, pero mezcla E/S con la búsqueda y dificulta una futura ampliación (por ejemplo, *ponder*). |
| D-38 | **Crate externo para interpretar FEN**, con una capa de adaptación hacia `Board` y `PositionState`. | Aprovecha un parser probado y evita reimplementar la validación. | Parser propio: más control y sin dependencias, pero más código que mantener. |
| D-39 | **Traducción de movimientos UCI buscando en la lista de movimientos legales.** | Reutiliza la generación legal y `Display` (RF-17); las banderas (captura, enroque, captura al paso) quedan correctas sin deducirlas. | Construir el `Move` directamente desde el texto: más rápido, pero más propenso a errores en casos especiales. |
| D-40 | **Gestor de tiempo como submódulo independiente** que calcula el tiempo disponible y controla la detención de la búsqueda, apoyado en los límites y la señal de la Fase 5 (RF-48). | Separa la política de tiempo de la búsqueda y permite ajustarla sin tocarla. | Tiempo fijo por jugada: más simple, pero con riesgo de perder por tiempo. |
| D-41 | **Dos límites de tiempo** (propuesta): uno *blando*, para no comenzar una iteración que probablemente no termine, y uno *duro*, para abortar la búsqueda. | Aprovecha mejor el tiempo y reduce el trabajo descartado al interrumpir una iteración. | Un único límite: más simple, pero desperdicia parte del tiempo. |
| D-42 | **Sin *ponder* ni opciones UCI en la v1** (propuesta). | Mantiene el alcance acotado. | Implementarlos desde el inicio: más completo, pero amplía la complejidad del manejo de hilos y de tiempo. |

## 6.5 Riesgos

| Riesgo | Impacto | Mitigación |
|---|---|---|
| Pérdida de partidas por tiempo (latencia de la GUI, cálculo incorrecto del presupuesto) | Alto | Margen de seguridad (RF-63), pruebas con relojes cortos y, si se decide, una opción UCI para ajustar la latencia (pregunta 3). |
| Interbloqueos, carreras de datos o `bestmove` duplicado o ausente | Alto: la GUI deja de responder o declara fallo del motor | Un único emisor de `bestmove`, comunicación con tipos de la biblioteca estándar y pruebas de estrés (RNF-22). |
| Texto no UCI en `stdout` que confunde a la GUI | Medio | Diagnóstico por `stderr` (RNF-21). |
| El crate de FEN no coincide con las estructuras propias, no permite exportar FEN o queda sin mantenimiento | Medio: retrasos o cambios en RF-06 | Evaluarlo antes de adoptarlo (pregunta 1), usar una capa de adaptación y tener como alternativa un parser propio. |
| Una partida larga recibida por `position ... moves` desborda el historial de tamaño fijo | Medio | Decidir la política en la pregunta 3 de la Fase 4; comprobación al agregar. |
| Comandos o parámetros inesperados de distintas GUI | Medio | Ignorar lo desconocido (RF-53) y probar con más de una GUI (RNF-24). |
| Comprobar el tiempo con poca frecuencia en la búsqueda, que supera el límite | Medio | Frecuencia de comprobación definida en la Fase 5 (RF-48) y margen de seguridad en RF-63. |

## 6.6 Preguntas abiertas para el grupo

1. ¿Qué crate se usará para interpretar FEN y qué criterios debe cumplir (mantenimiento, licencia, soporte para exportar FEN)? ¿Se aceptan otras dependencias o solo la biblioteca estándar para el resto?
2. ¿Cómo se calcula el tiempo por jugada: cuántas jugadas restantes se asumen si no llega `movestogo`, qué parte del incremento se usa y qué margen de seguridad se deja?
3. ¿Se expone alguna opción UCI (por ejemplo, un margen de latencia) o la v1 no tiene opciones?
4. ¿Qué hace el motor si recibe `go` mientras hay una búsqueda activa: lo ignora o detiene la anterior y comienza la nueva?
5. ¿Se incluyen los comandos de diagnóstico `d` y `go perft`?
6. ¿Con qué GUI o herramienta se validará la compatibilidad y cómo (por ejemplo, partidas contra otro motor)?
7. ¿Se implementará *ponder* en el futuro? De ser así, conviene tenerlo en cuenta en el diseño de los hilos.

---

# Resumen de identificadores por fase

| Fase | Requisitos funcionales | Requisitos no funcionales | Decisiones de diseño |
|---|---|---|---|
| 1. Representación del tablero | RF-01 a RF-10 | RNF-01 a RNF-05 | D-01 a D-07 |
| 2. Representación del movimiento | RF-11 a RF-17 | RNF-06 a RNF-08 | D-08 a D-13 |
| 3. Generador de movimientos | RF-18 a RF-28 | RNF-09 a RNF-12 | D-14 a D-19 |
| 4. Estado histórico y repeticiones | RF-29 a RF-41 | RNF-13 a RNF-15 | D-20 a D-26 |
| 5. Raciocinio y evaluación | RF-42 a RF-52 | RNF-16 a RNF-20 | D-27 a D-35 |
| 6. Interfaz y ciclo de juego | RF-53 a RF-64 | RNF-21 a RNF-26 | D-36 a D-42 |

---

# Preguntas transversales prioritarias

Estas decisiones afectan a varias fases y conviene resolverlas primero:

1. **Dependencias externas** (Fase 1, pregunta 2; Fase 3, pregunta 3; Fase 6, pregunta 1): qué crates se permiten, incluido el de FEN y el uso de `build.rs`.
2. **Repeticiones dentro de la búsqueda** (Fase 4, pregunta 1; Fase 5, RF-50): una repetición o triple repetición.
3. **Promociones y captura al paso en capturas y en la quietud** (Fase 3, pregunta 1; Fase 5, pregunta 5).
4. **Generación legal o pseudo-legal en la búsqueda** (Fase 3, pregunta 2; Fase 5, pregunta 4).
5. **Historial de tamaño fijo** y partidas largas recibidas por UCI (Fase 4, pregunta 3; Fase 6, RF-55).
6. **Metas de rendimiento y de tiempo** (Fase 3, pregunta 4; Fase 5, pregunta 3; Fase 6, pregunta 2).
7. **Fuerza de juego esperada** (Fase 1, pregunta 3), que condiciona la evaluación simple y la ausencia de tabla de transposición.

---

# Anexo A: Banderas de movimiento (propuesta de referencia)

Codificación habitual de 4 bits. Se propone usarla como valores explícitos del `enum` de banderas (D-09).

| Valor | Tipo de movimiento |
|---|---|
| 0 | Silencioso |
| 1 | Avance doble de peón |
| 2 | Enroque corto |
| 3 | Enroque largo |
| 4 | Captura |
| 5 | Captura al paso |
| 8 | Promoción a caballo |
| 9 | Promoción a alfil |
| 10 | Promoción a torre |
| 11 | Promoción a dama |
| 12 | Promoción a caballo con captura |
| 13 | Promoción a alfil con captura |
| 14 | Promoción a torre con captura |
| 15 | Promoción a dama con captura |

Los valores 6 y 7 quedan sin uso. Son 14 tipos de movimiento válidos dentro de un espacio de 16 valores posibles.

---

# Anexo B: Valores de referencia y posiciones de prueba

Valores publicados por la comunidad (por ejemplo, en la wiki de Chess Programming). **Conviene verificarlos contra la fuente antes de fijarlos como tests.**

## B.1 Conteos de perft

| Posición | FEN | Prof. 1 | Prof. 2 | Prof. 3 | Prof. 4 |
|---|---|---|---|---|---|
| Inicial | `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1` | 20 | 400 | 8.902 | 197.281 |
| Kiwipete | `r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1` | 48 | 2.039 | 97.862 | 4.085.603 |
| Posición 3 | `8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1` | 14 | 191 | 2.812 | 43.238 |
| Posición 4 | `r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1` | 6 | 264 | 9.467 | 422.333 |
| Posición 5 | `rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8` | 44 | 1.486 | 62.379 | 2.103.487 |

Para la posición inicial, la profundidad 5 da 4.865.609 nodos. Se recomienda empezar por las profundidades bajas y subir a medida que el generador se estabiliza.

## B.2 Otras posiciones de prueba

| Propósito | FEN | Resultado esperado |
|---|---|---|
| Máximo de movimientos legales (RF-18) | `R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1` | 218 movimientos legales |
| Captura al paso ilegal por clavada horizontal (RF-27) | `8/8/8/8/k2Pp2Q/8/8/3K4 b - d3 0 1` | La captura al paso `e4xd3` no debe aparecer entre los movimientos legales |
| Jaque mate (RF-41) | `7k/6Q1/6K1/8/8/8/8/8 b - - 0 1` | Sin movimientos legales y rey negro en jaque: jaque mate |
| Ahogado (RF-41) | `7k/5Q2/6K1/8/8/8/8/8 b - - 0 1` | Sin movimientos legales y rey negro sin jaque: ahogado |
| Material insuficiente (RF-40) | `4k3/8/8/8/8/8/8/4K3 w - - 0 1` y `4k3/8/8/8/8/8/8/4KN2 w - - 0 1` | Empate por material insuficiente en ambas |
| Regla de 50 movimientos (RF-39) | `4k3/8/8/8/8/8/8/4K2R w - - 100 80` | Empate por la regla de 50 movimientos |

## B.3 Posiciones de prueba de la búsqueda y la evaluación (Fase 5)

| Propósito | FEN | Resultado esperado |
|---|---|---|
| Mate en uno (RF-50) | `6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1` | Mejor movimiento `a1a8` (mate de pasillo); puntaje de mate con distancia 1 |
| Pieza colgada (RF-46, RF-47) | `4k3/8/8/3q4/8/8/8/3RK3 w - - 0 1` | Mejor movimiento `d1d5` (la torre captura la dama sin defensa) |
| Peón envenenado (RF-52) | `4k3/8/4p3/3p4/8/8/3Q4/4K3 w - - 0 1` | Con quietud, a profundidad 1 el mejor movimiento no es `d2d5` (el peón está defendido por `e6`) |
| Simetría de la evaluación (RF-44), posición A | `rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1` | La evaluación de A y la de su espejo B son iguales (desde el bando que mueve) |
| Simetría de la evaluación (RF-44), posición B (espejo de A) | `rnbqkbnr/pppp1ppp/8/4p3/8/8/PPPPPPPP/RNBQKBNR w KQkq e6 0 1` | Igual que A |
| Evaluación de la posición inicial (RF-42) | `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1` | Evaluación 0 |

## B.4 Sesiones UCI de prueba (Fase 6)

| Propósito | Entrada | Salida esperada |
|---|---|---|
| *Handshake* (RF-54) | `uci` | `id name <nombre>`, `id author <autor>` y `uciok` |
| Disponibilidad (RF-54) | `isready` | `readyok` |
| Posición con movimientos (RF-55) | `position startpos moves e2e4 e7e5` y `go depth 4` | Una o más líneas `info depth ...` y una única línea `bestmove <movimiento>` |
| Movimiento ilegal (RF-55, RF-56) | `position startpos moves e2e5` | Error por `stderr`; la posición anterior no cambia |
| Mate en uno (RF-60, RF-61) | `position fen 6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1` y `go depth 3` | `info ... score mate 1` y `bestmove a1a8` |
| Sin movimientos legales (RF-60) | `position fen 7k/6Q1/6K1/8/8/8/8/8 b - - 0 1` y `go depth 1` | `bestmove 0000` |
| Búsqueda infinita (RF-59, RF-60) | `go infinite`, y después `stop` | `bestmove <movimiento legal>` solo después de `stop`; entre ambos, `isready` responde `readyok` |
| Gestión del tiempo (RF-63) | `go wtime 2000 btime 2000 winc 0 binc 0` | `bestmove` emitido antes de agotar el tiempo restante menos el margen de seguridad |
| Diagnóstico (RF-64) | `go perft 4` en la posición inicial | 197.281 nodos |
