# Sesión 22, cierre — 27 de octubre de 2026: la base nueva en la carpeta de datos y el cortador de los PDF compuestos

Para retomar: este documento, `CLAUDE.md` (sobre todo «Guardrails de consumo de Claude») y `pendientes.json`. El traspaso anterior es [2026-10-27-sesion-22.md](2026-10-27-sesion-22.md), el de la mañana de la misma sesión.

**Datos reales:** ninguno en el repositorio. Lo de la corrida está en `C:\Users\Usuario\LocalRepo\llm_wiki_mi\desatendido-13\`; lo de la importación, en `llm_wiki_mi\integracion-78\`.

## 1. En una frase

Con la app cerrada, la base nueva de la carpeta de datos recibió todo lo de la vieja. Después, una corrida desatendida le agregó a la app el cortador de los PDF compuestos: cada documento incluido es un documento propio, con su tira de páginas para revisar el corte y sus copias y versiones. También dejó contado cuánto va a pedir a Codex llenar la base.

## 2. Estado al cerrar

| Qué | Estado |
|---|---|
| Pedidos #77 y #78 | Integrados por el dueño. |
| Pedido de esta corrida (rama `sesion-22/cortador`) | Abierto. Con esta rama, la app pasa la base de la versión 3 a la 5 y corta los compuestos con las reglas, sin preguntar. |
| `llm_wiki_wt` | En `main` (5f74213), con los modelos de RapidOCR en `src-tauri/ocr/`. Desde ahí corre la app. |
| La base de la app | `docbase/base.sqlite3`, versión 3: 2 orígenes, 139 archivos, 134 versiones, las 1.718 respuestas guardadas y las etiquetas. **Nada leído todavía**: con la lectura automática apagada, la app no pregunta, y la caché solo responde cuando pregunta. La vieja (`private.sqlite3`) no se tocó. Respaldos con sha256 en `integracion-78\`. |
| Pendientes | 222 abiertos: 124 por hacer, 68 a verificar, 25 en espera, 5 en curso. De Claude: 10, 56 y 59 (prioridades 1, 2 y 3). |

## 3. Las decisiones del dueño (27 oct)

- **La base nueva se llena una sola vez, después del cortador** (opción a). Preguntar ahora sobre los compuestos enteros se tiraría al cortarlos.
- **La segunda corrida, desatendida y de a un sub-agente,** con Codex solo para los cortes dudosos.

## 4. Lo construido (`desatendido-13`)

| Paso | Qué | Fichas |
|---|---|---|
| 1a | El cortador: reglas primero y Codex en los pasos dudosos, con las dos páginas como imágenes y la pregunta según D-527. Lo que dice en cada paso queda en `cut_step` (versión 4 de la base) | 245.992 |
| 1b | Cada parte de un compuesto es una versión con su texto y su aparición por páginas (versión 5). El compuesto no se lee entero. Las correcciones de una persona (partir, juntar, confirmar) vuelven en cada reproceso | 233.899 |
| 2 | La tira de páginas en el detalle de un compuesto. Cada incluido se abre como documento, también desde la búsqueda | 157.352 |
| 3 | Copias y versiones entre documentos incluidos; las de un corte sin confirmar esperan. La hoja traducida, en `conjunto-prueba\respuestas-v2.json` | 202.345 |
| 4 | La cuenta de lo que pediría el reproceso, sin preguntar; la lista de pasos (`herramientas\reproceso.md`) y dos guiones (`respaldar-base.py`, `sumar-respuestas.py`) | 158.373 |

**Medido** (bases armadas desde cero en carpetas temporales, sobre la carpeta de prueba):

- **Cortes, contra la hoja de partes:** 96 de 96, 3 de más, 0 perdidos y 97 de 104 documentos exactos, con 47 consultas. El prototipo daba 89, 4, 7 y 83. Por partes: 98 de 102 incluidos con las páginas exactas; un PDF de un solo documento queda cortado de más.
- **Toda la carpeta:** de 132 versiones a 331, con 29 compuestos y 199 incluidos. De los 29, solo 6 están en la hoja: los otros 23 no tienen con qué medirse.
- **Copias y versiones, contra la hoja traducida:** con los cortes confirmados, 51 aciertos, 4 errados y 2 faltantes de 57; tal como quedan, 48, 3 y 6. Antes, sin cortes: 52, 3 y 2. Hay un error nuevo (P-408).
- **El reproceso pediría unas 960 consultas a Codex:** 442 contadas y unas 520 estimadas, porque la identidad, la síntesis y los asuntos esperan a otras tareas. Son unas 2 horas o más. Si los escaneos siguen con Codex, unas 1.000.
- **Codex en la corrida:** 127 consultas, ninguna fallida. Sus respuestas están en `desatendido-13\semillas\cortes.json`; `sumar-respuestas.py` las suma a la base de la app para no pagarlas otra vez.

**La suite completa,** después del último paso: Rust 874 bien, simulaciones 1.184 de 1.184, tipos sin errores y la prueba de casos reales bien.

## 5. Lo que espera al dueño

1. **Integrar el pedido de esta corrida.** Antes de abrir la app con ese código, respaldar: `herramientas\respaldar-base.py` (pasos 1 y 2 de `reproceso.md`).
2. **Ver en la app** (pendientes «a verificar»):
   - la base en la versión 5 (P-392 y P-397);
   - la tira de páginas de un compuesto (P-403);
   - las copias y versiones que esperan al corte (P-407).
   Confirmar o corregir un corte cambia la base.
3. **Decidir:**
   - en qué PDF pregunta el cortador (P-393 y P-400). En los PDF que no juntan documentos quedan 181 pasos dudosos sin preguntar;
   - los 4 pares de la hoja en `desatendido-13\paso3-hoja-dudas.md` (P-409);
   - las decisiones asumidas D-581 a D-602.
4. **El reproceso,** con la lista de `reproceso.md` y su permiso para encender la lectura automática. Se recomienda pasar «Reconocer escaneos» al motor de esta computadora antes de encenderla (D-601).
5. Siguen de antes: aprobar el mensaje 005 a Core (P-371), revisar D-561 a D-580 y borrar las ramas viejas de la nube (P-367).

## 6. Lo que sigue para el agente local

- El análisis «como conjunto» del compuesto, armado con lo de sus incluidos (P-398).
- Lo que es del archivo en el detalle de un incluido: el chat, las etiquetas, «a quién nombra» y los parecidos (P-404).
- El ejemplar firmado que sale como «versión» (P-408).
- El guion que compara las bases (P-412).
- Los guiones de `herramientas` que todavía apuntan a la base vieja (P-413).
- El corte que no se ve en «En curso» (P-394) y su lentitud (P-395).
- La comparación después del reproceso (P-370).

## 7. Consumo

- Sub-agentes: **997.961 fichas** en cinco pasos, contra 1,0 M. Agente principal, estimado: unas 200 mil, con la importación de la mañana. Total, unas 1,2 M, en el tope.
- El paso 1a se pasó (246 mil contra 200 mil); los demás quedaron en su tope o debajo. Presupuestar como medianos los pasos que miden toda la carpeta funcionó.
