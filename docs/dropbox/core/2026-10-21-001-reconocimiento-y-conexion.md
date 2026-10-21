# 001 · Reconocimiento de texto, conexión desde la app de escritorio y lo que Desktop necesita de Core

- **Número:** 001 · **Fecha:** 21 de octubre de 2026 · **Estado:** abierto
- **De:** Micelya Desktop (`Micelya/llm_wiki`) · **Para:** Micelya Core (`extrimian/mi-platform`, rama `dev`)
- **Respuesta esperada en:** `docs/dropbox/desktop/` del repositorio de Core, con el número 001, punto por punto.
- **Depende de esto en Desktop:** Desktop P-242 (la base reconoce sola el texto de escaneos e imágenes), Desktop P-141 (servicios de IA configurables), Desktop P-026 (Textract), Desktop P-063 (integración con Core). Diseño de Desktop: `docs/design/servicios-de-ia-propuesta.md`.

## Contexto

Desktop es una base documental de escritorio para estudios jurídicos: inventaría las carpetas del estudio, obtiene el texto de cada documento una vez, detecta copias y versiones, y con un modelo extrae personas, organizaciones, afirmaciones con su cita y asuntos. Todo vive en una base local por usuario.

Desktop va a usar a Core como **un proveedor más** de sus servicios de IA, elegible por servicio en Ajustes, junto a motores locales o propios (Codex, Claude Code, APIs de proveedores). Desktop tiene que poder funcionar sin Core.

