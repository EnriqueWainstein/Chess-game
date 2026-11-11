# chess-game

Proyecto de ajedrez en Rust. Incluye una biblioteca (`../src/lib.rs`) y un binario
(`../src/main.rs`) con protocolo UCI para usar desde una GUI compatible.

## Requisitos

- Rust y Cargo con soporte para edition 2024.

## Ejecutar

Desde la raíz del repositorio:

```sh
cargo run --release
```

El motor lee UCI por `stdin`; por ejemplo, escribí `uci`, `isready`,
`position startpos`, `go depth 4` y luego `quit`. También acepta `position fen
<FEN>`, `go movetime <ms>`, relojes UCI (`wtime`, `btime`, `winc`, `binc`,
`movestogo`), `stop`, `ucinewgame`, `d` y `go perft <profundidad>`. Los errores
se envían por `stderr`; los comandos UCI desconocidos se ignoran.

Para validar una sesión automatizada corta (la entrada cerrada también detiene
la búsqueda activa):

```sh
{ printf 'uci\nisready\nposition startpos\ngo depth 4\n'; sleep 1; printf 'quit\n'; } | cargo run --release
```

Al terminar la entrada, el motor detiene ordenadamente cualquier búsqueda activa.

## Comprobaciones locales

```sh
./scripts/check.sh
```

El script corre tests y todos los targets, Clippy con warnings como errores y
`rustfmt` en modo verificación. No se configura CI remoto por ahora.

La suite de estrés de UCI también se puede ejecutar por separado:

```sh
cargo test --test stress
```

Repite 200 ciclos de `go`/`stop`, alternando cambios de posición y reinicios, y
verifica una respuesta `bestmove` y `readyok` por ciclo.

## Benchmarks

```sh
cargo bench --bench perft
cargo bench --bench search
```

`perft` informa nodos por segundo y compara ataques deslizantes; `search` mide
la búsqueda de inicio y una posición táctica de captura de dama a profundidad
fija. Criterion guarda resultados y comparaciones en `target/criterion/`.

La línea base local registrada (bench optimizado, 2026-11-10) es de 4.865.609
nodos de perft inicial a profundidad 5 en unos 149 ms (32,6 millones de nodos/s).
Para las 64 casillas y cuatro ocupaciones fijas por casilla, el cálculo mágico
tomó unos 1,35 µs para torres y 1,33 µs para alfiles, frente a 6,08 µs y
5,95 µs por rayos, respectivamente. La búsqueda inicial a profundidad 4 visitó
43.737 nodos en unos 28,0 ms (1,56 millones de nodos/s). Son mediciones de esta
máquina, no objetivos portables.

## Estructura

- `../src/`: biblioteca, motor y binario UCI.
- `../tests/`: pruebas de integración y `../tests/data/` para referencias.
- `../benches/`: benchmarks Criterion.
- `../scripts/check.sh`: compuerta local de calidad.
- `DEPENDENCIAS.md`: decisión de FEN e inventario de dependencias.

## Convenciones

- Usar nombres idiomáticos `from()` y `to()`; no anteponer `get_` a los
  accesores.
- Documentar las APIs públicas y mantener la documentación en español.
- Mantener los datos de pruebas reproducibles junto con su fuente y procedencia.

## Arquitectura y límites de v1

- La entrada UCI se mantiene disponible mientras un hilo busca sobre copias de
  tablero, estado e historial; `stop` usa una señal atómica. Al iniciar otro
  `go`, se detiene y finaliza el anterior antes de arrancar la búsqueda nueva.
- El historial es fijo (512 hashes) y conserva el segmento desde la última
  captura o avance de peón. El hash al paso solo se incluye si existe una
  captura legal.
- La evaluación usa material y una PST por pieza. La búsqueda implementa
  negamax alfa-beta, MVV-LVA, quietud e iterative deepening; no hay tabla de
  transposición, evaluación interpolada ni opciones UCI.
- La gestión de reloj reserva 50 ms de margen; reparte el resto entre las
  jugadas estimadas y limita cada búsqueda con cotas blandas y duras.
- La compilación no usa `unsafe`. No hay dependencias de ejecución; Criterion
  solo se usa como dependencia de desarrollo para benchmarks.

M1–M5 y el código de M6 están implementados. M7 incluye estrés de `go`/`stop`,
auditoría y esta documentación. La validación de M6 contra una GUI externa
queda pendiente si Cute Chess u otra GUI no está instalada. Los conteos de perft del Anexo B están
contrastados con Chessprogramming Wiki; los demás casos se conservan como
especificaciones pendientes para las fases correspondientes.
