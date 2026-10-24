# Prueba de Docling para leer documentos en el dispositivo

24 de octubre de 2026. Prueba hecha en una sesión de Claude Code en la nube, sobre los seis PDF de prueba. Los PDF y todo lo que salió de ellos quedaron fuera del repositorio. Acá solo hay números y descripciones, sin datos reales (ver el final).

## En resumen

1. **Docling sirve, pero solo con un ajuste distinto del normal.** Con su ajuste normal conserva la capa de texto vieja y mala (documento 04) y desordena el formulario aplanado (10 de 27 pares bien). Si se le pide **leer la página entera con RapidOCR**, rehace la capa vieja (las letras sueltas bajan del 63 % al 11 %) y saca **26 de 27 pares** del formulario.
2. **El mejor lector local es RapidOCR** (licencia Apache):
   - en lo impreso escaneado erró el 0,2 % de los caracteres;
   - leyó entera la zona de lectura de los dos DNI, con todos los dígitos de control bien;
   - es el único que rescata algo de lo manuscrito (12 de 37 valores, casi siempre con algún error).
   - **Tesseract** en castellano anda bien en lo impreso, pero no lee ni la zona de los DNI ni lo manuscrito. **EasyOCR** es lento y pierde renglones.
3. **Lo que no resuelve ningún lector local:** la cursiva del acta de matrimonio antigua sale ilegible, y lo manuscrito de las partidas sale casi todo mal. Eso sigue siendo trabajo de un modelo con imagen, en Core o en otro servicio.
4. **El modelo de visión `granite-docling-258M` no sirve para esto:**
   - en el procesador tarda hasta 7 minutos por página;
   - en 3 de 11 páginas no devolvió texto;
   - en 4 entró en un bucle repitiendo una palabra;
   - no leyó nada manuscrito.
5. **Docling se puede llevar a Windows sin PyTorch** (591 MB de paquetes en lugar de 1.254 MB, 8 segundos de arranque en lugar de 13), pero hoy hace falta **cambiarle tres cosas** para que funcione, y así se pierde la detección de tablas. Funciona sin conexión una vez bajados los modelos.

## Cómo se probó

- **Equipo:** procesador Intel Xeon de 2,1 GHz con 4 núcleos, 15 GB de memoria, Linux, sin placa de video. Los tiempos sirven para comparar herramientas entre sí; en una PC con Windows cambian según el procesador.
- **Una corrida a la vez:** cada documento se procesó en un proceso aparte, sin otras pruebas en paralelo, para que los tiempos y la memoria sean comparables.
- **Versiones:**
  - Docling 2.142.1, docling-core 2.104.0, docling-parse 7.23.0 y docling-ibm-models 4.0.4;
  - PyTorch 2.14.1 para procesador, onnxruntime 1.31.0 y transformers 5.20.2;
  - RapidOCR 3.10.0, que trae adentro los modelos PP-OCRv6 «small»;
  - Tesseract 5.5.3 (dentro de tesserocr 2.12.0), con dos modelos de castellano: el «rápido» del sistema (2,2 MB, de 2019) y el «mejor» (`tessdata_best`, 13,6 MB);
  - EasyOCR 1.7.2 y pypdfium2 5.14.0;
  - Python 3.13.
- **Configuraciones de Docling probadas:**
  - con RapidOCR, Tesseract (los dos modelos) y EasyOCR;
  - cada una con su ajuste normal y con «leer la página entera» (`force_full_page_ocr`, que en esta versión se llama `OcrMode.FULL_PAGE`);
  - la elección automática de lector de Docling, que eligió RapidOCR y dio lo mismo que RapidOCR con el ajuste normal;
  - Docling sin PyTorch.