El primer servicio a integrar es la **lectura de documentos (OCR)**: en Desktop, los escaneos y fotos quedan hoy sin leer. Leímos `dev` hasta el commit del 20 de octubre (PR #65). Entendimos que la lectura de documentos con Textract está construida y probada en el ambiente de desarrollo (pedir, subir a S3, avisar, consultar, leer el resultado; texto, renglones y palabras con confianza y ubicación). Si algo de lo que sigue ya existe o está mal entendido, por favor corríjannos.

## Pedidos (lo que necesitamos que Core implemente)

Prioridad: **A** = necesario para empezar · **B** = necesario para usarlo con un estudio real · **C** = mejora.

- **001-R1 (A) · Acceso para una app de escritorio de desarrollo.** Una organización (tenant) de prueba con un usuario técnico con el permiso de lectura de documentos, para desarrollar y probar el motor de Core en Desktop. Indíquennos cómo pedir la credencial y la dirección de la API y de Keycloak del ambiente de desarrollo.
- **001-R2 (B) · Inicio de sesión de cada persona desde la app de escritorio.** Un cliente de Keycloak para escritorio (inicio de sesión en el navegador que vuelve a la app, con código de autorización y PKCE, sin secreto embebido), con permisos por persona y tokens que se renuevan y se pueden revocar. Hoy solo existe el usuario técnico, con un secreto que no vence: copiado en cada PC de un estudio no permite saber quién hizo qué ni revocar el acceso de una persona (relacionado con Core P-089).
- **001-R3 (A) · El resultado dice qué proveedor y qué versión leyeron el documento.** Desktop guarda junto a cada texto qué motor lo produjo, para comparar motores, volver a leer con otro y explicar de dónde sale un dato.
- **001-R4 (B) · Pedidos repetidos.** Una clave de idempotencia en el pedido (o reconocer el mismo archivo por su huella SHA-256 dentro de la organización), para que un reintento de Desktop no lea ni cobre dos veces.
- **001-R5 (B) · No guardar el resultado en Core si el cliente lo pide.** Una opción en el pedido para que Core entregue el texto y lo borre enseguida (Desktop lo guarda localmente). Hoy queda en la base de la organización hasta que alguien lo borre. Necesitamos además saber con precisión qué conserva Textract (entendimos: el resultado, 7 días, en us-west-2, sin forma de borrarlo antes).
- **001-R6 (B) · Opciones de análisis en la lectura.** Que el pedido pueda indicar, además del texto: **formularios** con el estado de cada casilla (marcada o no) y los pares rótulo–valor, **tablas**, **firmas** (dónde hay una firma en la página) y, si es posible, **preguntas** (Textract Queries). Las casillas son un problema concreto de Desktop: en las fichas de cliente escaneadas el texto trae los rótulos ("SI / NO") pero no cuál está marcada, y eso lleva a datos falsos. Entendemos que está propuesto como Core P-161.
- **001-R7 (B) · Proveedor elegible por pedido.** Hoy el proveedor se elige por ambiente. Necesitamos poder pedir otro proveedor para casos que Textract no resuelve, sobre todo **manuscrito en castellano** (Textract lo lee solo en inglés): por ejemplo, un modelo que lea imágenes en Bedrock, con la misma forma de pedido y de resultado.
- **001-R8 (C) · Lectura inmediata de una imagen.** Para una sola página, una variante sin espera (Textract tiene lectura sincrónica de una página). Hoy todo pasa por el flujo asincrónico, de 9,5 a 25 segundos por página según las pruebas de Core.
- **001-R9 (C) · Aviso de fin en lugar de consultar.** Si Core prevé avisar cuando termina una lectura (por ejemplo, un evento o un canal de notificaciones), Desktop lo usaría en lugar de consultar cada pocos segundos.
- **001-R10 (C) · Uso y costo por organización.** Una consulta de cuánto se usó y cuánto costó cada servicio, para mostrarlo en Desktop antes de leer un lote grande.
- **001-R11 (C) · Inferencia con imágenes.** Para describir imágenes dentro de documentos (Desktop P-173) y para el manuscrito (ver R7).

## Preguntas (lo que necesitamos aclarar)

- **001-Q1 · Ambiente y estabilidad.** ¿Cuándo prevén un ambiente estable para probar con un estudio real? ¿Qué podemos dar por estable hoy del recurso de lectura de documentos?
- **001-Q2 · Contrato.** ¿Podemos generar los tipos de Desktop directamente desde `packages/contracts/src/http/openapi.yaml`? ¿Cómo nos enteramos de un cambio que nos afecte (por este buzón, por la versión del contrato)? Desde Desktop vamos a avisar acá antes de depender de un campo nuevo.
- **001-Q3 · Límites.** ¿Los límites actuales (imágenes 10 MB; PDF y TIFF 50 MB y 100 páginas; espera máxima 30 minutos) son definitivos? ¿Qué devuelve Core si un PDF supera las 100 páginas: error al final o se puede saber antes?
- **001-Q4 · Región y datos personales.** Textract no tiene región en Sudamérica. ¿Hay plan para procesar en otra región o con otro proveedor cuando un estudio no pueda sacar documentos del país? ¿Cómo queda documentado para el cliente del estudio?
- **001-Q5 · Uso de datos por terceros.** ¿Está resuelto que los proveedores no usen los documentos para entrenar (Core P-167)?
- **001-Q6 · Cupos.** ¿Hay límites de pedidos por organización o por minuto? ¿Qué responde Core cuando se superan (para que Desktop espere y reintente)?
- **001-Q7 · Confianza.** ¿La confianza de cada renglón y palabra es la de Textract sin cambios? ¿Hay un umbral recomendado para marcar texto dudoso?
- **001-Q8 · Lo que viene.** ¿Qué orden tienen embeddings o vectores, búsqueda con citas, transcripción para Desktop, agentes y MCP? Desktop planea rehacer su acceso para otros asistentes (MCP) y quisiera no duplicar lo que Core vaya a ofrecer.
- **001-Q9 · Identificadores.** ¿El identificador de una lectura es estable y se puede guardar en Desktop para volver a pedir el resultado mientras exista?

## Lo que Desktop va a hacer mientras tanto

1. Un registro de servicios y motores configurable en Ajustes (un motor por servicio, conexiones compartidas, secretos en el llavero del sistema).
2. El reconocimiento de texto en la base con el motor que ya tiene (Codex), para no depender del avance de Core.
3. Un Core simulado para sus pruebas, con los tipos del contrato de Core.
4. El motor de Core en cuanto tengamos R1 y las respuestas Q1 y Q2.
