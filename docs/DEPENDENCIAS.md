# Decisión de dependencias para FEN

**Decisión (2026-11-10):** implementar un parser y serializador FEN propios en
`../src/fen.rs`; no se agrega una dependencia externa por ahora. La representación
`Board`/`PositionState` del proyecto es propia y todavía no está definida. Usar
otro modelo de tablero solo para FEN obligaría a convertir el estado, y podría
introducir reglas o validaciones distintas de las del motor. El parser propio
debe validar los seis campos FEN y convertirlos directamente a esos tipos.

| Alternativa evaluada | Versión | Licencia | Lectura y escritura FEN | Mantenimiento / encaje |
|---|---:|---|---|---|
| `shakmaty` | 0.30.2 | GPL-3.0-or-later | Sí | Publicación del 2026-10-07, señal de mantenimiento reciente. Su licencia copyleft no se adopta sin definir antes la licencia del proyecto. Introduciría además una representación externa que habría que adaptar. |
| `cozy-chess` | 0.3.4 | MIT | Sí | Publicación del 2024-04-05; no ofrece evidencia de mantenimiento reciente en el registro consultado. Tiene FEN y su propio tablero, además de generación de movimientos que no se necesita para esta integración. |

Las versiones, licencias y fechas se contrastaron con crates.io el 2026-11-10;
el soporte FEN se contrastó con la documentación de
[`shakmaty`](https://docs.rs/shakmaty/0.30.2/shakmaty/fen/struct.Fen.html) y
[`cozy-chess`](https://docs.rs/cozy-chess/0.3.4/cozy_chess/struct.Board.html).
No se añade ninguna de estas crates al `../Cargo.toml`; por ello no hay una
versión FEN que fijar en `../Cargo.lock`. La dependencia de FEN queda en cero
hasta que exista una necesidad concreta de reemplazar el parser propio.

## Inventario actual

`../Cargo.toml` no declara dependencias de ejecución. La única dependencia
directa es `criterion` 0.5.1, solo para benchmarks, con licencia
`Apache-2.0 OR MIT`; su versión exacta y las versiones de sus dependencias
transitivas quedan fijadas en `../Cargo.lock`. Se revisó la metadata de licencia
de todos los paquetes del lockfile; todos declaran una licencia y no se
incorporan dependencias de ejecución. No se usa código `unsafe` propio.