- **Cómo se midió la calidad.** Se pidió no copiar texto de los documentos, así que todo se convirtió en números:
  - **Pares del formulario 01:** una planilla con los 27 valores esperados, verificada contra la imagen. Un par está bien si el valor aparece entre su rótulo y el rótulo siguiente.
  - **Error de caracteres en lo impreso:** transcribí a mano un trozo de dos páginas (600 caracteres cada uno) y medí qué porcentaje de caracteres erró cada herramienta. No cuentan los espacios ni los guiones; sí cuentan las tildes y las mayúsculas.
  - **Manuscrito:** 37 valores escritos a mano en dos partidas, leídos a ojo. Se cuenta cuántos salen exactos y cuántos salen «casi» (85 % de parecido o más).
  - **Letras sueltas y «palabras reconocibles»:** la proporción de palabras de 3 letras o más que figuran en una lista de 200.000 palabras del castellano. Sirve como señal; con muy pocas palabras engaña, como pasa en lo manuscrito.
  - **Zona de lectura de los DNI:** si salen los tres renglones y si los cuatro dígitos de control dan bien.
- **Dónde está cada número:** en `metricas.json`, por herramienta, documento y página. Los guiones están en `guiones/`.

## Tabla de calidad

Nombres cortos usados en la tabla:
- **«D»:** Docling con PyTorch.
- **«normal»:** el ajuste normal de Docling.
- **«entera»:** leer la página entera.
- **«Tess.»:** Tesseract con el modelo «mejor» de castellano (con el «rápido» los resultados son parecidos y están en `metricas.json`).

| Qué se midió | pdfium simple | pdfium por posición | RapidOCR solo | D + RapidOCR normal | D + RapidOCR entera | D + Tess. normal | D + Tess. entera | D + EasyOCR normal | D + EasyOCR entera | D sin PyTorch, RapidOCR entera | granite-docling |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 01: pares bien (de 27) | 0 | 23 | — | 10 | **26** | 10 | 17 | 10 | 16 | **26** | 19 |
| 01: casillas bien (de 3) | no se ven | 3 | — | 3 (*) | 3 (*) | 3 (*) | 3 (*) | 3 (*) | 3 (*) | sin tabla | 1 |
| 05: tabla Concepto/Importe (2 pares) | 0 | **2** | — | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 |
| 05: tabla de infracciones (pág. 3) | sin celdas | renglones bien | — | tabla 8 × 2, bien | igual | igual | igual | igual | igual | zona sin celdas | no probada |
| 06: tabla de cuotas | sin celdas | renglones bien | — | tabla 4 × 4, bien | igual | igual | igual | igual | igual | zona sin celdas | la perdió |
| 03: páginas en blanco (3 y 9) | sí, con la regla de renglones repetidos | — | — | sí | sí | sí | sí | sí | sí | sí | no probada |
| 03: escaneos 6 a 8, palabras reconocibles | sin texto | sin texto | 80 % | 76 % | 77 % | 89 % | 66 % | 73 % | 77 % | 77 % | 88 % (solo pág. 6) |
| 04: letras sueltas en las 37 páginas ruidosas | 63 % | — | 12 % | 55 % | **11 %** | 57 % | 16 % | 56 % | **10 %** | 11 % | — |
| 04, pág. 14: error de caracteres | 3,8 % (letras bien, espacios mal) | — | 8,7 % (**) | 3,8 % (dejó la capa vieja) | **2,4 %** | 3,8 % | 3,0 % | 3,8 % | 51 % | 2,4 % | 2,1 % |
| 02, pág. 1 (impreso escaneado): error de caracteres | sin texto | sin texto | 1,5 % | **0,2 %** | **0,2 %** | 1,0 % | 1,0 % | 47 % | 31 % | 0,2 % | 99 % (no devolvió texto) |
| 02: zona de lectura de los DNI (de 2) | sin texto | sin texto | **2** | **2** | **2** | 0 | 0 | 1 | 1 | **2** | 0 (de 1 probada) |
| Manuscrito: valores casi bien (exactos), de 37 | 0 | 0 | 10 (5) | 11 (6) | **12 (6)** | 0 | 0 | 4 (3) | 2 (2) | 12 (6) | 0 |
| Cursiva del acta antigua (02, págs. 6 y 7) | sin texto | sin texto | ilegible | ilegible | ilegible | ilegible | ilegible | ilegible | ilegible | ilegible | ilegible, en bucle |
| Tildes y eñes conservadas (de 22 en los dos trozos) | 10 de 10 donde hay texto | — | 20 | 22 | 20 | 22 | 22 | 20 | 16 | 20 | 10, y en la boleta cambió «á» por «Æ» |

