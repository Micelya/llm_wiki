# Huellas de página y embeddings para encontrar duplicados, contenedores y relaciones

26 de octubre de 2026. Prueba hecha en una sesión de Claude Code en la nube con el guion [`prompt.md`](prompt.md) (pendiente P-359). Los documentos y todo lo que salió de ellos quedaron fuera del repositorio. Acá solo hay números y guiones.

## En resumen

1. **La huella de la imagen de la página (pHash) es la que mejor funciona.** Encontró los 3 documentos repetidos que no son idénticos byte por byte y los 4 ejemplares firmados dentro de su original. Se equivocó en 3 de 30 pares que la hoja dice que no son el mismo documento. La huella exacta y la del texto casi no encuentran nada que no encuentre ya la huella del archivo entero.
2. **El modelo de embeddings grande (Qwen3-Embedding-0.6B) busca mucho mejor que la búsqueda por palabras:** en 30 preguntas trae el 83 % de los documentos esperados entre los 5 primeros, contra el 50 % de BM25. El modelo chico (MiniLM) busca igual que BM25.
3. **La trampa jurídica es real:** los documentos hechos con la misma plantilla quedan tan cerca como las copias. Se separan exigiendo además que coincidan los datos clave (fechas, montos y números) o la huella de imagen: así queda 1 falso de 25.
4. **La mejor combinación:** el embedding propone 5 candidatos por documento y la huella de imagen confirma. Encuentra lo mismo que comparar todo contra todo, con 17 veces menos comparaciones.
5. **Costo en una PC:** las huellas son casi gratis (5 segundos para 769 páginas). El modelo chico tarda 0,02 s por página y el grande 1,9 s por página en este procesador. El OCR llegó a usar 11,6 GB de memoria: hay que revisarlo antes de llevarlo a la app.

## Cómo se probó

- **Equipo:** procesador Xeon de 2,1 GHz con 4 núcleos, 15 GB de memoria, Linux, sin placa de video. En una PC con Windows los tiempos cambian, pero las proporciones entre herramientas deberían mantenerse (inferencia, no medido).
- **Datos:** la carpeta de prueba de Drive se bajó a disco con un enlace compartido. Su contenido no pasó por el contexto del agente.
  - Son 271 archivos y 187 PDF (los de la carpeta de la empresa, los de la carpeta del caso y los del expediente del portal judicial), sin contar las copias de `documentos-de-prueba` ni los archivos de apoyo.
  - Quedan 130 contenidos distintos y 601 páginas distintas. 90 páginas no tienen texto.
- **Copias idénticas:** los 52 PDF del expediente del portal son idénticos byte por byte a 52 PDF de la carpeta del caso. Para comparar se usó un solo representante por contenido.
- **Umbrales elegidos antes de medir, por una razón general (R-25):**

  | Umbral | Valor | Razón |
  |---|---|---|
  | Tolerancia de las huellas de imagen | distancia ≤ 6 de 64 bits | un valor habitual para dHash y pHash |
  | Página común | aparece en 5 contenidos distintos o más | hojas en blanco y carátulas no prueban nada; esas páginas se ignoran |
  | Vecinos | 5 | |
  | Datos clave | Jaccard ≥ 0,8 | el conjunto de palabras que tienen dígitos |
  | «Casi iguales» para un embedding | coseno ≥ 0,95 | |

  - Las tolerancias 3 y 10 se midieron solo para ver qué tan sensible es el resultado; no se eligió ninguna con eso (ver `metricas.json`).
  - **Una corrección:** en un primer cálculo, el umbral de la trampa salió del par «mismo documento» menos parecido de la hoja. Eso es aprender de la hoja, así que se reemplazó por el 0,95 fijo.
- **Versiones:** pypdfium2 5.14.0, pypdf 6.21.2, ImageHash 4.3.2, rank-bm25 0.2.2, fastembed 0.9.1, onnxruntime 1.31.0 y rapidocr-onnxruntime 1.2.3.
  - El OCR anterior (`../docling/informe.md`) usó RapidOCR 3.10.0. Este es el paquete viejo, con sus propios modelos.

## Parte A · Huellas de página

