# Datos de referencia: Anexo B

`../tests/data/annex-b.json` archiva las FEN, los conteos de perft, las posiciones
y las sesiones UCI del Anexo B. Los valores de perft de B.1 se contrastaron el
2026-11-10 con la tabla correspondiente de
[Chess Programming Wiki: Perft Results](https://www.chessprogramming.org/Perft_Results);
coinciden las cinco posiciones, sus FEN y los conteos publicados hasta la
profundidad 4. También se verificó allí el conteo inicial de profundidad 5.

Los casos B.2–B.4 y sus resultados esperados se copiaron del Anexo B facilitado
para este proyecto. Se conservan como especificaciones de prueba, pero todavía
no se han validado ejecutándolos: el motor no implementa aún la generación de
movimientos, evaluación, búsqueda ni UCI. No interpretar estos datos como tests
ya ejecutados.