(*) Docling ubica bien las tres casillas dentro de la tabla, pero la marca sale como el carácter «4» en lugar de «✔», porque el lector de PDF de Docling no traduce la fuente de símbolos. pdfium la traduce bien.

(**) RapidOCR solo se corrió con la página a unos 140 puntos por pulgada; Docling la pasa a 216. La diferencia de error en la página 14 sale de ahí, no del lector.

**Orden de lectura:**
- **Formulario aplanado (01):** con el ajuste normal, Docling pone los rótulos en un bloque y los valores en otro. Leyendo la página entera, cada valor queda pegado a su rótulo. El par que falla siempre es la fecha del formulario, que está escrita más arriba que su rótulo.
- **Boleta (05):** Docling deja juntos los dos importes en un renglón y los separa de su concepto. pdfium por posición los deja bien.
- **Escritos digitales (03 y 23 del 02):** bien en todas las configuraciones.
- **Párrafos entre páginas:** Docling une en un solo párrafo el texto que sigue de una página a la siguiente. Es bueno para leer, pero hay que tenerlo en cuenta al guardar el texto por página.

**Un problema grave de Docling que no se ve a simple vista:** cuando decide que una zona es una «imagen» (una foto, un logo, un sello), el texto que reconoce adentro **no aparece en su Markdown**. Solo queda en su archivo JSON, dentro de la imagen. Pasó con:
- la página 1 entera de la boleta 05, que en Markdown quedó vacía;
- la libreta de matrimonio y la libreta vieja (02, páginas 5 y 16), que quedaron vacías;
- los DNI, de los que en Markdown solo aparece la zona de lectura.

Las medidas de la tabla usan el JSON completo. Quien use Docling tiene que leer el JSON, no el Markdown.

**Lo que dice cada documento:**
- **01:** leyendo la página entera, Docling con RapidOCR iguala o supera a pdfium por posición (26 contra 23 pares). Además ordena la página y detecta la tabla de casillas.
- **02:** RapidOCR es el único que da lo impreso casi sin errores y las dos zonas de DNI. Lo manuscrito sale mal en todos.
- **03:** Docling detecta las dos páginas en blanco porque marca el pie del Poder Judicial como pie de página. Lo hace en 7 de las 9 páginas; en la 1 y en la 6 queda un pedazo del pie dentro del texto. En los escaneos 6 a 8, Tesseract con el ajuste normal da más palabras reconocibles (89 %) que RapidOCR (76 %).
- **04:** ver la sección siguiente.
- **05 y 06:** Docling detecta bien las tablas con renglones y columnas dibujadas (cuotas, infracciones). No ve como tabla el bloque Concepto/Importe de la boleta, que no tiene líneas.

## Cómo decide Docling qué reconocer, y qué pasa con el pie y la capa vieja

Lo leí en el código: `docling/models/base_ocr_model.py`, versión 2.142.1. La forma cambió respecto de versiones anteriores: ya no hay un umbral por área de imagen.

1. **Primero busca la forma de la página.** Un modelo (docling-layout-heron) marca zonas: texto, título, tabla, imagen, pie, encabezado.
2. **Con el ajuste normal, manda a reconocer una zona si cumple alguna de estas tres condiciones:**
   - tiene encima o debajo alguna imagen, aunque sea un logo chico;
   - no tiene ninguna letra del PDF;
   - tiene dibujos que el texto del PDF no explica, porque podrían ser letras dibujadas como trazos.
   - No cuenta caracteres ni mira si el texto «parece palabras».
