# Fase 7: Interfaz Gráfica de Usuario (GUI en Rust)

**Base:** `requisitos_motor_de_ajedrez_en_rust_v6.md` (Fases 1 a 6) y `planificacion_motor_de_ajedrez_rust.md` (M0 a M7).
**Objetivo de esta fase:** añadir una interfaz de escritorio interactiva que permita jugar contra el motor o usarlo como analizador visual, comunicándose con él **únicamente** mediante el protocolo UCI que ya implementa `src/uci.rs`.

> **Nota de alcance respecto al documento v6:** la planificación lista "interfaz gráfica propia" como fuera de alcance. Esta fase es una ampliación explícita y debe aprobarse como tal. Se numera como Fase 7 porque las Fases 1 a 6 ya existen; los identificadores (RF, RNF, D) continúan desde RF-65, RNF-27 y D-43, que son los siguientes libres tras RF-64, RNF-26 y D-42.

---

## 7.1 Alcance de la Fase 7

**Incluye:**
- Ventana de escritorio en Rust (`egui` / `eframe`) en un **crate separado** del motor (ver D-48).
- Tablero 8×8 escalable con coordenadas a–h y 1–8, con orientación configurable.
- Piezas como imágenes embebidas en el binario.
- Interacción por ratón: clic en origen y destino, o arrastrar y soltar.
- Resaltados: pieza seleccionada, destinos legales, última jugada y rey en jaque.
- Selector modal de promoción (Dama, Torre, Alfil, Caballo).
- Subproceso del motor comunicado por `stdin`/`stdout` con UCI.
- Panel lateral: lista de jugadas, relojes con incremento, barra de evaluación y controles (Nueva partida, Invertir tablero, Detener cálculo, Rendirse).
- Modos de juego: humano contra motor (eligiendo color) y análisis (`go infinite`).
- Nivel del motor mediante profundidad o tiempo por jugada (el motor no tiene opciones UCI, ver 7.6).
- Detección y aviso de fin de partida.
- Carga y exportación de FEN.

**No incluye (v1):**
- Renderizado 3D o animaciones físicas de piezas.
- Multi-PV, línea principal (PV) o varios motores a la vez. El motor actual no emite `pv` (la variante principal está fuera de alcance en la planificación).
- Juego en línea (Lichess / Chess.com).
- Editor libre de posiciones con paleta de piezas (se usa FEN).
- Ofertas de tablas, pensar en el turno rival (*ponder*), sonidos, deshacer jugadas.

---

