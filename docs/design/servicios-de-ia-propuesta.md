# Servicios de IA configurables, reconocimiento de texto y recuperación de Clip y la API — especificación y plan

**Estado:** en construcción · actualizado el 22 de octubre de 2026 con la respuesta de Core al mensaje 001 y la primera lectura real por Core · construidos los pasos 0a a 3; los pasos 4 y 5 son parte del producto mínimo (D-463, D-468)
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
| Preguntar al modelo | Tipo y datos de cada documento, lectura (personas, afirmaciones, resumen), identidad, asuntos, síntesis, chat | Codex o Claude Code (línea de comandos), API de un proveedor, Core: pedidos de texto a modelos (`executePrompt`), solo texto, sin instrucciones de sistema, herramientas ni respuesta con estructura; Desktop pide el JSON dentro del texto y lo controla (paso 4b) |
| Reconocer texto de imágenes | Escaneos, fotos, PDF sin texto | Codex (el que existe hoy), Core (Textract), otros más adelante (un modelo que lea imágenes vía API, un reconocedor local) |
| Describir imágenes | Lo que hay en una imagen dentro de un documento (P-173) | Un modelo que lea imágenes; Core no recibe imágenes en sus modelos (respuesta 001-R11, lo decide el dueño de Core) |
| Investigar en internet | Búsqueda web y síntesis (por ficha, D-397) | Buscadores configurables (los de hoy) + el motor de preguntas |
| Transcribir audio | Reuniones (P-062, más adelante) | Core (Transcribe), otros |
| Vectores | Búsqueda por significado: se retiró con el wiki y hoy no hay nada que la haga | Core (bases documentales con búsqueda que devuelve fragmentos, mensaje 002), local |

Reglas:

1. **Un motor por servicio**, elegido en Ajustes, con su configuración (modelo, nivel de razonamiento, opciones). Se puede elegir además un **motor alternativo** para lo que el principal no hace bien (por ejemplo, manuscrito en castellano: Textract solo lo lee en inglés).
2. **Un contrato por servicio** en el programa (hoy existen dos: preguntar, `DocumentAsker`, y reconocer texto, `TextRecognizer` en `src-tauri/src/source_volume/`). Cada motor es una implementación de ese contrato. Agregar un motor no toca a quien lo usa.
3. **Cada resultado guarda qué motor lo produjo** (como hoy `codex-cli:gpt-6-luna`), para poder comparar, volver a pedir con otro motor y explicar de dónde salió un dato.
4. **Conexiones compartidas:** los datos para hablar con un proveedor (por ejemplo Core: dirección, organización, credencial) se cargan una vez y los usan todos los servicios que lo elijan. Los secretos van al **llavero del sistema operativo**, no a un archivo de configuración.
5. **Capacidades declaradas:** cada motor dice qué sabe hacer (por ejemplo: texto, tablas, casillas, firmas, manuscrito, idiomas). La base pide lo que necesita y avisa en Atención cuando el motor elegido no puede (por ejemplo, "este formulario tiene casillas y el motor elegido no las lee").
6. **Costo y privacidad a la vista:** Ajustes muestra, para cada motor, dónde se procesa el documento y qué guarda el proveedor. Core: procesa en Oregón (Estados Unidos); Textract guarda el resultado 7 días y no se puede borrar antes; la política que impide que Amazon use los datos todavía no está aplicada (Core P-167). El dueño asume procesar ahí los documentos de clientes (D-482). Desktop borra la lectura en Core apenas la guarda y le manda un nombre genérico, no el del archivo. El uso del modelo sigue la regla de siempre: se usa donde agrega valor, con aviso de costo (D-150).

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

Lo que sigue sale de la respuesta de Core al mensaje 001 (`docs/dropbox/desktop/2026-10-21-001-reconocimiento-y-conexion.md` del repositorio de Core) y de la prueba del 22 oct con el usuario técnico de desarrollo.

- **Conexión** (Ajustes → Conexiones → Micelya Core): dirección de la API (desarrollo: `https://api.core.micelya.net/v1`), emisor de Keycloak, organización y credencial. La organización y la credencial van al **llavero del sistema operativo** (hoy, para las pruebas, en el llavero de Windows con el nombre `connection:micelya-core`), nunca al repositorio ni al buzón. "Probar la conexión" pide el permiso de acceso (dura una hora) y la lista de lecturas.
- **Autenticación:**
  - **hoy,** un usuario técnico por organización (`client_credentials`). Sus lecturas son "personales" de ese usuario: todas las PC que comparten la credencial ven las mismas;
  - **como destino,** Core aceptó las dos variantes: cada persona inicia sesión en el navegador del sistema y vuelve a la app (código de autorización con PKCE, renovación en el llavero), o la app actúa como usuario técnico de la organización. Falta que Core las especifique (Core P-177 y Core P-178).
