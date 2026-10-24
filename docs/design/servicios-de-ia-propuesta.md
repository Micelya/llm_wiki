# Servicios de IA configurables, reconocimiento de texto y recuperación de Clip y la API — especificación y plan

**Estado:** en construcción · propuesta del 21 de octubre de 2026 · actualizado el 22 de octubre con la respuesta de Core al mensaje 001 y la primera lectura real por Core · **puesto al día el 23 de octubre de 2026:** construidos los pasos 0a a 5, 7 y 8 (en este orden: el reconocimiento en la base pasó antes que el registro de motores, D-428; el 8, la API local con el MCP, en la corrida desatendida del 23, D-509). Faltan el 4b (preguntar al modelo por Core), el 6 y el 9, que esperan a Core, y el inicio de sesión por persona con Keycloak, decidido y sin construir (D-501). Falta que el dueño vea en la app lo construido. Los pasos 4, 4b y 5 son parte del producto mínimo (D-463, D-468)
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
| Identificar las piezas de un documento (antes «Describir imágenes», D-486) | Partir un escaneo en piezas, decir qué es cada una y dónde está, y leer cada pieza con su tratamiento: DNI, facturas y otros documentos conocidos (P-173) | Core, que tiene Bedrock además de Textract (mensaje 003, sin respuesta todavía). Sus modelos de texto no reciben imágenes (respuesta 001-R11) |
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

**Cómo quedó (22 oct, D-443 a D-445).** El registro vive en la base y la base lo lee en cada pedido: tres servicios (preguntar al modelo, que vale también para el chat; reconocer escaneos; investigar en internet), cada uno con su motor y un alternativo, su conexión, su modelo y su razonamiento. Los contratos `DocumentAsker` y `TextRecognizer` no cambiaron: el motor de Core se sumó al segundo sin tocar a quien lo usa. Las claves de API pasaron al llavero del sistema, cada una recién después de confirmar que el llavero la guardó; sin llavero quedan en el archivo, como antes. Las capacidades, dónde se procesa y qué guarda cada proveedor están en un catálogo de motores que es un dato (`ai-engines.json`); los textos de privacidad son generales y conviene que el dueño los revise. Hoy la base pregunta al modelo solo con Codex y reconoce escaneos con Codex o con Core; el motor alternativo se elige y se guarda, pero la base todavía no lo usa (P-261). Ajustes → «Lo que se le pregunta al modelo» muestra cada pregunta armada por las mismas funciones que la lectura, sobre un documento inventado (D-481).

### 2.2 Reconocimiento de texto en la base (P-242)

**Construido el 21 de octubre** (D-428 a D-431), antes que el registro de motores; los pedidos a mano se guardan en la base desde el 22 (D-452, versión 22 del esquema). El motor de Core se sumó el 22 (sección 2.3).

- **Qué documentos:** los que la base marca como "tienen el texto en imagen" (escaneos, fotos, PDF sin capa de texto) y las páginas sin texto de un PDF mixto.
- **Cuándo:**
  - **solo**, en segundo plano, de a un documento, mientras la "Lectura automática" esté activa. Va **antes que cualquier otra pregunta al modelo**, porque sin texto el documento no puede seguir (D-430);
  - **a pedido**, con «Reconocer el texto» en el documento y «Reconocer» en la etapa «Texto de los escaneos» de Atención → «En curso», aunque la lectura automática esté apagada. Lo pedido va primero y queda guardado en la base aunque se cierre la app.
- **Con qué motor:** el que Ajustes → «Servicios de IA» elige para «Reconocer escaneos»: Codex con su modelo (D-429, D-443) o Micelya Core (Textract), que se puede elegir una vez cargada su conexión. Sin motor, la etapa y el documento lo dicen, con el camino a Ajustes, y los botones quedan apagados.
- **Una sola vez por documento y por motor:** el texto se guarda junto al motor que lo produjo; las copias no se vuelven a reconocer. Las páginas ya leídas de un escaneo largo se guardan, así un reintento lee solo lo que faltaba. Cambiar de motor no relee lo ya leído salvo que una persona lo pida («Reconocer de nuevo con <motor>»).
- **Cuando falla:** una falla del proveedor (límite de uso, sin conexión, no responde) deja el documento esperando y la etapa muestra el error con la hora en que vuelve a intentar; tres fallas con el mismo documento, o una falla del documento (una página que no se pudo convertir, un archivo que no está), lo marcan con el motivo en palabras hasta que alguien toque «Reintentar».
- **Después,** el documento sigue el camino normal: tipo, lectura, personas, casos, búsqueda.
- **Lo que se informa:** en «Datos y correcciones» del documento, un recuadro con «Texto reconocido por <motor> el <fecha>» y las páginas donde el motor escribió «[ilegible]», con el consejo de compararlas con el original (D-431). Codex no informa confianza; Core informa la de cada palabra, y las de menos de 0,90 se guardan aparte sin cambiar el texto (D-489): la pantalla todavía no las muestra.
- **Atención:** la etapa «Texto de los escaneos» dice cuántos documentos esperan, cuántas páginas faltan, cuál se lee ahora y cuáles fallaron con el motivo. El tamaño del trabajo se da en páginas, no en dinero.
- **Falta:** que el dueño lo vea en la app (P-242).