## 7.2 Requisitos funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| **RF-65** | La GUI debe renderizar un **tablero de 8×8 casillas alternadas** en colores claro y oscuro, escalable con la ventana. | Must | El tablero mantiene relación 1:1, dibuja columnas `a`–`h` y filas `1`–`8` (minúsculas, como en UCI) y permite invertir la orientación en cualquier momento. |
| **RF-66** | La GUI debe **renderizar las piezas** a partir del estado de la partida. | Must | Se ven las 32 piezas en la posición inicial y se actualizan de inmediato tras cada jugada, incluyendo enroque, captura al paso y promoción. |
| **RF-67** | El usuario debe poder **mover con el ratón**, por clics (origen y destino) o arrastrando. | Must | Solo se puede mover en el turno del usuario y con piezas de su color. Soltar en un destino ilegal devuelve la pieza a su casilla sin efecto. Una jugada legal actualiza el tablero local y dispara el envío de `position` + `go` al motor (ver D-50), si corresponde. |
| **RF-68** | La GUI debe **resaltar los destinos legales** de la pieza seleccionada. | Should | Al seleccionar una pieza propia se marcan sus destinos legales, calculados con la biblioteca del motor (D-47), sin consultar al subproceso. |
| **RF-69** | Cuando un peón llegue a la última fila, la GUI debe **mostrar un selector modal de promoción**. | Must | La jugada no se aplica ni se envía hasta elegir Dama, Torre, Alfil o Caballo. Cancelar el selector deja la posición intacta. |
| **RF-70** | La GUI debe comunicarse con el motor mediante **UCI en un subproceso independiente**. | Must | Realiza el apretón de manos `uci` → `uciok` e `isready` → `readyok` con tiempo límite, envía `ucinewgame`, `position` y `go`, y procesa `info` y `bestmove`. Por cada `go` espera exactamente un `bestmove` y descarta el de una búsqueda ya cancelada o que ya no corresponde a la posición actual. |
| **RF-71** | La GUI debe mostrar **relojes regresivos** para ambos bandos, con incremento por jugada. | Must | Corre solo el reloj del bando que mueve. Se calcula con un reloj monotónico (`Instant`), no contando fotogramas. Se muestra con centésimas cuando quedan menos de 10 s. El incremento se suma tras completar la jugada. Los relojes se pueden desactivar (partida sin tiempo). |
| **RF-72** | La GUI debe mostrar una **barra de evaluación** a partir de `info ... score cp` / `score mate`. | Should | La puntuación del motor viene desde el punto de vista del bando que mueve, y la barra la convierte siempre a la perspectiva de las blancas. Los centipeones se acotan con una escala fija documentada (por ejemplo ±1000 cp satura la barra). `score mate N` se muestra como `M N` con el signo correcto. Antes del primer `info` la barra queda neutra. La barra se actualiza una vez por profundidad completada. |
| **RF-73** | La GUI debe **detectar y notificar el fin de partida**. | Must | Se detectan: jaque mate, ahogado, tablas por 50 movimientos, por material insuficiente y por triple repetición (todo con `rules::game_status` de la biblioteca), además de caída de bandera y rendición (que gestiona la GUI). Se muestra el resultado (`1-0`, `0-1`, `½-½`) y se bloquea el tablero. |
| **RF-74** | La GUI debe **cargar y exportar FEN**. | Could | Pegar un FEN válido (`fen::parse_fen`) coloca la posición y la envía como `position fen ...`. Un FEN inválido muestra el error sin alterar la partida. Copiar al portapapeles usa `fen::to_fen`. |
| **RF-75** | La GUI debe **resaltar la última jugada y el rey en jaque**. | Should | Origen y destino de la última jugada tienen un color propio. La casilla del rey en jaque se marca de forma distinta. |
| **RF-76** | La GUI debe mostrar la **lista de jugadas** de la partida. | Must (formato UCI) / Could (SAN) | La lista se actualiza tras cada jugada, en pares de jugadas por número de movimiento. SAN no existe en la biblioteca actual, así que habría que implementarlo en el crate de la GUI (desambiguación, `+`, `#`); no bloquea la v1. |
| **RF-77** | La GUI debe ofrecer **modos de juego**: humano contra motor (con elección de color) y análisis. | Must | En modo análisis el motor busca con `go infinite` y la barra de evaluación se actualiza continuamente; la búsqueda se reinicia con `stop` al cambiar de posición. En modo partida el motor solo busca en su turno. |
| **RF-78** | El usuario debe poder **elegir el nivel del motor**. | Should | Se configura por profundidad máxima (`go depth N`) o por tiempo por jugada (`go movetime T`). Con reloj activo se usa `wtime`/`btime`/`winc`/`binc`. |
| **RF-79** | La GUI debe ofrecer los **controles de ciclo de vida**: Nueva partida, Invertir tablero, Detener cálculo y Rendirse. | Must | *Nueva partida* envía `stop` (si hay búsqueda) y `ucinewgame`. *Detener* envía `stop`; la GUI espera el `bestmove` resultante y decide si lo aplica o lo descarta (lo descarta en análisis). *Rendirse* termina la partida con derrota del usuario. |

---

## 7.3 Requisitos no funcionales