- **Motor de reconocimiento de Core** (implementa `TextRecognizer`):
  - **los cinco pasos:** pedir (con un nombre genérico), subir antes de que venza el permiso de 15 minutos, avisar, consultar cada 5 segundos (Core no avisa cuando termina) y leer; después, **borrar en Core** la lectura guardada;
  - **por páginas o piezas, no el documento entero** (D-483): un escaneo de muchas páginas suele juntar cosas distintas (un escrito, fotos de DNI, facturas, manuscritos). Desktop manda cada página o pieza por separado, guarda el resultado de cada una y, cuando Core lo ofrezca, pide un tratamiento distinto según el tipo (DNI, facturas y otros documentos conocidos, que Core está evaluando). Además evita el límite de 100 páginas, que falla al final y se cobra igual. Límites: PDF y TIFF hasta 50 MB, JPEG y PNG hasta 10 MB;
  - **páginas sin leer:** Core no informa las páginas que Textract no pudo leer; llegan sin renglones. Desktop las marca como hoy marca «[ilegible]» y las ofrece a otro motor;
  - **confianza:** cada renglón y cada palabra traen la suya (de 0 a 1). Desktop marca lo dudoso (umbral provisional: 0,90);
  - **motor y versión:** el resultado trae `provider` y `modelVersion`; se guardan como motor (`core:textract-1.0`);
  - **fallas y reintentos:** el motivo de una falla es un texto para mostrar, no para decidir; antes de repetir un pedido se mira la lista, para no pagar dos veces.
- **Primera lectura real (22 oct):** un escrito judicial escaneado de 23 páginas, con muchas fotos de documentos y manuscritos, en 27 s de punta a punta (unos 3,5 centavos de dólar). Leyó las 23 páginas, pero el 26 % de las palabras quedó con confianza menor a 0,90, concentrado en las páginas de imágenes difíciles; la lectura de Codex del mismo archivo tenía 84 huecos «[ilegible]», y coinciden tres de cada cuatro palabras y 34 de unos 50 números. Para elegir motor hay que comparar página por página contra el original (paso 6).
- **Preguntar al modelo por Core** (paso 4b): `executePrompt` recibe un modelo y un texto y devuelve un texto, sin historial. Desktop arma la pregunta entera en el texto, pide la respuesta en JSON y la controla como hoy; el chat necesita además un protocolo de acciones en el texto (P-286). Faltan el permiso `inference.execute`, la lista de modelos habilitados y el límite de tamaño del texto (mensaje 002).
- **Contrato:** `packages/contracts/src/http/openapi.yaml` de Core, versión 0.5.0-draft, con referencias entre archivos y todavía sin validar con una herramienta de OpenAPI. Los tipos de Desktop se generan de ahí; las pruebas usan un **Core simulado**, para no depender de que el ambiente de desarrollo esté disponible (se puede reiniciar y borrar organizaciones y credenciales).
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
| 0d | **Plantillas de documentos** (P-247, prioridad del dueño): extraer la plantilla de cada familia de "misma plantilla" (texto común y campos variables, alineando los textos, sin modelo), mostrarla con sus campos y el valor de cada documento. Más adelante, documentos nuevos desde una plantilla. | Mediano | — |
| 1 | **Registro de servicios y motores:** un lugar único en Ajustes con un renglón por servicio (preguntar, reconocer texto, investigar), su motor, su configuración y su motor alternativo; conexiones compartidas; secretos en el llavero; capacidades declaradas. Migra la configuración actual (`ask-engine`, ajustes del chat) sin cambiar el comportamiento. | Mediano | — |
| 2 | **Reconocimiento en la base (P-242)** con el motor que existe (Codex): en segundo plano con la lectura automática, a pedido desde el documento y Atención, una vez por documento y motor. | Mediano | 1 |
| 3 | **Pedido a Core** (`docs/dropbox/core/`), respuesta y acuerdo; organización de prueba y usuario técnico. Hecho el 22 oct: respuesta al 001 leída, acceso probado; sigue el mensaje 002. | Chico | — |
| 4 | **Conexión con Core** en Ajustes y "Probar la conexión"; Core simulado para las pruebas; tipos desde el contrato de Core. Parte del producto mínimo. | Mediano | 1, 3 |
| 4b | **Preguntar al modelo por Core**: la lectura, quién es quién, resúmenes, casos y el chat con un modelo de Core, pidiendo el JSON dentro del texto. Parte del producto mínimo: el estudio de referencia no usa Codex (D-468). | Mediano a grande | 4 y la respuesta al 002 |
| 5 | **Motor de reconocimiento de Core** (Textract), por páginas o piezas, con las páginas sin leer marcadas y el borrado en Core. Parte del producto mínimo. | Mediano | 4 |
| 6 | **Comparación de motores** página por página contra el original, empezando por la carpeta de prueba del PJN (la lectura de Codex ya está guardada; Core cuesta centavos): códigos e importes carácter por carácter, casillas, manuscrito, sellos, costo y tiempo. El dueño elige el motor por defecto y el alternativo. | Chico a mediano | 2, 5 |
| 7 | **Clip sobre la base.** | Mediano | — |
| 8 | **API local y MCP sobre la base**, de solo lectura primero. | Grande | 1 |
| 9 | **Opciones de análisis** (tablas, formularios, firmas, consultas; están en el contrato de Core desde el 22 oct) y tratamiento por tipo de pieza (DNI, facturas), cuando Core los ofrezca. | Según lo que ofrezca Core | 5 y la respuesta al 002 |

**Prioridad:** los pasos 0a a 3 están hechos. Ahora, por el producto mínimo (D-463), 4, 5 y 4b; después 6, 9, 7 y 8.

## 4. Decisiones abiertas para el dueño

1. Motor por defecto y alternativo para reconocer texto (después de la comparación del paso 6).
2. Si el reconocimiento corre solo con la lectura automática o también siempre a pedido (propuesta: las dos).
3. Carpeta donde se guardan los recortes de Clip.
4. Si la API local permite guardar notas o consultas, o solo leer.
5. ~~Si un documento con datos personales sensibles puede salir del país~~ Decidido el 22 oct: el dueño asume que Core procese en Estados Unidos los documentos de clientes (D-482).
