# Sesión 19 — del 23 al 25 de octubre de 2026: cierre y traspaso

Para retomar: este documento, `CLAUDE.md` (sobre todo «Guardrails de consumo de Claude») y `pendientes.json`. El detalle de cada parte está en los traspasos parciales:

- [2026-10-24-sesion-19.md](2026-10-24-sesion-19.md): primera tanda de las observaciones, firmas digitales y «Copias y versiones»;
- [2026-10-25-sesion-19c.md](2026-10-25-sesion-19c.md): la hoja de partes controlada, la ficha de forma, «Atrás» y el corte de documentos.

**Datos reales:** todo lo que tiene nombres, números o textos reales está fuera del repositorio, que es público. Está en `C:\Users\Usuario\LocalRepo\llm_wiki_mi\` (en la PC del dueño) y en la carpeta de Drive de pruebas (sección 7).

## 1. En una frase

En esta sesión se atacaron las observaciones del dueño sobre la app:
- casos y expedientes;
- personas y organizaciones mal armadas;
- nombres de personas;
- volver atrás;
- firmas digitales;
- «Copias y versiones»;
- la forma de cada página.

Además, se investigó cómo partir los PDF que juntan varios documentos.

Al final, el dueño fijó una **dirección para replantear el proceso**: primero la base, con el documento como unidad sin importar dónde esté, y después las relaciones; los contenedores con su nivel de certeza. También fijó **dos entornos de trabajo**: las pruebas en la nube y la continuidad en la PC.

## 2. Estado al cerrar (25 oct, tarde)

| Qué | Estado |
|---|---|
| `main` | Tiene integrados el #69 (las dos primeras partes) y el #70 (la prueba de Docling). |
| Rama `feat/night-2026-10-25`, pedido de integración **#71** | **Abierto.** Trae todo lo posterior al #69: la noche del 25, la madrugada del corte, el criterio nuevo, las notas del replanteo, los guardrails en `CLAUDE.md`, el prompt de la prueba en la nube y este traspaso. Lo integra el dueño. |
| `llm_wiki_wt` | Desde ahí corre la app del dueño, en el código de la rama. **El dueño la abrió el 25 a la tarde para bajar un caso nuevo del PJN.** |
| La base real | Versión 29. **La lectura automática está apagada** desde el 25: el dueño quiere bajar el caso sin procesarlo con Codex hasta afinar las reglas. Se enciende desde «Orígenes», panel «Lectura automática», o con `herramientas\lectura-automatica.py 1` con la app cerrada. |
| Respaldos | Con huellas sha256: `llm_wiki_mi\desatendido-9\respaldo-antes-v28` y `llm_wiki_mi\desatendido-11\respaldo-antes-de-abrir-25oct` (antes de que el dueño abriera la app). |
| Pendientes | 182 abiertos: 2, 43 y 50 de Claude (prioridades 1, 2 y 3); 57 para ver en la app; 24 en espera; 6 compartidos o del dueño. |

## 3. Lo logrado

**En la app** (construido, «a verificar» por el dueño):
- **Casos y expedientes (D-514):**
  - el expediente es un dato del caso;
  - al integrar un portal, la app busca primero el caso, con cerrojo;
  - si otro caso parece el mismo, pregunta en Atención;
  - el nombre que puso una persona no se toca.
- **Personas y organizaciones (D-515):**
  - se descartan sin modelo las fichas que no nombran a nadie;
  - las listas tienen el mismo filtro que las páginas;
  - un identificador único por persona.
- **Nombres y afirmaciones (D-516):**
  - el nombre de una persona es el de la mayoría de sus documentos, y se puede elegir a mano;
  - lo que apunta al propio caso se dice una vez;
  - el estado civil sale de una lista cerrada.
- **Volver atrás y adelante (D-517),** con el arreglo de los dos pasos desde Personas (D-524).
- **Firmas digitales verificadas sin conexión (D-519, D-526):** el documento dice si está íntegro.
- **«Copias y versiones»:**
  - una versión comparte el texto en las dos direcciones;
  - las familias, solo con relaciones de un mismo documento (D-520);
  - se compara sin el ruido del formato (D-521).
- **La ficha de forma de cada página (D-522, D-523):**
  - decide qué se reconoce en las lecturas nuevas;
  - lo ya leído no se relee solo: Atención cuenta las páginas escaneadas sin leer.
- **«Caso» y no «asunto»** en las páginas, y el mismo expediente una sola vez (P-313, P-330).

**En investigación y diseño:**
- **La prueba de Docling (#70):** pdfium y RapidOCR en el dispositivo; Docling no como programa.
- **Lo manuscrito no se transcribe automáticamente:** se detecta (D-518).
- **La hoja de respuestas de partes** (`llm_wiki_mi\conjunto-prueba\partes.json`):
  - seis PDF, 134 páginas, 104 documentos;
  - corregida por el dueño y controlada por una segunda revisión a ciegas;
  - con el criterio D-527.
- **El corte de documentos** (`docs/investigacion/corte/informe.md`):
  - reglas más Codex en lo dudoso;
  - con el criterio nuevo, 89 de 96 cortes;
  - en la prueba en seco sobre la base, lo «para mirar» bajó de 40 a 8. Pero la revisión acotada todavía no marca los errores reales (P-355).
- **D-527, decisión del dueño:**
  - lo que tiene emisor y fecha propios es un documento aparte, relacionado;
  - el documento compuesto se analiza también como un conjunto.
- **La propuesta de documentos compuestos** (`docs/design/documentos-compuestos-propuesta.md`, D-525, en revisión, con 10 preguntas para el dueño). Va a quedar absorbida por el replanteo del proceso.
- **Las notas del replanteo del proceso** (`docs/design/replanteo-del-proceso-notas.md`): la dirección del dueño, los contenedores y su certeza, la estructura de expedientes del PJN, la huella por página y las capas. Son notas, no un diseño.

**En la forma de trabajo:**
- **Consumo de Claude (D-513, D-528):** tope por paso y total, comunicación con los sub-agentes solo por disco, y los guardrails en `CLAUDE.md`.
- **Dos entornos (D-529).**

## 4. Lo que espera al dueño

1. **Integrar el #71.**
2. **Lanzar la prueba en la nube** con `docs/investigacion/huellas-y-embeddings/prompt.md` (P-359). Los datos ya están en la carpeta de Drive de pruebas.
3. **Bajar el caso nuevo del PJN** (en curso). Si tiene varios expedientes, hoy el conector no los recorre como tales (P-357).
4. **Ver en la app lo construido:** los pendientes «Ver en la app», en prioridad 1.
5. **Revisar la prueba en seco del corte** (`llm_wiki_mi\desatendido-11\c2-propuestas-v3.md`, P-349) y, si quiere, las 10 preguntas del diseño de compuestos (P-350). Puede esperar al replanteo.
6. **Siguen abiertos:**
   - la segunda tanda: datos del portal, plantillas sin datos de las personas, «es la marca o el grupo»;
   - el mensaje 004 a Core (borrador en `llm_wiki_mi\desatendido-9\core-004-borrador.md`);
   - la cuenta de prueba de una persona para el inicio de sesión en Core.

## 5. Lo que sigue para el agente local

1. **Mover a Google Drive la base y los archivos que se usan, para los agentes en la nube (P-358).** Es lo primero que quiere el dueño en la próxima sesión.
   - Planearlo con él: qué se copia (una copia consistente de la base, nunca la viva), cómo se mantiene al día, y la privacidad, porque son datos reales.
   - Lo que ya está en Drive se describe en la sección 7.
2. **Revisar el resultado de la prueba en la nube** cuando llegue:
   - buscar datos reales en la rama (como se hizo con Docling: los valores de las hojas, números con forma de DNI o CUIT, correos);
   - preguntarle al dueño antes de integrar.
3. **Con ese resultado, escribir el diseño del proceso replanteado** (P-356), sobre `docs/design/replanteo-del-proceso-notas.md`. Absorbe la propuesta de compuestos (P-350).
4. **Pendientes de código que no necesitan decisión:**
   - P-355: la revisión acotada que marque los errores reales;
   - P-337: la revocación con los datos de validación del propio archivo;
   - P-333: el caso del portal que dice «creado a mano» y la carátula, que pide una página real del portal;
   - P-328: las siglas de normas tomadas como organizaciones.

## 6. Cómo trabajar (lo aprendido en la sesión)

- **Guardrails de consumo:** en `CLAUDE.md`. Medido: las corridas sin tope usaron entre 4,3 y 5,5 millones de fichas; con tope, entre 0,4 y 1 millón. Lo más caro es mirar imágenes, y los pasos se pasan de su tope entre un 20 y un 70 % cuando miran páginas.
- **Plantilla de sub-agentes:** `docs/plantillas/sub-agente-comun.md`. Las corridas, con su plan, especificaciones e informes, están en `llm_wiki_mi\desatendido-9\`, `-10\` y `-11\`.
- **Guiones del agente** (en la PC), en `llm_wiki_mi\herramientas\`, con su `LEEME.md`:
  - consultas a una copia de la base;
  - texto de los PDF;
  - la lectura automática;
  - `pend.py` y `dec.py`, para los registros. La versión que completa sola la fecha de las decisiones está en `desatendido-11\`.
- **Revisión a ciegas:** para controlar una hoja de respuestas, una segunda revisión que no ve la primera, y un desempate con las opciones en orden al azar. En la hoja de partes encontró errores en 24 de 93 páginas.
- **Prueba en seco antes de construir:** encontró una regla que en la prueba no fallaba.
- **La prueba de que ningún caso real está en el programa** también revisa los documentos de diseño. Las palabras generales de las partidas se sumaron a sus palabras generales (`similar/check.rs`).
- **Reusar respuestas de Codex** ahorra, pero hay que volver a preguntar cuando cambia el criterio. La segunda medición del corte perdió 7 cortes, probablemente por eso.
- **Al dueño:** en castellano, simple y autocontenido; nunca códigos sueltos; verificar antes de afirmar; lo que trae de otras conversaciones es hipótesis.

## 7. Dónde está cada cosa

**En la PC del dueño:**
- `C:\Users\Usuario\LocalRepo\llm_wiki_mi\`:
  - `observaciones-del-dueno.md`: las nueve observaciones, con su estado;
  - `conjunto-prueba\`: las hojas de respuestas y `armar_partes.py`;
  - `prueba-docling\`;
  - `desatendido-9\`, `-10\` y `-11\`: corridas, informes y respaldos;
  - `herramientas\`.

**En Google Drive, para los agentes en la nube:**
- `Mi unidad\@Temporal - Se puede borrar\Desarrollo Micelya\POC-Micelya\`: copia de `C:\Documentos temporales para borrar\POC-Micelya`.
- Su subcarpeta `pruebas\`, con un `LEEME.md`:
  - las hojas de respuestas;
  - los seis PDF de prueba;
  - el expediente bajado del PJN;
  - lo que sabe la app, exportado a CSV;
  - investigaciones previas.

**Reglas de solo lectura:** `C:\Documentos temporales para borrar\POC-Micelya` y todo `G:\` siguen siendo de solo lectura. La única excepción es la carpeta `pruebas\` de Drive, que el dueño pidió armar.
