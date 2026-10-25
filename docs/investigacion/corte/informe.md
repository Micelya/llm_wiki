# Cómo partir solo un PDF que junta varios documentos (investigación, 25 oct 2026)

**Pregunta:** un PDF puede juntar varios documentos (un escrito con su documentación adjunta, partidas, documentos de identidad, una declaratoria). ¿Qué tan bien se puede partir sin una persona, antes de diseñar cómo se guarda?

**Cómo se midió:** seis PDF de prueba (134 páginas, 128 pasos de una página a la siguiente, 88 documentos) contra la hoja de partes del dueño (versión 2). La hoja mide, no enseña (R-25): las reglas se escribieron antes de medir y no se ajustaron mirando páginas. Para cada paso, el cortador dice «continúa», «empieza otro» o «dudoso», y dice el tipo de cada documento que empieza.

## Resultado en pocas palabras

- **Solo reglas (variante A):** encuentran 78 % de los cortes, con 84 % de precisión. Dejan dudosos 47 de los 128 pasos. No cuestan nada (menos de 1 segundo).
- **Reglas más Codex solo en los dudosos (variante B):** encuentran 79 de los 80 cortes (99 %), con 85 % de precisión, y dejan 72 de los 88 documentos exactos. Son 47 consultas, unos 4 minutos. **Es la que conviene para la app.**
- **Codex en todos los pasos (variante C):** más precisa (96 %), pero pierde más cortes (91 %), y cuesta 128 consultas (12 minutos).
- Los cortes de más de B son casi todos **adjuntos de un documento en otro papel** (una certificación, un timbre, la hoja de firmas): se arreglan juntando, que es más fácil que encontrar un documento perdido dentro de otro.

## Las reglas (variante A, `guiones/cortar.py`)

Se aplican en este orden; la primera que vale decide.

| # | Regla | Decide | Bien | Mal |
|---|---|---|---|---|
| 1 | La página siguiente está en blanco o tiene solo el pie | continúa | 2 | 0 |
| 2 | «Página x de y»: x mayor que 1 sigue; x = 1, o la anterior era «y de y», empieza otro | las dos | 6 | 1 |
| 3 | El mismo código de documento en las dos páginas sigue; códigos distintos, otro | las dos | 0 | 0 |
| 4 | Hojas notariales con números correlativos | continúa | 0 | 0 |
| 5 | Dos fotos (menos de 100 puntos por pulgada) del mismo tamaño en píxeles y en página: las dos caras de una tarjeta | continúa | 2 | 0 |
| 6 | Una página escaneada y la otra digital | empieza otro | 3 | 0 |
| 7 | Dos escaneos con otra resolución (más de 10 %) u otro espacio de color: otra captura | empieza otro | 8 | 2 |
| 8 | Dos escaneos de la misma resolución con otra orientación, o más de 4 % de diferencia de ancho o alto: otro papel | empieza otro | 35 | 8 |
| 9 | La página anterior termina con una fórmula de cierre, o la siguiente empieza con una de apertura (`vocabulario.json`) | empieza otro | 13 | 1 |
| 10 | Ninguna de las anteriores | dudoso | — | — |

- Las señales de la página son las de la ficha de forma de la app (`page_shape.rs`, umbrales de `page-shapes.json`), más las de corte de la investigación anterior (los números de página, los códigos, las hojas notariales, los pares de fotos de un documento de identidad, el cambio de escaneo a digital) y la imagen principal de cada página (píxeles, resolución, color).
- El tipo, en la variante A, sale del vocabulario (`vocabulario.json`) buscado en el texto de la primera página. Una página escaneada sin texto no tiene tipo.
- **Las reglas 7 y 8 son las que más fallan:** el adjunto de un documento (la certificación al dorso, el comprobante de timbre) muchas veces está en otro papel o se escaneó aparte. La investigación anterior ya decía que un tamaño distinto no prueba un corte.

## Resultados por variante

Columnas: cortes reales; aciertos; de más; perdidos; precisión; alcance; documentos bien cortados (empiezan y terminan donde dice la hoja); tipo bien (entre los bien cortados); pasos dudosos.

**Variante A, solo reglas.** Sin consultas; 8 segundos para leer las señales de las 134 páginas y 0,1 segundos para decidir.

| Archivo | Cortes | Aciertos | De más | Perdidos | Precisión | Alcance | Bien cortados | Tipo bien | Dudosos |
|---|---|---|---|---|---|---|---|---|---|
| 01 | 0 | 0 | 0 | 0 | — | — | 1 de 1 | 0 | 2 |
| 02 | 12 | 11 | 6 | 1 | 65 % | 92 % | 6 de 13 | 0 | 2 |
| 03 | 5 | 4 | 0 | 1 | 100 % | 80 % | 4 de 6 | 0 | 2 |
| 04 | 63 | 47 | 6 | 16 | 89 % | 75 % | 34 de 66 | 15 | 37 |
| 05 | 0 | 0 | 0 | 0 | — | — | 1 de 1 | 1 | 3 |
| 06 | 0 | 0 | 0 | 0 | — | — | 1 de 1 | 1 | 1 |
| **Total** | **80** | **62** | **12** | **18** | **84 %** | **78 %** | **47 de 88** | **17** | **47** |