3. **Después junta lo reconocido con el texto del PDF, y gana el texto del PDF.** Lo reconocido se agrega solo donde no se superpone con letras del PDF que se ven. Las letras invisibles se dejan de lado en esa comparación.
4. **Leyendo la página entera, tira el texto del PDF y usa solo lo reconocido.**

**El pie del documento 03: lo resuelve.**
- En las páginas 3 y 9 marca el pie como pie de página y no manda nada a reconocer; esas páginas quedan vacías.
- En las páginas escaneadas (6 a 8), la zona del escaneo tiene una imagen, así que la reconoce.
- El umbral de caracteres de la app de hoy no tiene equivalente en Docling: el pie no lo engaña.

**La capa vieja del documento 04: no la resuelve con el ajuste normal.**
- La capa de ABBYY FineReader 12 está escrita como texto **visible**, tapado por la imagen del escaneo que se dibuja encima. No usa el modo «invisible». Lo verifiqué en el PDF: primero vienen los objetos de texto, después la imagen.
- Docling ve una imagen en cada zona y la manda a reconocer (el 59 % de la superficie en promedio, 5,6 segundos por página con RapidOCR). Pero después **descarta lo reconocido**, porque se superpone con texto del PDF que cree visible, y se queda con la capa vieja.
- Resultado: paga el reconocimiento y no lo usa. Las letras sueltas siguen en 55 %, contra 63 % de la capa original.

**Leyendo la página entera, sí la rehace,** y además más rápido: 2,8 segundos por página, porque no lee la página dos veces. Pero entonces también vuelve a leer las páginas digitales buenas, que no lo necesitaban. Por eso conviene decidir **antes**, página por página, cuáles leer enteras. La señal de letras sueltas de pdfium ya lo detecta.

**Un matiz que vale la pena:** en la página medida, la capa vieja tiene casi todas las letras correctas (3,8 % de error sin contar espacios). Su problema son los espacios metidos entre letras. Leer de nuevo la página la deja en 2,4 %, con palabras bien separadas.

## Señales para partir el documento 02

Docling no parte documentos. Probé una regla simple con lo que sí da: «empieza un documento si la página tiene un título en su tercio de arriba».

| Herramienta | Pasos de página bien (de 22) | Cortes bien | Cortes de más | Cortes perdidos |
|---|---|---|---|---|
| D + RapidOCR | 12 | 6 | 3 | 7 |
| D + Tesseract entera | 14 | 6 | 1 | 7 |
| D + EasyOCR | 13 | 5 | 1 | 8 |

- Las señales de pdfium de la investigación anterior decidieron 9 de los 22 pasos **sin error** («Página 1 de 2», el mismo código IF, las hojas notariales correlativas, los pares de DNI) y dejaron los otros 13 para mirar.
- Docling agrega poco: los encabezados y pies de página que repite (por ejemplo, el de CamScanner) confirman una continuación en un solo caso.
- Sus títulos sirven como pista, pero no deciden solos: 3 cortes de más y 7 perdidos.
- No reemplaza a las reglas de pdfium. A lo sumo es una señal más para los pasos dudosos.

## Correos

Armé un `.eml` inventado (`guiones/hacer_eml.py`), con remitente, destinatarios, copia, fecha, asunto, cuerpo en texto y en HTML, y un PDF adjunto.

- **Lo que lee bien:** el asunto (como título), el remitente, los destinatarios, la copia y el cuerpo, con tildes. También lee archivos `.msg` de Outlook (trae `python-oxmsg`), pero no tuve un `.msg` para probarlo.
- **La fecha:** la pasa a la hora universal (10:30 en Argentina queda como 13:30 con la marca «+00:00»). El instante es correcto, pero se pierde la hora local.
- **El adjunto no lo lee.** Solo puede listar el nombre del archivo, y únicamente si se activa una opción (`list_attachments`). Para leer el PDF adjunto hay que sacarlo del correo y pasarlo por separado.