- **Exacta:** el flujo de contenido de la página más sus imágenes.
- **dHash y pHash:** la página dibujada a 256 px de alto, en grises.
- **Texto:** el texto normalizado; solo cuenta si tiene 40 caracteres o más.

| Medida | Exacta | dHash | pHash | Texto |
|---|---|---|---|---|
| Páginas ignoradas por comunes | 18 | 85 | 51 | 18 |
| Pares de PDF distintos que resultan «el mismo» | 2 | 11 | 22 | 3 |
| Pares en que un PDF está «dentro» de otro | 4 | 10 | 14 | 3 |
| Pares entre carpetas de clientes distintos (seguro falsos) | 0 | 2 | 0 | 0 |
| «Mismo documento» de la hoja, sin ser idénticos (3 medibles) | 0 | 1 | **3** | 1 |
| «Ejemplar firmado» de la hoja (4): el original queda dentro del firmado | 0 | 3 | **4** | 0 (3 parciales) |
| Falsos de la hoja (misma plantilla, sin relación, etc.) | 0 | 3 | 3 | 0 |
| Documentos incluidos en los 6 PDF de `partes.json` (104) con una aparición suelta | 0 | 2 | 5 | 0 |
| …de esos, con el corte exacto | 0 | 1 | 1 | 0 |

**La carpeta del caso frente al expediente del portal.** Todo lo que comparten son copias idénticas: los 52 PDF del portal están byte por byte en la carpeta del caso. Ninguna huella encontró, además, un documento de una carpeta dentro de un PDF de la otra. Dentro de la carpeta del caso, la huella exacta encontró 4 documentos sueltos dentro de otro PDF, y pHash 5.

**Qué huella falla, y cuándo:**
- **La exacta** falla en cuanto el PDF se vuelve a guardar: no encontró ninguno de los 3 «mismo documento» que no son idénticos. Esos 3 tienen el mismo texto (coseno 1,0), pero otros bytes. Solo sirve cuando el compuesto se armó pegando PDF sin rehacerlos.
- **La del texto** falla en las páginas escaneadas sin texto (90 de 601). También falla cuando un sello o una firma agregan texto: en los ejemplares firmados solo coincidió una parte de las páginas.
- **pHash** aguanta que el PDF se vuelva a guardar y que se agregue una página de firma. Falla en las páginas casi iguales de una misma plantilla: 2 falsos de «misma plantilla» y 1 de «sin relación».
- **dHash** es peor que pHash en todo, y es el único que confundió páginas de clientes distintos.
- **Un documento vuelto a escanear:** la hoja no tiene ningún par así entre PDF, así que no se pudo medir.

**Sobre los contenedores.** Para muy pocos de los 104 documentos incluidos apareció un ejemplar suelto: 5 con pHash, y 3 con un embedding casi igual. `partes.json` no dice cuáles tienen de verdad una copia suelta. Por eso esto no prueba que la huella falle: puede ser que en estas carpetas casi no haya copias sueltas. Para medirlo hace falta una hoja que lo diga (ver «Lo que no se pudo probar»).

## Parte B · Embeddings en el procesador

- **Texto:** el de pypdfium2. En las páginas sin texto, el de RapidOCR: 78 de las 90 salieron con texto.
- **Recorte:** como máximo 2.000 caracteres por página. MiniLM además recorta solo a 128 piezas de texto.
- **Vector del documento:** el promedio de los vectores de sus páginas. Un documento incluido en un compuesto usa el promedio de sus propias páginas.