### 2.3 Core como proveedor

Lo que sigue sale de la respuesta de Core al mensaje 001 (`docs/dropbox/desktop/2026-10-21-001-reconocimiento-y-conexion.md` del repositorio de Core) y de la prueba del 22 oct con el usuario técnico de desarrollo.

- **Conexión** (Ajustes → Conexiones → Micelya Core; **construida el 22 oct**): dirección de la API (desarrollo: `https://api.core.micelya.net/v1`), emisor de Keycloak, organización y credencial, con el secreto oculto y «Cambiar». La organización y la credencial van al **llavero del sistema operativo** (en Windows, con el nombre `connection:micelya-core`), nunca al repositorio ni al buzón, y el secreto nunca vuelve a las pantallas. «Probar la conexión» pide el permiso de acceso (dura una hora) y la lista de lecturas, sin guardar nada, y dice en palabras si anduvo o por qué no. Las pruebas usan un Core simulado.
- **Autenticación:**
  - **hoy, solo para desarrollo,** un usuario técnico de la organización (`client_credentials`). Sus lecturas son "personales" de ese usuario: todas las PC que comparten la credencial ven las mismas, por eso cada PC lleva un registro de las suyas (D-494);
  - **decidido como destino (23 oct, D-501): Keycloak, por cada persona.** Cada persona inicia sesión desde la app, en el navegador del sistema, y vuelve a la app (código de autorización con PKCE, sin secreto dentro del programa, renovación en el llavero, se puede cerrar la sesión); lo que pide a Core queda a su nombre. Reemplaza la variante «la app actúa como usuario técnico de la organización» y lo que pedía el mensaje 002 para el piloto (002-R2), que hay que corregir con Core. **Sin construir** (P-302): espera que Core especifique el cliente de escritorio (Core P-177 y Core P-178) y se definen con el dueño y con Core cómo se crean los usuarios del estudio de referencia en su organización de Keycloak y cómo se trabaja sin conexión (qué se exige al abrir la app sin conexión, qué pasa cuando vence o se revoca la sesión, cómo se marca lo que se hizo sin conexión).
- **Motor de reconocimiento de Core** (implementa `TextRecognizer`; **construido el 22 oct**, D-489 y D-494, a ver por el dueño: P-063, P-292, P-294):
  - **cada página como imagen:** un JPEG en gris de hasta 2000 píxeles, hecho de la misma imagen que la base arma para Codex (así la comparación de motores es pareja y no viajan datos internos del PDF), con el nombre genérico «pagina-N.jpg»;
  - **hasta 4 páginas a la vez,** cada una con su recorrido completo, su huella y su borrado; si una falla, las que están en curso terminan y se borran y no arranca ninguna más. Medido con un escaneo inventado de 8 páginas: de 102 s a 34 s;
  - **solo se borran las lecturas propias:** un registro local (`micelya-core-lecturas.json`, sin nada de los documentos) guarda las lecturas que pidió esta computadora hasta que Core confirma el borrado. Una lectura de otra computadora con la misma imagen se usa sin pagar de nuevo y no se borra. Si Core no deja borrar una, la página se guarda igual, Atención lo avisa con «Reintentar ahora» y la app reintenta cada 10 minutos;
  - **los cinco pasos:** pedir (con un nombre genérico), subir antes de que venza el permiso de 15 minutos, avisar, consultar cada 5 segundos (Core no avisa cuando termina) y leer; después, **borrar en Core** la lectura guardada;
  - **por páginas o piezas, no el documento entero** (D-483): un escaneo de muchas páginas suele juntar cosas distintas (un escrito, fotos de DNI, facturas, manuscritos). Desktop manda cada página o pieza por separado, guarda el resultado de cada una y, cuando Core lo ofrezca, pide un tratamiento distinto según el tipo (DNI, facturas y otros documentos conocidos, que Core está evaluando). Además evita el límite de 100 páginas, que falla al final y se cobra igual. Límites: PDF y TIFF hasta 50 MB, JPEG y PNG hasta 10 MB;
  - **páginas sin leer:** Core no informa las páginas que Textract no pudo leer; llegan sin renglones. Desktop las marca «[ilegible]», como con Codex; ofrecerlas a otro motor espera a que la base use el alternativo (P-261);
  - **confianza:** cada renglón y cada palabra traen la suya (de 0 a 1). Desktop guarda aparte las palabras de menos de 0,90 (umbral provisional), sin cambiar el texto;
  - **motor y versión:** el motor se guarda con el nombre del modelo de Servicios de IA (`core:textract-1.0`) y cada página guarda la versión que informa Core (`modelVersion`);
  - **fallas y reintentos:** el motivo de una falla es un texto para mostrar, no para decidir; cada página va con su huella y, antes de repetir un pedido sin respuesta, se mira la lista: una página nunca se paga dos veces.