## granite-docling-258M (modelo de visión)

Lo corrí, con su configuración normal en Docling, sobre 11 páginas: formulario, recibo con tabla, boleta, poder escaneado, acta impresa, certificado escaneado, página de la capa vieja, partida manuscrita, certificado médico manuscrito, acta en cursiva y dorso de DNI.

- **Tiempo:** de 5 a 424 segundos por página; 6 de las 11 páginas tardaron entre 6 y 7 minutos. Usó 2,1 GB de memoria.
- **Sin texto en 3 páginas** (poder escaneado, partida, dorso de DNI): las tomó enteras como imagen.
- **En bucle en 4 páginas** (formulario, boleta, certificado médico y cursiva): repitió una palabra, una letra o la marca de una casilla cientos de veces, hasta el tope de largo de su salida. De ahí vienen los 6 a 7 minutos. El acta impresa y el certificado escaneado tardaron lo mismo sin que la repetición se vea en el resultado.
- **Lo que hizo bien:**
  - en la página de la capa vieja, el error de caracteres más bajo de todos (2,1 %), en 40 segundos;
  - en el formulario, 19 pares de 27 en orden (antes de entrar en bucle);
  - en el acta impresa, una tabla del encabezado.
- **Lo que hizo mal:**
  - no leyó ningún valor manuscrito: en el certificado médico devolvió solo los rótulos;
  - perdió la tabla de cuotas del 06;
  - en la boleta cambió las tildes por caracteres raros.
- **Conclusión:** su ficha dice que es para inglés, y en castellano y en el procesador no rinde. No lo recomiendo.

## Tabla de costo

Segundos por página (todo el documento dividido por sus páginas, incluido el arranque) y memoria máxima del proceso.

| Herramienta | 01 (3 pág.) | 02 (23) | 03 (9) | 04 (93) | 05 (4) | 06 (2) | Memoria máxima |
|---|---|---|---|---|---|---|---|
| pdfium, texto | menos de 0,01 | menos de 0,01 | menos de 0,01 | menos de 0,01 | menos de 0,01 | menos de 0,01 | 58 MB (las 134 páginas en 0,5 segundos) |
| RapidOCR solo (página como imagen) | — | 2,5 | 2,2 | 2,1 | — | — | 1,0 GB |
| D + RapidOCR normal | 5,7 | 4,3 | 3,0 | 5,6 | 4,0 | 4,9 | 3,7 GB |
| D + RapidOCR entera | 6,3 | 3,6 | 3,5 | 2,8 | 4,8 | 5,9 | 3,6 GB |
| D + Tesseract «rápido» normal / entera | 4,3 / 5,2 | 3,0 / 2,9 | 1,7 / 2,3 | 1,8 / 1,6 | 2,9 / 3,8 | 3,8 / 4,2 | 4,7 GB |
| D + Tesseract «mejor» normal / entera | 5,4 / 9,4 | 4,1 / 4,3 | 2,3 / 4,0 | 3,2 / 3,0 | 4,4 / 8,0 | 4,3 / 6,9 | 4,6 GB |
| D + EasyOCR normal / entera | 13 / 26 | 21 / 21 | 9 / 19 | 20 / 22 | 14 / 30 | 14 / 23 | 6,7 GB |
| D sin PyTorch, RapidOCR entera | 5,3 | 3,5 | 3,4 | 3,0 | 4,5 | 4,7 | 2,9 GB |
| granite-docling | 424 | de 5 a 396 | 396 | 40 | 355 | 32 | 2,1 GB |

- En los documentos cortos, buena parte del tiempo es el arranque.
- **Arranque desde cero hasta la primera página** (06, una página, sin conexión):
  - con PyTorch: 12,9 segundos (3,7 para importar, 3,4 para cargar los modelos y 5,5 para la página), 1,6 GB de memoria;
  - sin PyTorch: 8,3 segundos (2,0 + 1,6 + 4,4), 1,2 GB.