| Medida | MiniLM-L12 multilingüe | Qwen3-Embedding-0.6B |
|---|---|---|
| **Copias y versiones:** «mismo documento» con el par entre los 5 vecinos | 3 de 3 | 3 de 3 |
| «Ejemplar firmado» con el par entre los 5 vecinos | 4 de 4 | 4 de 4 |
| «Misma plantilla» con el par entre los 5 vecinos (la trampa) | 15 de 25 | 17 de 25 |
| «Misma plantilla» con coseno ≥ 0,95 | 16 de 25 | 6 de 25 |
| «Ejemplar firmado» con coseno ≥ 0,95 | 1 de 4 | 3 de 4 |
| Vecinos de los 46 documentos de la hoja (230 en total) | 12 pares buenos, 25 misma plantilla, 7 sin relación, 2 acuses, 1 constancia; 183 sin etiqueta en la hoja | 12 buenos, 30 misma plantilla, 7 sin relación, 2 acuses, 3 constancias, 2 respuestas; 174 sin etiqueta |
| **Regla contra la trampa:** coseno ≥ 0,95 y (los mismos datos clave o pHash dice «mismo» o «dentro») | mismo documento 3/3, firmado 1/4, **plantilla 1/25** | mismo documento 3/3, firmado 3/4, **plantilla 1/25** |
| **Casos:** vecinos con algún caso en común (azar: 25 %) | 57 % | 63 % |
| **Tipos:** acierto con el vecino más cercano (el tipo más común solo: 16 %) | 72 % | 71 % |
| Acierto con el voto de 5 vecinos | 70 % | 69 % |
| **Búsqueda:** esperados entre los 5 primeros (BM25: 50 %) | 48 % | **83 %** |
| Esperados entre los 10 primeros (BM25: 62 %) | 58 % | **90 %** |
| Posición del primer esperado, MRR (BM25: 0,59) | 0,45 | **0,70** |
| Mezcla con BM25, esperados entre los 5 primeros (fusión por rangos, RRF) | 42 % | 59 % |

- **Sobre la trampa.** Que el par esté entre los vecinos no alcanza: los documentos de una misma plantilla quedan tan cerca como las copias. La mediana del coseno en «misma plantilla» es 0,98 con MiniLM y 0,93 con Qwen3.
  - Los datos clave sí los separan: ninguno de los 25 pares de misma plantilla tiene los mismos datos clave (la mediana del Jaccard es 0,46).
  - Pero los ejemplares firmados tampoco los tienen, porque la firma agrega fechas y números. Por eso la regla suma la huella de imagen, que es la que reconoce al firmado.
- **Casos y tipos se midieron contra lo que dice la app**, que no es una verdad. Los dos modelos agrupan bastante mejor que el azar, pero el tipo por vecinos acierta solo 7 de cada 10.
- **Búsqueda.** 17 de los documentos esperados no son PDF (son de Word) y quedaron afuera. La mezcla con BM25 empeoró a Qwen3 en estas preguntas.

## Combinación

Pares que cuentan como «mismo» o «dentro»:

| Método | Pares | Buenos de la hoja (7 medibles) | Falsos de la hoja | Comparaciones de páginas |
|---|---|---|---|---|
| pHash, todos contra todos | 36 | 7 | 3 | 8.385 pares de documentos |
| Embedding MiniLM (5 vecinos) + pHash | 30 | 7 | 3 | 494 |
| Embedding Qwen3 (5 vecinos) + pHash | 29 | 7 | 3 | 467 |

Proponer con el embedding y confirmar con pHash no pierde nada de la hoja y saca 6 o 7 pares sin etiqueta. Esos pares son probablemente falsos, pero no está verificado. Para corregir los 3 falsos que quedan hace falta además la regla de los datos clave (ver la parte B).

## Costo en la PC

| Paso | Tiempo | Memoria máxima | Tamaño |
|---|---|---|---|
| Huellas de 769 páginas (4 procesos) | 5 s en total | 181 MB por proceso | — |
| OCR de páginas sin texto (RapidOCR, a 200 ppp) | 3,2 s por página | **11,6 GB** | viene con el paquete |
| Embeddings MiniLM-L12 (cuantizado) | 0,019 s por página, 0,09 s por documento | 0,7 GB | 241 MB, vectores de 384 |
| Embeddings Qwen3-Embedding-0.6B (cuantizado) | 1,9 s por página, 8,9 s por documento | 4,7 GB | 1,1 GB, vectores de 1.024 |
| Paquetes de Python | — | — | 597 MB |

Los 11,6 GB del OCR son el pico de todo el proceso con las 90 páginas. Lo más probable es que salgan de páginas escaneadas muy grandes dibujadas a 200 ppp (inferencia, no medido). Antes de usarlo hay que limitar el tamaño de la imagen.

## Licencias

Verificadas en su fuente (Hugging Face y PyPI) el 26 oct 2026.