| ID | Requisito | Prioridad | Criterio de aceptación |
|---|---|---|---|
| **RNF-27** | **E/S sin bloqueos:** el hilo de interfaz nunca debe esperar a la E/S del motor. | Must | Con `go infinite` en curso, ningún fotograma supera ~16 ms de procesamiento propio de la GUI y la ventana responde a clics y arrastres sin retraso perceptible. |
| **RNF-28** | **Tolerancia a fallos del subproceso:** si el motor termina, cierra su salida o escribe en `stderr`, la GUI no debe entrar en pánico. | Must | La GUI detecta fin de proceso o EOF en `stdout`, muestra aviso de desconexión y permite reiniciar el motor. El `stderr` se **consume en un hilo** (o se redirige a un registro) para que el motor nunca se bloquee por una tubería llena. Los mensajes de `stderr` (por ejemplo `position error`) se registran y se muestran como aviso, pero no se tratan como fatales. `bestmove 0000` se maneja como "sin jugada". |
| **RNF-29** | **Reposo eficiente:** la GUI debe usar un modo reactivo cuando no haya nada que actualizar. | Should | Con tablero estático, sin reloj corriendo y sin búsqueda activa, el consumo de CPU **de la GUI** es inferior al 2 %. Mientras corre un reloj o llegan `info`, se repinta solo lo necesario (`request_repaint_after`). |
| **RNF-30** | **Multiplataforma:** compilar y ejecutarse en Linux y Windows (macOS opcional). | Should | `cargo build` genera el binario sin instalación manual en Windows. En Linux se documentan las bibliotecas del sistema que requiere el *backend* gráfico (X11/Wayland, OpenGL, `libxkbcommon`, etc.). |
| **RNF-31** | **Aislamiento de dependencias:** añadir la GUI no debe introducir dependencias de ejecución en el motor. | Must | `cargo build -p chess-game` sigue sin dependencias de ejecución (coherente con `docs/DEPENDENCIAS.md`). Las dependencias gráficas viven solo en el crate de la GUI y quedan inventariadas con licencia. |
| **RNF-32** | **Lógica de partida verificable sin ventana:** el estado de la partida (turno, historial, estado final, relojes) debe estar separado del dibujo. | Should | Existen pruebas automatizadas de esa lógica sin abrir ventana, y una prueba de integración que lanza el binario del motor, completa el apretón de manos y recibe un `bestmove` válido. |

---

## 7.4 Decisiones de diseño

| ID | Decisión | Justificación | Alternativa descartada |
|---|---|---|---|
| **D-43** | **Framework: `egui` con `eframe`.** | Modo inmediato, ligero y con poca fricción; permite dibujar casillas, halos y texto directamente. | `iced` (arquitectura Elm rígida para un tablero dinámico) o GTK/Qt (rompen la portabilidad y complican el build). |
| **D-44** | **Subproceso desacoplado por tuberías UCI.** | Cumple el estándar, permite probar la GUI con otros motores y el motor con otras GUIs (pendiente T6.12 de Cute Chess). | Enlazar el motor como biblioteca dentro de la GUI para jugar: acopla el código y oculta errores del protocolo. |
| **D-45** | **Hilo lector + `std::sync::mpsc` hacia la GUI, con eventos tipados** (`Info`, `BestMove`, `Stderr`, `Disconnected`). El hilo lector llama a `ctx.request_repaint()` al enviar un evento. | Sin `request_repaint`, el modo reactivo (RNF-29) no se despertaría al llegar datos. | Leer `stdout` en cada fotograma: bloquearía la ventana. |
| **D-46** | **Piezas como imágenes (PNG o SVG rasterizado) embebidas con `include_bytes!` y cargadas como texturas una sola vez.** Debe verificarse y documentarse la licencia del conjunto de piezas. | Aspecto igual en todos los sistemas. Las fuentes con símbolos de ajedrez Unicode no están garantizadas en la fuente por defecto de `egui`. | Fuentes Unicode (dependen de la fuente disponible); dibujar cada pieza por código (difícil de mantener). |
| **D-47** | **La GUI usa la biblioteca `chess_game` como autoridad del estado de la partida:** `Board`, `PositionState`, `PositionHistory`, `generate_legal`, `rules::game_status`, `fen`. | Evita latencia y duplicar reglas. El motor solo recibe posiciones y busca. | Preguntar al motor si una jugada es legal: añade latencia y protocolo innecesario. |
| **D-48** | **Workspace de Cargo con el motor intacto y un crate `gui` que depende de él por ruta.** | Mantiene el motor sin dependencias de ejecución (RNF-31) y deja la GUI como binario aparte. | Añadir `eframe` al `Cargo.toml` del motor: arrastraría cientos de dependencias a un crate que hoy no tiene ninguna. |
| **D-49** | **La GUI es la autoridad del reloj.** Usa `Instant`, descuenta la latencia observada del motor y envía `wtime`/`btime`/`winc`/`binc` en cada `go`. | El motor ya aplica su propio margen de seguridad (50 ms) y límites blando/duro; si la GUI es la autoridad se evita el desfase. | Reloj calculado en el motor o por fotogramas. |
| **D-50** | **Sincronización de posición:** tras cada jugada la GUI envía `position startpos moves <lista>` (o `position fen <base> moves <lista>`) y luego `go`. No existe un comando "jugar esta jugada". | Es el único modo en que el motor recibe jugadas y reconstruye el historial de repeticiones. | Enviar solo la última jugada: el motor no la admite y perdería el historial. |