- La memoria crece con el largo del documento: con 93 páginas, Docling llegó a 3,6 GB.

**Tamaño de lo instalado**

| Qué | Tamaño |
|---|---|
| Paquetes de Python para Windows, con PyTorch, EasyOCR y Tesseract (medido instalando las versiones de Windows) | 1.254 MB, de los cuales PyTorch ocupa 482 MB |
| Paquetes de Python para Windows sin PyTorch (Docling + RapidOCR + onnxruntime) | 591 MB |
| Lo mismo en Linux, con PyTorch para procesador / sin PyTorch | 2.000 MB / 869 MB |
| Con el PyTorch de PyPI para Linux, que trae lo de placas NVIDIA | 6.300 MB (evitarlo) |
| Modelo de forma de página (heron), versión PyTorch / versión ONNX | 172 MB / 171 MB |
| Modelo de tablas TableFormer, preciso / rápido | 213 MB / 145 MB |
| Modelos de RapidOCR (vienen dentro del paquete) | 31 MB |
| Modelos de EasyOCR | 95 MB |
| Tesseract castellano «rápido» / «mejor» | 2,2 MB / 13,6 MB |
| granite-docling-258M | 530 MB |
| Python embebido para Windows | no medido (el sitio de Python estaba bloqueado en esta sesión) |

**Qué baja Docling la primera vez:**
- los modelos de forma de página y de tablas, desde Hugging Face (`docling-project/docling-layout-heron` y `docling-project/docling-models`);
- la variante ONNX y `granite-docling`, si se usan;
- los dos modelos de EasyOCR, desde GitHub.

RapidOCR no baja nada: sus modelos vienen en el paquete. **Sin conexión funciona:** lo probé con la descarga de Hugging Face bloqueada, con PyTorch y sin él.

## Tabla de licencias

Las de los paquetes salen de sus metadatos instalados. Las de los modelos, de su ficha en Hugging Face o del archivo LICENSE de su repositorio.

| Componente | Licencia | ¿Sirve para vender una app GPL v3? |
|---|---|---|
| Docling, docling-core, docling-parse, docling-ibm-models | MIT | Sí |
| pdfium / pypdfium2 | BSD-3 y Apache 2.0 | Sí |
| RapidOCR | Apache 2.0 | Sí |
| onnxruntime | MIT | Sí |
| PyTorch / torchvision | BSD y otras permisivas (Apache 2.0, MIT) | Sí |
| transformers, huggingface-hub | Apache 2.0 | Sí |
| OpenCV (opencv-python-headless) | Apache 2.0 | Sí |
| NumPy, SciPy | BSD | Sí |
| Tesseract | Apache 2.0 | Sí |
| tesserocr | MIT | Sí |
| EasyOCR | Apache 2.0 | Sí |
| mail-parser / python-oxmsg (correos) | Apache 2.0 / MIT | Sí |
| **Modelo** docling-layout-heron (PyTorch y ONNX) | Apache 2.0 | Sí |
| **Modelo** TableFormer (`docling-project/docling-models`) | CDLA-Permissive-2.0 y Apache 2.0 | Sí. La CDLA permisiva es una licencia de datos que no pone condiciones al uso de lo que se produce con ellos |
| **Modelo** granite-docling-258M | Apache 2.0 | Sí (pero no lo recomiendo por calidad) |
| **Modelos** PP-OCRv6 de RapidOCR (vienen de PaddleOCR) | Apache 2.0 | Sí |
| **Modelo** de reconocimiento de EasyOCR (latin_g2) | Apache 2.0 (repositorio de EasyOCR) | Sí |
| **Modelo** detector CRAFT de EasyOCR | MIT (NAVER) | Sí |
| **Modelos** de castellano de Tesseract (`tessdata` y `tessdata_best`) | Apache 2.0 | Sí |