- **Primera lectura real (22 oct):** un escrito judicial escaneado de 23 páginas, con muchas fotos de documentos y manuscritos, en 27 s de punta a punta (unos 3,5 centavos de dólar). Leyó las 23 páginas, pero el 26 % de las palabras quedó con confianza menor a 0,90, concentrado en las páginas de imágenes difíciles; la lectura de Codex del mismo archivo tenía 84 huecos «[ilegible]», y coinciden tres de cada cuatro palabras y 34 de unos 50 números. Para elegir motor hay que comparar página por página contra el original (paso 6).
- **Preguntar al modelo por Core** (paso 4b, sin empezar, P-300): `executePrompt` recibe un modelo y un texto y devuelve un texto, sin historial. Desktop arma la pregunta entera en el texto, pide la respuesta en JSON y la controla como hoy; el chat necesita además pedir sus operaciones dentro del texto, con el agente del [contrato de operaciones](contrato-de-operaciones.md) (sección 3.4). Faltan el permiso `inference.execute`, la lista de modelos habilitados y el límite de tamaño del texto (mensaje 002, sin respuesta todavía).
- **Contrato:** `packages/contracts/src/http/openapi.yaml` de Core, versión 0.5.0-draft, con referencias entre archivos y todavía sin validar con una herramienta de OpenAPI. Los tipos de Desktop se generan de ahí; las pruebas usan un **Core simulado**, para no depender de que el ambiente de desarrollo esté disponible (se puede reiniciar y borrar organizaciones y credenciales).
- **Coordinación:** Desktop escribe en `docs/dropbox/core/` de este repositorio; Core responde en `docs/dropbox/desktop/` de su repositorio (ver `CLAUDE.md`).

### 2.4 Clip sobre la base (P-227)

**Construido el 22 de octubre** (D-450, D-451); desde el 23, dentro del servidor único de la app (D-487, D-509). Falta la prueba del dueño con el navegador.

- La extensión del navegador volvió, en español y sin proyectos, como **«Micelya Clipper»**.
- La app la atiende con su **servidor único**, que escucha solo en la propia máquina (`127.0.0.1:19827`, la dirección que ya usaba el recortador): la extensión instalada sigue andando sin reinstalarla, con los mismos pedidos y sin token. El servidor rechaza cualquier página web, también las de la misma máquina y la ventana del portal.
- Cada recorte se guarda como **archivo markdown** en **«Recortes web»**, dentro de la carpeta de trabajo (Ajustes → «Carpeta de trabajo»; lo decidió el dueño el 21 oct), con el nombre «<fecha> <título>.md» y, al principio, la dirección de la página y la fecha del recorte. En el contrato de operaciones es «guardar un recorte», el único cambio que no pide confirmación (D-504).
- La carpeta de trabajo es un origen de la base: el recorte se inventaría y se procesa como cualquier documento (buscable, con sus personas, se puede sumar a un caso).
- Atención muestra «N recortes web nuevos» entre las decisiones, cada uno para abrir, hasta que alguien toca «Ya los vi».
- **Falta:** instalar la extensión sin el repositorio y con la marca de Micelya (P-266), y que deje elegir el caso del recorte (P-305).

### 2.5 API local y MCP sobre la base (P-227)

**Construido el 23 de octubre** en la corrida desatendida (D-509), sobre el [contrato de operaciones](contrato-de-operaciones.md) (D-498, D-504), en el mismo servidor que el recortador (D-487, D-495). Falta que el dueño lo vea. El detalle y las decisiones asumidas están en el contrato, secciones 6 y 13.