**Variante B, reglas más Codex en los 47 pasos dudosos.** 47 consultas, 256 segundos (5,4 por consulta).

| Archivo | Cortes | Aciertos | De más | Perdidos | Precisión | Alcance | Bien cortados | Tipo bien |
|---|---|---|---|---|---|---|---|---|
| 01 | 0 | 0 | 0 | 0 | — | — | 1 de 1 | 1 |
| 02 | 12 | 12 | 6 | 0 | 67 % | 100 % | 8 de 13 | 3 |
| 03 | 5 | 5 | 0 | 0 | 100 % | 100 % | 6 de 6 | 3 |
| 04 | 63 | 62 | 6 | 1 | 91 % | 98 % | 57 de 66 | 37 |
| 05 | 0 | 0 | 1 | 0 | 0 % | — | 0 de 1 | 0 |
| 06 | 0 | 0 | 1 | 0 | 0 % | — | 0 de 1 | 0 |
| **Total** | **80** | **79** | **14** | **1** | **85 %** | **99 %** | **72 de 88** | **44** |

**Variante B2, como B pero las reglas 7 y 8 también van a Codex.** 100 consultas, 545 segundos. Total: 73 aciertos, 5 de más, 7 perdidos; precisión 94 %, alcance 91 %; 69 de 88 bien cortados; 62 con el tipo bien.

**Variante C, Codex en los 128 pasos (referencia).** 128 consultas, 691 segundos.

| Archivo | Cortes | Aciertos | De más | Perdidos | Precisión | Alcance | Bien cortados | Tipo bien |
|---|---|---|---|---|---|---|---|---|
| 01 | 0 | 0 | 0 | 0 | — | — | 1 de 1 | 1 |
| 02 | 12 | 8 | 0 | 4 | 100 % | 67 % | 7 de 13 | 7 |
| 03 | 5 | 5 | 0 | 0 | 100 % | 100 % | 6 de 6 | 6 |
| 04 | 63 | 60 | 1 | 3 | 98 % | 95 % | 56 de 66 | 53 |
| 05 | 0 | 0 | 1 | 0 | 0 % | — | 0 de 1 | 0 |
| 06 | 0 | 0 | 1 | 0 | 0 % | — | 0 de 1 | 0 |
| **Total** | **80** | **73** | **3** | **7** | **96 %** | **91 %** | **70 de 88** | **67** |

- **La integridad se cumple en todas:** cada página queda en exactamente un documento.
- **Dos cortes no los puede hacer ninguna variante:** dos documentos en la misma página (arriba y abajo). Un cortador por páginas no los ve; quedan como un documento.
- **Los errores de Codex son de dos clases.** Junta un documento con el siguiente, sobre todo actas seguidas que terminan en su certificación (pierde el corte). Separa una página que es parte del documento (la constancia de firma de un servicio de firma, la segunda hoja de una boleta).
- **El tipo:** las reglas solo lo saben cuando la página tiene texto. Codex acierta el tipo en 67 de los 70 documentos bien cortados de C. En B, si se le pide el tipo de cada documento que empezó por una regla (57 consultas más, estimado con las respuestas de C), acierta 69 de 72.

## Qué conviene para la app

1. **Reglas primero y Codex solo en los dudosos (variante B).** Encuentra casi todos los cortes con la tercera parte de las consultas de C. Lo que falla son cortes de más en adjuntos, que la persona arregla juntando dos documentos en la pantalla.
2. **Las reglas de forma (7 y 8) conviene mostrarlas como «posible corte», no como corte seguro**, o pedir a Codex que confirme solo cuando el documento siguiente sería de una o dos páginas. Mejora candidata, sin medir: la regla 8 solo con escaneos de 100 puntos por pulgada o más (una foto no tiene tamaño de papel), como ya hace la regla 5.
3. **El tipo:** el que dice Codex al cortar, y para los documentos cortados por una regla, el paso de tipo que la app ya tiene para cada documento.
4. **Cuidado al llamar a Codex con imágenes:** si la ruta de una imagen no existe, `codex exec` no falla; contesta igual, sin verla (todas «continúa», tipo «other», confianza baja). Hay que comprobar que las imágenes existen y desconfiar de esa respuesta.

## Cómo repetirlo (`guiones/`)

- `senales.py <carpeta de PDF> <senales.json>`: las señales de cada página (pdfium, sin modelo).
- `cortar.py <senales.json> <variante-a.json>`: la variante A.
- `codex_corte.py <b|b2|c> <variante-a.json> <carpeta de PDF> <carpeta de trabajo> <salida.json>`: las variantes con Codex (`gpt-6-luna`, razonamiento `high`, imágenes con el lado largo de 1.000 píxeles, de a una consulta).
- `medir.py <hoja de partes> <decisiones.json>`: la tabla de una variante.

Las señales y las respuestas de Codex tienen texto de los documentos: quedan fuera del repositorio. `CORTE_PYLIB` dice dónde están pypdfium2 y Pillow.