- Apache 2.0, MIT y BSD son compatibles con la GPL v3, y la GPL v3 permite vender.
- Ninguno de estos componentes tiene la restricción de Surya (límite de facturación) ni la AGPL de los modelos de MinerU.
- Lo único para mirar con un abogado es si la CDLA del modelo de tablas pide algo al redistribuir el archivo del modelo dentro del instalador. Mi lectura es que no, pero no es una opinión legal.

## Para llevarlo a una app de Windows

**Cómo se empaquetaría:** un proceso aparte, con Python embebido y los paquetes ya instalados, que la app en Rust lanza y con el que habla por la entrada y salida estándar o por un puerto local. Los modelos se distribuyen con el instalador o se bajan la primera vez, y quedan en una carpeta de la app.

**Con PyTorch (Docling tal como viene):**
- unos 1,25 GB de paquetes más unos 530 MB de modelos (forma de página y tablas);
- arranca en unos 13 segundos;
- funciona sin conexión.

**Sin PyTorch, solo con ONNX:**
- unos 590 MB de paquetes más 171 MB del modelo de forma de página;
- arranca en unos 8 segundos y usa menos memoria;
- el texto sale igual que con PyTorch (98 % a 100 % de coincidencia; los pares y las zonas de DNI, iguales).
- **Pero Docling no lo permite tal como viene:** tuve que cambiarle tres cosas, que están en `guiones/docling_onnx_run.py`.
  1. Para elegir el dispositivo (procesador o placa de video), Docling importa PyTorch aunque el motor sea ONNX. Lo reemplacé por «siempre el procesador».
  2. El preprocesador de imágenes del modelo de forma de página elige la variante que necesita PyTorch. Lo forcé a la variante que solo usa la biblioteca de imágenes PIL.
  3. Dos módulos de tablas que no se usan importan PyTorch al arrancar. Los reemplacé por módulos vacíos.
- **Lo que se pierde: la detección de tablas.** El modelo de tablas (TableFormer) no tiene versión ONNX. Sin él, Docling marca dónde hay una tabla pero no separa sus celdas.
- Estos cambios habría que mantenerlos en cada versión nueva de Docling, o proponerlos al proyecto.

**Una alternativa más liviana:** no usar Docling como programa, sino solo sus piezas, desde Rust:
- pdfium, que la app ya tiene;
- el modelo de forma de página en ONNX (171 MB, Apache);
- RapidOCR en ONNX (31 MB, Apache), con onnxruntime.

Así no hace falta Python. Es más trabajo de construcción que empaquetar Docling, pero evita Python, PyTorch y los remiendos. No lo probé en Rust: solo verifiqué que el modelo ONNX corre con onnxruntime.

## Recomendación

**Qué usaría para cada forma de página:**

| Forma de página | Qué usaría | En el dispositivo |
|---|---|---|
| Digital en prosa | pdfium, como hoy, sin contar los renglones repetidos (pies) | Sí |
| Formulario digital con campos | pdfium: cada campo con su valor | Sí |
| Formulario aplanado (Docusign, Acrobat Sign) | pdfium por posición (23 de 27 pares), o Docling leyendo la página entera con RapidOCR (26 de 27) si se acepta su peso. Las casillas, con pdfium por posición | Sí |
| Tabla con líneas | Docling con TableFormer (necesita PyTorch). Sin PyTorch, pdfium por posición da los renglones bien pero sin separar las celdas | Sí |
| Tabla sin líneas (Concepto / Importe) | pdfium por posición; ninguna herramienta la detectó como tabla | Sí |
| Escaneo impreso | RapidOCR a unos 200 puntos por pulgada | Sí |
| Formulario completado a mano (partidas, certificados) | Rótulos impresos con RapidOCR; valores manuscritos con un modelo con imagen | Los rótulos sí; lo manuscrito no |
| Manuscrito y cursiva | Modelo con imagen (Core u otro servicio). Ningún lector local sirve | No |
| Documento de identidad | RapidOCR para la zona de lectura, con sus dígitos de control. Tesseract no la leyó | Sí |
| En blanco o solo el pie del sistema | Señal de pdfium (renglones repetidos), sin leer nada. Docling también lo detecta, pero después de pasar el modelo de forma de página | Sí |
| Capa de texto vieja | Detectarla con la proporción de letras sueltas de pdfium (no con «texto invisible»: esta capa no lo es) y leer esas páginas enteras con RapidOCR | Sí |
| Correo (.eml, .msg) | El lector de correos de Docling, que no necesita modelos, o uno propio. Los adjuntos se sacan y se procesan como documentos aparte | Sí |

