# Replanteo del proceso: la dirección del dueño y notas para el diseño (25 oct 2026)

**Qué es este documento.** Son notas de la conversación del 25 oct, **no definiciones ni el diseño del proceso**.
- El dueño expresó una dirección y algunas ideas para replantear cómo la app procesa los archivos.
- El diseño se escribe después, sobre estas notas y sobre lo que mida la prueba en la nube (`docs/investigacion/huellas-y-embeddings/prompt.md`).
- Lo que propone el agente está marcado como propuesta.

Sirve para cualquier agente, local o en la nube.

## 1. Por qué replantear

**Un ejemplo concreto.** El abogado de un caso tiene en su carpeta:
- los documentos sueltos: partidas, poder, DNI;
- el PDF compuesto que armó para subir al portal judicial, o que bajó del portal.

Hoy la base reconoce dos archivos como el mismo solo si son idénticos byte por byte, por la huella del archivo entero. El compuesto y los documentos sueltos que contiene no quedan unidos.

«Copias y versiones» puede proponer «uno dentro de otro» comparando el texto, pero solo si hay texto de los dos lados: con escaneos que esperan reconocimiento, no hay nada que comparar.

**Lo que se arrastra después:**
- páginas de resumen duplicadas;
- personas leídas dos veces;
- relaciones que no se ven.

## 2. La dirección (dueño, 25 oct)

1. **Primero la base, después las relaciones.** Asegurar, en este orden:
   - qué es cada documento, con certeza, **esté dentro de un documento compuesto o suelto;**
   - su texto, extraído según su forma: prosa, formulario, escaneo;
   - los duplicados;
   - el tipo de documento;
   - las personas y organizaciones, sin duplicar.

   Recién sobre esa base se afinan las reglas que descubren vínculos.
2. **Los documentos compuestos son contenedores,** con la lista de los documentos que contienen. Con eso:
   - **(a)** un documento que está dentro de un compuesto y además suelto en una carpeta tiene **una sola página de resumen;**
   - **(b)** el resumen del documento compuesto **no es** la suma de los resúmenes de lo que contiene: habla de **las relaciones que crea y de por qué existe;**
   - **(c)** el compuesto crea conocimiento por sí mismo. Que una certificación aparezca justo después de la partida que certifica, dentro de un compuesto, da **más certeza** que encontrar esos dos documentos en carpetas distintas.
3. **Hay varias clases de contenedor, cada una con su nivel de certeza.** Esto introduce dos conceptos nuevos: el **contenedor** y el **nivel de certeza** que aporta.

   | Contenedor | Qué dice de lo que contiene | Certeza que aporta |
   |---|---|---|
   | Documento compuesto (un PDF que junta varios) | Una persona los eligió y los juntó para algo | Alta |
   | Expediente de un portal judicial | El portal dice que esas actuaciones son de ese expediente, con su tipo, fecha y fojas | Alta |
   | Carpeta del sistema de archivos | Alguien los guardó juntos | Menor, pero sigue siendo un contenedor |
   | Propuesta del agente, a validar: un correo con adjuntos | El remitente los mandó juntos | Alta |

   Un mismo documento puede estar en varios contenedores a la vez. Por ejemplo, una partida suelta en la carpeta del abogado, dentro del PDF de documentación adjunta, y en el expediente del portal.
4. **Lo manuscrito no se transcribe** (D-518), y **lo que tiene emisor y fecha propios es un documento aparte, relacionado** (D-527).

## 3. La estructura del portal del PJN (dueño, 25 oct)

- **Lo que sabíamos:** un caso tiene **actuaciones**, y cada una es un despacho o escrito, una notificación o información, con su oficina, fecha, tipo y fojas.
- **Lo nuevo:** en casos más complejos hay **un nivel más arriba, el expediente.** Un caso puede tener varios expedientes, y cada expediente tiene sus despachos.
- **Uno de los casos que el dueño quiere bajar tiene esta estructura.**
- **Qué hay que ajustar en la integración con el PJN:**
  - **El modelo de datos ya admite varios expedientes por caso** (D-514). Falta que el conector del portal (`src-tauri/src/portals/pjn.rs`) los recorra y los traiga como tales, cada uno con sus actuaciones.
  - Relacionado: traer también las pestañas «Intervinientes», «Vinculados» y «Recursos» (P-301), y usar los datos de cada actuación (P-323).
  - El vocabulario sigue D-512: palabras canónicas adentro y las del PJN solo en la pantalla de la vertical jurídica.

## 4. Propuesta del agente, para discutir

**La unidad de trabajo pasa del archivo al documento,** y la página es la base para reconocerlo.
- **La huella de cada página:**
  - la huella exacta: cuando el compuesto se armó juntando PDF, las páginas suelen ser idénticas por dentro;
  - la huella de la imagen de la página: reconoce la misma página aunque se haya vuelto a guardar o comprimir;
  - la huella del texto, como respaldo.

  Así, la partida suelta y la partida dentro del compuesto son **un documento con dos apariciones.** La base ya hace eso con un archivo copiado en dos carpetas, pero solo a nivel de archivo entero.
- **Las capas del proceso.** Cada una se apoya en la anterior y se mide contra una hoja de respuestas:
  1. El archivo: suelto o contenedor.
  2. La página: su huella, su forma (la ficha de forma, D-522), su texto según su forma, y si tiene manuscrito o firmas, que se detectan y no se transcriben.
  3. El documento: dónde empieza y termina, sus apariciones y sus contenedores.
  4. Qué es: tipo, fecha, emisor; duplicados y versiones.
  5. Personas y organizaciones, sin duplicar.
  6. Relaciones entre documentos, entre personas, y entre personas y bienes u organizaciones. Cada una con su nivel de certeza, según de dónde sale: el contenedor, los datos del portal, una mención en el texto o un parecido.
  7. Casos.
- **Lo que ya existe, por capa:**

  | Capa | Lo que existe |
  |---|---|
  | 2 | La ficha de forma (D-522, D-523) |
  | 4 | Las firmas digitales (D-519); «Copias y versiones» (D-497, D-520, D-521) |
  | 5 | Los controles de personas (D-515); los nombres (D-516) |
  | 7 | Casos y expedientes (D-514) |
  | Investigación | El corte de documentos (`docs/investigacion/corte/informe.md`) y la propuesta de documentos compuestos (`docs/design/documentos-compuestos-propuesta.md`, D-525) |

- **Lo que falta:**
  - la huella de cada página;
  - el documento como unidad, con sus apariciones;
  - los contenedores y su nivel de certeza;
  - la pasada sobre cada contenedor;
  - leer los formularios por posición;
  - un lector local para escaneos (RapidOCR);
  - la detección de manuscrito y firmas.

## 5. Dos entornos de trabajo (dueño, 25 oct)

- **Las pruebas las hace un agente en la nube.** Solo tiene acceso a la carpeta de Google Drive `Mi unidad\@Temporal - Se puede borrar\Desarrollo Micelya\POC-Micelya`, que es una copia de la carpeta de prueba de la PC. Lo que necesitan las pruebas está en su subcarpeta `pruebas`, descrita en su `LEEME.md`.
- **La continuidad del trabajo la hace un agente local.**
- **Próximo paso del dueño:** mover a Google Drive la base y los archivos que se usan, para que los agentes en la nube puedan probar sobre ellos.