- **Qué puede cada programa:** cada uno se agrega en Ajustes → «Programas conectados», con un nombre y un permiso, y recibe su propio token (en el llavero del sistema; la base guarda solo su huella); se puede copiar, renovar o quitar. **«Solo leer»**, por defecto, da las once operaciones de lectura del contrato (buscar, leer un documento, una ficha o un caso, entre otras). **«Leer y pedir cambios»** suma las operaciones de cambio construidas: cada cambio queda como pedido a nombre del programa en Atención → «Pedidos de otros programas», con la misma tarjeta del chat, y vence a los 7 días si nadie lo decide.
- **Lo que no puede:** preguntarle al chat, para no gastar cupo (D-504).
- **Dónde:** la API local en `/api/v1/operaciones` y `/api/v1/pedidos`; el MCP en `/mcp`, por HTTP en el mismo servidor. El paquete aparte de antes (`mcp-server/`, en la historia de git) no se usa.
- **Falta:** probar la configuración para Codex y para Claude Desktop (P-315), y un puente por línea de comandos para los asistentes que no aceptan MCP por HTTP (P-303).
- Más adelante, cuando Core ofrezca MCP, ver si conviene que el acceso de otros asistentes pase por Core.

### 2.6 Las pantallas con el mismo formato (pedido del dueño, 21 oct)

**Construido** el 21 y el 22 de octubre (D-423, D-441, D-442) y, el 23, con la forma de enciclopedia de la maqueta aprobada (D-508); falta que el dueño lo vea (P-214, P-245). Lo que sigue es la especificación de entonces.

- **Documento, persona u organización y asunto con las mismas dos pestañas:** «Artículo» (lo que se lee: el texto con sus citas) y «Datos y correcciones» (los datos uno por uno, con su cita y sus acciones). Hoy solo persona u organización las tiene; el documento muestra todo en una sola página y el asunto no tiene artículo.
  - **Documento:** Artículo = tema, lo esencial con citas y certeza, importes, versiones y relacionados; Datos y correcciones = tipo y etapa, datos del tipo, personas y organizaciones con su rol, referencias, asuntos (agregar o sacar), ubicaciones y copias, texto completo, etiquetas.
  - **Asunto:** Artículo = el que ya arma la base para el asunto (partes con su rol, lo que dicen sus documentos, documentos con su tipo y por qué están); Datos y correcciones = lo que hoy tiene la pantalla (nombre, confirmar o descartar, documentos firmes y propuestos, agregar y sacar, decisiones con deshacer).
  - Las notas de una persona van al pie de las dos pestañas; la investigación en internet, en el Artículo.
- **Sin repeticiones en persona u organización:** hoy el Artículo tiene «Referencias» (cada dato que usa la síntesis con su cita y su documento) y «Otros datos», y además «Documentos» (todos sus documentos por asunto); las tres llevan a los mismos documentos, y los datos con su cita ya están en «Datos y correcciones». Propuesta: las citas quedan **como notas numeradas dentro del texto** (al pasar o tocar el número se ve la cita y se abre el documento); «Referencias» y «Otros datos» salen del Artículo (siguen en Datos y correcciones, con sus acciones); «Documentos» queda una sola vez, por asunto. El mismo criterio vale para documento y asunto.

### 2.7 El chat

El chat está en el menú y, desde el retiro del wiki, busca y lee en la base. **El dueño lo verificó el 21 de octubre** con preguntas reales: responde y cada respuesta trae sus referencias de la base (P-239). Falta probar guardar una consulta. Desde entonces:

- **usa el registro de motores** (22 oct): el renglón «Preguntar al modelo» de «Servicios de IA» vale también para el chat (hoy Codex, `gpt-6-luna`, razonamiento alto); si el chat estaba en otra conexión, la conserva y Ajustes lo dice, con «Que use el mismo» (D-443);
- **adjunta documentos de la base** (22 oct) con el botón «Documento» o escribiendo «@», hasta 8 por mensaje; el agente lee su texto desde la base, con su nombre y su lugar, y los cita. Desde un documento, «Preguntar al chat sobre este documento» (D-453). Falta verlo con una pregunta de verdad (P-243). Adjuntar personas, organizaciones y casos queda para después (P-269);
- **propone acciones** (23 oct, en la corrida desatendida; D-498, D-504, D-506): el chat solo puede pedir operaciones del [contrato de operaciones](contrato-de-operaciones.md); los cambios los confirma una persona en una tarjeta con casillas y se pueden deshacer. Muestra «Vuelta i de n» y, al tope, «Seguir buscando». Codex pasa por el mismo agente, con una corrida de solo lectura por vuelta; Claude Code sigue sin acciones (P-304) y Core en el chat espera el paso 4b (P-300). Falta verlo con un modelo real (P-286).