**En qué orden lo construiría:**

1. **La ficha de forma por página con pdfium**, que la investigación anterior ya propuso y no necesita modelos. Esta prueba agrega una señal: la capa vieja se detecta por las letras sueltas, no por el texto invisible.
2. **RapidOCR local** para las páginas que la ficha marque como escaneo, capa vieja o documento de identidad, con la verificación de los dígitos de control. Es lo que más mejora la calidad con menos peso (31 MB de modelos, unos 2 segundos por página y 1 GB de memoria).
3. **Lectura por posición** para formularios aplanados y tablas sin líneas, con pdfium.
4. **Recién después, decidir si sumar Docling.** Su aporte propio es la detección de tablas con líneas y el orden de páginas complejas, y para eso cuesta PyTorch. Si se suma, que sea **leyendo la página entera, solo en las páginas que la ficha mande**, y leyendo su JSON, no su Markdown. La otra opción es usar solo su modelo de forma de página en ONNX desde Rust.
5. **Lo manuscrito y la cursiva, a un modelo con imagen** (Core), y solo la parte manuscrita de la página.

## Lo que no se pudo probar, y por qué

- **Windows de verdad:** la sesión corre en Linux. Los tamaños de Windows salen de instalar las versiones de Windows de los paquetes en una carpeta, sin ejecutarlas. Los tiempos son de este procesador.
- **El tamaño de Python embebido para Windows:** el sitio de descargas de Python estaba bloqueado en esta sesión.
- **Un `.msg` de Outlook:** no había ejemplos, y armar uno inventado no es trivial. Docling dice leerlo; no lo verifiqué.
- **`granite-docling` en las 11 páginas completas:** en 3 no devolvió texto y en 4 se cortó por el tope de largo de su salida, así que en esas no hay una medida de calidad justa. No lo probé con una placa de video, donde sería mucho más rápido.
- **Una transcripción completa de páginas:** la calidad de lo impreso se midió sobre dos trozos de 600 caracteres, y la de lo manuscrito sobre 37 valores de dos páginas. Para el resto se usó la proporción de palabras reconocibles, que es una señal más gruesa.
- **La instalación de PyTorch para procesador:** al principio no se pudo, porque el sitio de PyTorch estaba bloqueado. Después de habilitarlo se rehízo todo con la versión para procesador; los tiempos y tamaños de este informe son de esa versión.
- **El ajuste de resolución de RapidOCR solo:** se corrió a unos 140 puntos por pulgada. Docling lo pasa a 216 y da menos error. No repetí la prueba sola a más resolución.

## Revisión de datos reales

- Antes del commit busqué en todos los archivos de esta carpeta:
  - números con forma de DNI o CUIT, correos, teléfonos y fechas;
  - los nombres, apellidos, calles, localidades y números de documento que aparecen en los seis PDF.
- No quedó ninguno. Los únicos nombres de personas o lugares son los del correo inventado, que son ficticios.
- Las planillas de respuestas (pares, trozos transcritos, valores manuscritos) y todo lo que sacaron las herramientas quedaron fuera del repositorio.
- `metricas.json` tiene solo números y nombres neutros de campos («documento del cónyuge», «correo laboral»).