| Componente | Licencia |
|---|---|
| paraphrase-multilingual-MiniLM-L12-v2 (el modelo y su versión ONNX de Qdrant) | Apache 2.0 |
| Qwen3-Embedding-0.6B (el modelo y su versión ONNX de Qdrant) | Apache 2.0 |
| fastembed | Apache 2.0 |
| onnxruntime | MIT |
| pypdfium2 | BSD-3 y Apache 2.0 (PDFium: BSD-3) |
| pypdf | BSD-3 |
| ImageHash | BSD-2 |
| rank-bm25 | Apache 2.0 |
| rapidocr-onnxruntime | Apache 2.0 |

Todas se pueden usar en una app con licencia GPL v3: Apache 2.0, MIT y BSD son compatibles con la GPL v3.

## Recomendación

| Uso | Qué conviene | Corre en la PC |
|---|---|---|
| El mismo documento en dos lugares, idéntico | La huella del archivo entero, que la base ya tiene. Cubre todo lo que comparten la carpeta del caso y el expediente del portal | sí, gratis |
| Duplicados que se volvieron a guardar | pHash por página. La exacta no sirve | sí, casi gratis |
| Versiones y ejemplares firmados | pHash «dentro», con el embedding para proponer candidatos | sí |
| Contenedores (un suelto dentro de un compuesto) | pHash por página, y el rango de páginas que coincide marca el corte. Funcionó en los pocos casos que hay; falta una hoja que lo mida bien | sí |
| No confundir la misma plantilla con el mismo documento | Coseno alto, más los mismos datos clave o pHash | sí |
| Relaciones y casos | Embedding Qwen3 para proponer vecinos. Agrupa el 63 % frente al 25 % del azar; es una sugerencia, no una certeza | sí, en segundo plano |
| Tipos | El vecino más cercano acierta 7 de cada 10: sirve como pista, no para decidir | sí |
| Búsqueda | Embedding Qwen3, sin mezclarlo con BM25. MiniLM no alcanza | sí, en segundo plano |

**En qué orden lo construiríamos:**
1. pHash y la huella del texto por página, guardadas en la base al leer cada PDF. Es lo más barato y ya resuelve duplicados, firmados y cortes.
2. Embeddings Qwen3 por página, calculados en segundo plano: unas 1.900 páginas por hora en un procesador como este.
3. Candidatos por embedding, confirmados con pHash y con los datos clave, para «Copias y versiones» y para los contenedores.
4. La búsqueda por embeddings.
5. Antes de llevar el OCR a la app, limitar el tamaño de la imagen y volver a medir la memoria.

## Lo que no se pudo probar, y por qué

- **Un documento vuelto a escanear:** la hoja no tiene ningún par así entre PDF.
- **Si las huellas encuentran todas las copias sueltas de los documentos incluidos:** `partes.json` no dice cuáles tienen copia suelta. Hace falta agregarlo a la hoja.
- **La relación «contenido en»:** la hoja no tiene ningún par así.
- **Los documentos de Word:** el guion pide el texto de pypdfium2, así que se dejaron afuera. Son 17 esperados de las preguntas y 37 extremos de pares de la hoja.
- **Los tiempos en una PC con Windows:** solo se midió en este equipo.
- **Guardar los resultados en Drive:** subirlos con el conector haría pasar su contenido por el contexto. Quedaron en un archivo comprimido entregado aparte.

## Fichas de Claude

- **Sub-agentes:** ninguno durante la prueba. Antes de la prueba, uno contó la carpeta de Drive: 133 mil fichas.
- **Agente principal:** unas 70 mil fichas desde el arranque de la prueba, según el contador de la sesión. Ese contador probablemente no incluye la relectura del contexto ya guardado, así que el número real puede ser mayor.
- **Tope:** 400 mil. No se superó ningún tope por parte.

## Revisión de datos reales

Antes del commit se buscaron en los archivos de esta carpeta:
- nombres de las carpetas de clientes;
- números largos (de documento, CUIT o expediente);
- correos, teléfonos y rutas personales.

No se encontró ninguno. Los grupos aparecen como «empresa», «caso» y «pjn».