## 3. Plan de trabajo

Esfuerzo relativo (nunca en tiempo). Cada paso tiene algo visible para revisar en la app. Los pasos van en el orden en que se hicieron o se harán; los números son los de siempre, porque el registro de decisiones los cita.

**Cambio de orden (21 oct, D-428):** el dueño agregó un caso real con muchos escaneos y pidió recuperar el reconocimiento enseguida. Se hizo el paso 2 antes que el 1, con el motor que ya existía (Codex) y siempre a través del contrato `TextRecognizer`, para que el registro de motores se sumara después sin rehacer nada. Así fue: el 22 de octubre el registro tomó el motor del reconocimiento, y después el motor de Core, sin cambiar el contrato.

| # | Paso | Estado al 23 oct | Esfuerzo | Depende de |
|---|---|---|---|---|
| 0a | **Pantallas con el mismo formato** (sección 2.6): pestañas «Artículo» y «Datos y correcciones» en documento y caso; el caso con su artículo | Construido el 21 oct (D-423); el 23, como enciclopedia (D-508). Falta que el dueño lo vea (P-214) | Chico a mediano | — |
| 0b | **Sin repeticiones** en persona u organización, documento y caso: citas como notas dentro del texto; fuera «Referencias» y «Otros datos» del artículo | Construido el 22 oct (D-441, D-442), con título legible del documento y fechas en castellano; se sumó la vista previa flotante de notas y enlaces (D-454, D-455). Falta que el dueño lo vea (P-245, P-246) | Chico | — |
| 0c | **Verificar el chat** con una pregunta real y guardar una consulta | Hecho en parte: el dueño lo probó el 21 oct (P-239); falta guardar una consulta. El 22 oct se sumó adjuntar documentos de la base (D-453; falta verlo, P-243) y el 23, las acciones (D-506; falta verlas, P-286) | Chico | — |
| 0d | **Plantillas de documentos** (P-247): la plantilla de cada familia de "misma plantilla", sus campos y el valor de cada documento; después, documentos nuevos desde una plantilla | Construido el 22 oct: las plantillas (D-438 a D-440, versión 21), la sección «Plantillas» con nombres para los campos (D-466, D-488, D-490) y generar un Word simple (D-492). Falta verlo (P-287, P-257, P-285); el membrete y el Word original como formato, después (P-295) | Mediano | — |
| 2 | **Reconocimiento en la base** (P-242), con el motor que existe (Codex): solo con la lectura automática y a pedido, una vez por documento y motor | Construido el 21 oct (D-428 a D-431), adelantado al paso 1; pedidos a mano guardados en la base el 22 oct (D-452, versión 22). Falta que el dueño lo vea | Mediano | — (antes dependía del 1) |
| 1 | **Registro de servicios y motores:** un lugar único en Ajustes con un renglón por servicio, su motor, su configuración y un alternativo; conexiones compartidas; secretos en el llavero; capacidades declaradas. Sin cambiar el comportamiento | Construido el 22 oct (D-443 a D-445): «Servicios de IA» en Ajustes, registro en la base, claves en el llavero. El alternativo se guarda pero no se usa (P-261); claves de funciones retiradas sin decidir (P-262). Falta que el dueño lo vea (P-141) | Mediano | 2 (usa su contrato) |
| 3 | **Pedido a Core** (`docs/dropbox/core/`), respuesta y acuerdo; organización de prueba y usuario técnico. No es código | Hecho el 22 oct: respuesta al 001 leída y acceso probado. Los mensajes 002 (lectura probada, modelos y piloto) y 003 (las piezas de un escaneo) se enviaron el 22 oct y no tienen respuesta todavía. Lo que el 002 pedía para el piloto (un usuario técnico) quedó reemplazado por el inicio de sesión por persona (D-501): hay que corregirlo con Core | Chico | — |
| 4 | **Conexión con Core** en Ajustes y «Probar la conexión»; Core simulado para las pruebas | Construido el 22 oct (Ajustes → Conexiones → Micelya Core), con el usuario técnico de desarrollo. Falta que el dueño lo vea (P-063). El inicio de sesión por persona con Keycloak (D-501) es un paso nuevo, sin construir (P-302) | Mediano | 1, 3 |
| 4b | **Preguntar al modelo por Core**: la lectura, quién es quién, resúmenes, casos y el chat con un modelo de Core, pidiendo el JSON dentro del texto. Parte del producto mínimo: el estudio de referencia no usa Codex (D-468) | Sin empezar (P-300): espera el permiso y los límites pedidos en el 002. Se puede construir antes contra un Core simulado | Mediano a grande | 4 y la respuesta al 002 |
| 5 | **Motor de reconocimiento de Core** (Textract), por páginas o piezas, con las páginas sin leer marcadas y el borrado en Core | Construido el 22 oct (D-483, D-489, D-494): cada página como imagen, hasta 4 a la vez, borrado de las lecturas propias. Falta que el dueño lo vea (P-063, P-292, P-294) | Mediano | 4 |
| 6 | **Comparación de motores** página por página contra el original, empezando por la carpeta de prueba del PJN (la lectura de Codex ya está guardada; Core cuesta centavos): códigos e importes carácter por carácter, casillas, manuscrito, sellos, costo y tiempo. El dueño elige el motor por defecto y el alternativo | Sin empezar | Chico a mediano | 2, 5 |
| 7 | **Clip sobre la base** | Construido el 22 oct (D-450, D-451) en la carpeta de trabajo; desde el 23, en el servidor único (D-509). Falta la prueba del dueño con el navegador e instalarlo sin el repositorio (P-266) | Mediano | — |
| 8 | **API local y MCP sobre la base**, de solo lectura primero | Construido el 23 oct en la corrida desatendida, sobre el contrato de operaciones (D-504, D-509): leer, y pedir cambios que una persona confirma. Falta que el dueño lo vea (P-227) | Grande | 1 |
| 9 | **Opciones de análisis** (tablas, formularios, firmas, consultas; están en el contrato de Core desde el 22 oct) y **las piezas de un documento** (D-486: qué es cada una y su tratamiento, como DNI o facturas), cuando Core los ofrezca | Sin empezar; las piezas, pedidas a Core en el mensaje 003 (P-173) | Según lo que ofrezca Core | 5 y las respuestas al 002 y al 003 |

