# Servicios de IA configurables, reconocimiento de texto y recuperación de Clip y la API — especificación y plan

**Estado:** propuesta para revisar · 21 de octubre de 2026 · nada de esto está construido
**Alcance:** cómo se configuran y usan los servicios de IA en Desktop (con Micelya Core como un proveedor más), cómo la base reconoce sola el texto de escaneos e imágenes, y cómo se recuperan sobre la base el recortador web (Clip) y el acceso para otros asistentes (API local y MCP). Incluye, por pedido del dueño del 21 oct, el formato uniforme de las pantallas de documento, persona u organización y asunto, porque va primero en el plan. Lo que se pide o se pregunta al equipo de Core está en [`docs/dropbox/core/`](../dropbox/core/).

## 1. Por qué

- **Se perdió una función.** Al retirar el wiki (PR #60) la base dejó de reconocer el texto de los documentos que son imagen: lo hacía la incorporación del wiki con el motor configurado y se lo devolvía a la base (P-242). Los 66 documentos de prueba ya están leídos; un escaneo nuevo queda sin leer.
- **Un requisito del dueño (21 oct):** cada servicio de IA tiene que poder usarse con distintos motores, configurables, como ya pasa con las preguntas al modelo. Core es un proveedor más, nunca el único; Desktop tiene que seguir funcionando con motores locales o propios (D-291, P-141).
- **Core ya ofrece reconocimiento de texto con Textract** (relevamiento del 21 oct, `llm_wiki_mi\core\relevamiento-core-2026-10-21.md`, fuera del repositorio), en un ambiente de desarrollo.
- **Clip y la API local con MCP se retiraron** con el wiki porque escribían en sus archivos (P-227); hay que rehacerlos sobre la base.

## 2. Especificación

### 2.1 Servicios y motores

| Servicio | Para qué | Motores posibles |
|---|---|---|
| Preguntar al modelo | Tipo y datos de cada documento, lectura (personas, afirmaciones, resumen), identidad, asuntos, síntesis, chat | Codex o Claude Code (línea de comandos), API de un proveedor, Core (cuando ofrezca inferencia con lo que hace falta) |
| Reconocer texto de imágenes | Escaneos, fotos, PDF sin texto | Codex (el que existe hoy), Core (Textract), otros más adelante (un modelo que lea imágenes vía API, un reconocedor local) |
| Describir imágenes | Lo que hay en una imagen dentro de un documento (P-173) | Un modelo que lea imágenes; Core (Bedrock con imágenes, si lo ofrece) |
| Investigar en internet | Búsqueda web y síntesis (por ficha, D-397) | Buscadores configurables (los de hoy) + el motor de preguntas |
| Transcribir audio | Reuniones (P-062, más adelante) | Core (Transcribe), otros |
| Vectores | Búsqueda por significado (más adelante) | Core, local |

Reglas:

1. **Un motor por servicio**, elegido en Ajustes, con su configuración (modelo, nivel de razonamiento, opciones). Se puede elegir además un **motor alternativo** para lo que el principal no hace bien (por ejemplo, manuscrito en castellano: Textract solo lo lee en inglés).
2. **Un contrato por servicio** en el programa (hoy existen dos: preguntar, `DocumentAsker`, y reconocer texto, `TextRecognizer` en `src-tauri/src/source_volume/`). Cada motor es una implementación de ese contrato. Agregar un motor no toca a quien lo usa.
3. **Cada resultado guarda qué motor lo produjo** (como hoy `codex-cli:gpt-6-luna`), para poder comparar, volver a pedir con otro motor y explicar de dónde salió un dato.
4. **Conexiones compartidas:** los datos para hablar con un proveedor (por ejemplo Core: dirección, organización, credencial) se cargan una vez y los usan todos los servicios que lo elijan. Los secretos van al **llavero del sistema operativo**, no a un archivo de configuración.
5. **Capacidades declaradas:** cada motor dice qué sabe hacer (por ejemplo: texto, tablas, casillas, firmas, manuscrito, idiomas). La base pide lo que necesita y avisa en Atención cuando el motor elegido no puede (por ejemplo, "este formulario tiene casillas y el motor elegido no las lee").
6. **Costo y privacidad a la vista:** Ajustes muestra, para cada motor, dónde se procesa el documento y qué guarda el proveedor (por ejemplo, Textract conserva el resultado 7 días en Estados Unidos). El uso del modelo sigue la regla de siempre: se usa donde agrega valor, con aviso de costo (D-150).

### 2.2 Reconocimiento de texto en la base (P-242)

- **Qué documentos:** los que la base marca como "tienen el texto en imagen" (escaneos, fotos, PDF sin capa de texto) y las páginas sin texto de un PDF mixto.
- **Cuándo:**
  - **solo**, en segundo plano, de a un documento, mientras la "Lectura automática" esté activa (el mismo interruptor que gobierna las preguntas al modelo);
  - **a pedido**, con un botón "Reconocer el texto" en el detalle del documento y en el aviso de Atención, aunque la lectura automática esté apagada.
- **Una sola vez por documento y por motor:** el texto se guarda con `store_recognized_text` junto al motor que lo produjo; las copias del mismo archivo no se vuelven a reconocer. Cambiar de motor no relee lo ya leído salvo que una persona lo pida (como P-090 para la lectura).
- **Después,** el documento sigue el camino normal: tipo, lectura, personas, asuntos, búsqueda.
- **Lo que se informa:** en el detalle del documento, "texto reconocido por <motor> el <fecha>", y las páginas o renglones de baja confianza cuando el motor la informa.
- **Atención:** cuántos documentos esperan reconocimiento, con su costo estimado, y los que fallaron con el motivo.

### 2.3 Core como proveedor

- **Conexión** (Ajustes → Conexiones → Micelya Core): dirección de la API, dirección de inicio de sesión, organización (tenant) y credencial. Botón "Probar la conexión".
- **Autenticación:**
  - **para empezar,** un usuario técnico de una organización de prueba (identificador y secreto, en el llavero);
  - **como destino,** el inicio de sesión de cada persona desde la app (cada abogado con su usuario), cuando Core tenga un cliente para escritorio. Ver el pedido a Core.
- **Motor de reconocimiento de Core:** implementa `TextRecognizer` con la lectura asincrónica de Core: pedir, subir el archivo, avisar, consultar el estado con espera creciente y un tope, leer el resultado y **borrarlo en Core** después de guardarlo en la base (privacidad). Envía el archivo entero cuando se puede (Core admite PDF de varias páginas).
- **Contrato:** los tipos de Desktop salen del mismo archivo de contrato de Core (`packages/contracts/src/http/openapi.yaml`); las pruebas de Desktop usan un **Core simulado**, para no depender de que el ambiente de Core esté disponible.
- **Coordinación:** Desktop escribe en `docs/dropbox/core/` de este repositorio; Core responde en `docs/dropbox/desktop/` de su repositorio (ver `CLAUDE.md`).

### 2.4 Clip sobre la base (P-227)

- La extensión del navegador se reutiliza (sigue en la historia de git: `extension/`).
- La app recibe el recorte en su servidor local y lo guarda como **archivo markdown** (con la dirección de la página y la fecha) en una **carpeta de un origen** que la persona elige en Ajustes (por ejemplo "Recortes web").
- La base lo detecta en su recorrido y lo procesa como cualquier documento: queda buscable, con sus personas, y se puede sumar a un caso.
- Atención avisa de los recortes nuevos.

### 2.5 API local y MCP sobre la base (P-227)

- **Primero de solo lectura,** con token obligatorio: buscar en la base, leer un documento, una persona, una organización o un asunto (su artículo y sus datos), preguntarle al chat con citas, consultar el grafo.
- **Escritura acotada,** a decidir: guardar una nota o una consulta guardada. Nunca escribir páginas ni cambiar datos de la capa de conocimiento.
- El servidor MCP (paquete aparte, en la historia de git: `mcp-server/`) se adapta a esas herramientas.
- Más adelante, cuando Core ofrezca MCP, ver si conviene que el acceso de otros asistentes pase por Core.

### 2.6 Las pantallas con el mismo formato (pedido del dueño, 21 oct)

- **Documento, persona u organización y asunto con las mismas dos pestañas:** «Artículo» (lo que se lee: el texto con sus citas) y «Datos y correcciones» (los datos uno por uno, con su cita y sus acciones). Hoy solo persona u organización las tiene; el documento muestra todo en una sola página y el asunto no tiene artículo.
  - **Documento:** Artículo = tema, lo esencial con citas y certeza, importes, versiones y relacionados; Datos y correcciones = tipo y etapa, datos del tipo, personas y organizaciones con su rol, referencias, asuntos (agregar o sacar), ubicaciones y copias, texto completo, etiquetas.
  - **Asunto:** Artículo = el que ya arma la base para el asunto (partes con su rol, lo que dicen sus documentos, documentos con su tipo y por qué están); Datos y correcciones = lo que hoy tiene la pantalla (nombre, confirmar o descartar, documentos firmes y propuestos, agregar y sacar, decisiones con deshacer).
  - Las notas de una persona van al pie de las dos pestañas; la investigación en internet, en el Artículo.
- **Sin repeticiones en persona u organización:** hoy el Artículo tiene «Referencias» (cada dato que usa la síntesis con su cita y su documento) y «Otros datos», y además «Documentos» (todos sus documentos por asunto); las tres llevan a los mismos documentos, y los datos con su cita ya están en «Datos y correcciones». Propuesta: las citas quedan **como notas numeradas dentro del texto** (al pasar o tocar el número se ve la cita y se abre el documento); «Referencias» y «Otros datos» salen del Artículo (siguen en Datos y correcciones, con sus acciones); «Documentos» queda una sola vez, por asunto. El mismo criterio vale para documento y asunto.

### 2.7 El chat

El chat ya está en el menú y, con el retiro del wiki (PR #60), busca y lee en la base; usa el motor configurado para el chat (hoy Codex, `gpt-6-luna`, razonamiento alto). No falta nada para usarlo. Falta **verificarlo con una pregunta real** (usa cupo; Desktop P-239) y lo pendiente: adjuntar al chat un documento de la base (Desktop P-243). Con el paso 1 del plan, el chat pasa a ser un servicio más del registro de motores.

## 3. Plan de trabajo

Esfuerzo relativo (nunca en tiempo). Cada paso tiene algo visible para revisar en la app.

| # | Paso | Esfuerzo | Depende de |
|---|---|---|---|
| 0a | **Pantallas con el mismo formato** (sección 2.6): pestañas Artículo y Datos y correcciones en documento y asunto; el asunto con su artículo. Sin modelo. | Chico a mediano | — |
| 0b | **Sin repeticiones** en persona u organización (y documento y asunto): citas como notas dentro del texto; fuera «Referencias» y «Otros datos» del Artículo. Sin modelo (cambia cómo se escribe la página, no la síntesis). | Chico | — |
| 0c | **Verificar el chat** con una pregunta real (con aviso de cupo) y guardar una consulta. | Chico | — |
| 1 | **Registro de servicios y motores:** un lugar único en Ajustes con un renglón por servicio (preguntar, reconocer texto, investigar), su motor, su configuración y su motor alternativo; conexiones compartidas; secretos en el llavero; capacidades declaradas. Migra la configuración actual (`ask-engine`, ajustes del chat) sin cambiar el comportamiento. | Mediano | — |
| 2 | **Reconocimiento en la base (P-242)** con el motor que existe (Codex): en segundo plano con la lectura automática, a pedido desde el documento y Atención, una vez por documento y motor. | Mediano | 1 |
| 3 | **Pedido a Core** (`docs/dropbox/core/`), respuesta y acuerdo; organización de prueba y usuario técnico. No es código. | Chico | — |
| 4 | **Conexión con Core** en Ajustes y "Probar la conexión"; Core simulado para las pruebas; tipos desde el contrato de Core. | Mediano | 1, 3 |
| 5 | **Motor de reconocimiento de Core** (Textract), con borrado del resultado en Core después de guardarlo. | Mediano | 4 |
| 6 | **Comparación de motores** con escaneos reales del estudio (con aviso de cupo): códigos e importes carácter por carácter, casillas, manuscrito, sellos, costo y tiempo. El dueño elige el motor por defecto y el alternativo. | Chico a mediano | 2, 5 |
| 7 | **Clip sobre la base.** | Mediano | — |
| 8 | **API local y MCP sobre la base**, de solo lectura primero. | Grande | 1 |
| 9 | **Opciones de análisis** (tablas, casillas, firmas) y **lectura de imágenes con un modelo** cuando Core las ofrezca. | Según lo que ofrezca Core | 5 y la respuesta de Core |

**Prioridad propuesta:** primero 0a, 0b y 0c (lo que se ve todos los días, sin modelo salvo la pregunta de prueba); después 1 y 2 (los escaneos nuevos quedan sin leer hasta entonces); el 3 corre en paralelo desde ya; 4 a 6 cuando Core responda; 7 y 8 después. Los pasos 1 y 2 sirven aunque Core tarde.

## 4. Decisiones abiertas para el dueño

1. Motor por defecto y alternativo para reconocer texto (después de la comparación del paso 6).
2. Si el reconocimiento corre solo con la lectura automática o también siempre a pedido (propuesta: las dos).
3. Carpeta donde se guardan los recortes de Clip.
4. Si la API local permite guardar notas o consultas, o solo leer.
5. Si un documento con datos personales sensibles puede salir del país para reconocerlo (Textract procesa en Estados Unidos) o hace falta un motor local o regional para esos casos.