---

## 7.5 Riesgos técnicos y mitigaciones

| Riesgo | Impacto | Mitigación |
|---|---|---|
| **`bestmove` obsoleto:** `stop`, `go` y `ucinewgame` hacen que el motor termine la búsqueda en curso y emita su `bestmove`; si la GUI ya cambió de posición, aplicaría una jugada ajena. | Alto: jugadas ilegales o aplicadas a la posición equivocada. | Llevar un identificador de búsqueda y un contador de `bestmove` pendientes; validar toda jugada recibida con `uci::parse_uci_move` contra la posición actual; descartar las obsoletas. |
| **Errores de `position` silenciosos:** el motor responde por `stderr` y conserva la posición anterior (incluye el desborde del historial fijo de 512). | Alto: la GUI y el motor quedan desincronizados sin aviso. | Tras cada `position`, enviar `isready` y vigilar `stderr`; ante un error, avisar y reiniciar la sincronización con `ucinewgame` + `position`. |
| **Tubería de `stderr` sin consumir** | Alto: el motor puede bloquearse al escribir. | Hilo dedicado a `stderr` o redirección a archivo (RNF-28). |
| **Promoción incompleta** | Alto: la jugada UCI sería inválida. | Selector modal obligatorio (RF-69); el sufijo `q`/`r`/`b`/`n` se obtiene de la jugada legal elegida, no de texto armado a mano. |
| **Reloj y repintado reactivo en conflicto** (RNF-29 frente a RF-71) | Medio: reloj congelado en reposo, o CPU alta. | Pedir repintado periódico solo mientras un reloj corre; en pausa o con tablero estático no se repinta. |
| **Pérdida de tiempo por latencia** | Medio: caída de bandera en partidas rápidas. | Margen de seguridad en la GUI, `stop` local si el motor no responde antes del tiempo límite y descarte de su `bestmove` tardío. |
| **Cierre con el motor calculando** | Bajo: procesos huérfanos. | Enviar `stop` y `quit` en `eframe::App::on_exit` y en `Drop` del gestor del motor; esperar un tiempo límite y entonces `kill()`. |
| **Aumento de dependencias y del tamaño del build** | Medio: contradice la política de dependencias mínimas. | Workspace (D-48) e inventario de licencias en `DEPENDENCIAS.md` antes de fijar versiones. |

*Nota:* el riesgo de "saturación del canal por líneas `info`" del borrador original se retira. El motor emite un `info` por profundidad completada, no por nodo, por lo que el volumen es bajo. Se deja un límite de tamaño del canal solo como defensa.

---

## 7.6 Preguntas abiertas para el grupo

1. **Alcance:** la planificación excluía la interfaz gráfica propia. ¿Se aprueba esta fase como ampliación y con qué hito (M8)?
2. **Variante principal:** ¿se amplía el motor para emitir `pv` (y quizá `nps`) en `info`, o se acepta que la GUI muestre solo evaluación y profundidad?
3. **Nivel de juego:** como el motor no tiene opciones UCI, ¿el nivel se controla solo por profundidad/tiempo, o se añade `setoption` (por ejemplo `Skill`)?
4. **Notación:** ¿basta UCI en la lista de jugadas en la v1, o SAN es obligatoria? Implementarla en la GUI supone trabajo propio.
5. **Piezas:** ¿qué conjunto de imágenes se usará y con qué licencia, dado que aún no se define la licencia del proyecto (ver `DEPENDENCIAS.md`)?
6. **Estructura del repositorio:** ¿se convierte el repositorio en un workspace o la GUI vive en un repositorio aparte?
7. **Deshacer y tablas:** ¿se admiten deshacer jugada u ofertas de tablas en la v1, o se dejan fuera como está?
8. **Validación externa:** la tarea T6.12 (Cute Chess) sigue pendiente. ¿Se exige completarla antes de empezar la GUI?