**Lo que sigue:** que el dueño vea en la app lo construido (0a a 0d, 1, 2, 4, 5, 7 y 8) y pruebe el recortador; guardar una consulta del chat. Por el producto mínimo (D-463): el paso 4b cuando Core responda el 002 (o antes, contra un Core simulado) y el inicio de sesión por persona, cuando el dueño y Core definan sus detalles (P-302). Después, 6 y 9. Usar el motor alternativo (P-261) tiene sentido con dos motores que funcionen, y ya los hay para reconocer escaneos.

## 4. Decisiones abiertas para el dueño

**Resueltas:**

- Si el reconocimiento corre solo con la lectura automática o también a pedido: **las dos**; lo pedido corre aunque la lectura esté apagada y va primero (D-430, construido el 21 oct).
- Carpeta de los recortes de Clip: **«Recortes web», dentro de la carpeta de trabajo** (el dueño, 21 oct, D-450).
- Si un documento con datos personales sensibles puede salir del país para reconocerlo: **sí**; el dueño asume que Core procese en Estados Unidos los documentos de clientes (22 oct, D-482).
- Si la API local permite guardar notas o consultas, o solo leer: **los programas leen y piden cambios que una persona confirma en la app**; por defecto, solo leen; guardar un recorte es el único cambio sin confirmación, y otros programas no le pueden preguntar al chat (resuelta por el contrato de operaciones, D-504, asumida en la corrida desatendida y a revisar por el dueño).
- Cómo se autentica la app en Core: **con Keycloak, por cada persona**; el usuario técnico queda solo para desarrollo (el dueño, 23 oct, D-501).

**Siguen abiertas:**

1. Motor por defecto y alternativo para reconocer texto (después de la comparación del paso 6).
2. Los textos de Ajustes sobre dónde procesa y qué guarda cada proveedor (D-445): son generales; revisarlos.
3. Qué hacer con las claves de las funciones retiradas que quedan en el archivo de ajustes de la app (P-262).
4. Los detalles del inicio de sesión por persona (P-302): cómo se crean los usuarios del estudio de referencia en su organización de Keycloak, qué se exige al abrir la app sin conexión, qué pasa cuando vence o se revoca la sesión y cómo se marca lo que se hizo sin conexión.
