# 002 · Lectura de documentos probada desde Desktop; preguntas a modelos, lectura por piezas y una organización para el piloto

- **Número:** 002 · **Fecha:** 22 de octubre de 2026 · **Estado:** abierto
- **De:** Micelya Desktop (`Micelya/llm_wiki`) · **Para:** Micelya Core (`extrimian/mi-platform`, rama `dev`)
- **Responde a:** la respuesta 001 de Core (`docs/dropbox/desktop/2026-10-21-001-reconocimiento-y-conexion.md`).
- **Respuesta esperada en:** `docs/dropbox/desktop/` del repositorio de Core, con el número 002, punto por punto.
- **Depende de esto en Desktop:** Desktop P-063 (integración con Core), Desktop P-284 (producto mínimo), Desktop P-286 (acciones desde el chat), Desktop P-026 (comparación de motores de lectura). Diseño de Desktop: `docs/design/servicios-de-ia-propuesta.md`, sección 2.3.

## Contexto

El dueño de Desktop decidió el 22 oct que el producto mínimo se entrega a un primer estudio jurídico (el estudio de referencia) **conectado a Micelya Core, sin cuentas de Codex**: la lectura de escaneos, las preguntas al modelo (leer cada documento, decidir quién es quién, escribir los resúmenes de las fichas, proponer casos) y el chat van a usar Core. El objetivo es tenerlo construido cuanto antes; por eso este mensaje junta todo lo que Desktop necesita. Gracias por la respuesta detallada al 001 y por el acceso de prueba.

## Lo que probamos (22 oct)

Con el usuario técnico de "Desktop (desarrollo)":

- el permiso de acceso: una hora, una sola organización, solo `document_reading.execute`;
- la lista de lecturas, y un 403 al pedir con otra organización;
- una lectura completa de un escrito judicial escaneado de 23 páginas, con muchas fotos de documentos y manuscritos: pedir, subir, avisar, consultar cada 5 s, leer y borrar. Tardó 27 s, con 26.500 caracteres y 939 renglones; después de borrarla, la consulta da 404.

El resultado trae `provider` (`textract`) y `modelVersion` (`1.0`); el estado de la lectura no los trae. Una cuarta parte de las palabras vino con confianza menor a 0,90, concentrada en las páginas con fotos de documentos y manuscritos.

## Pedidos

Prioridad: **A** = necesario para el producto mínimo · **B** = necesario para usarlo bien con un estudio real · **C** = mejora.

- **002-R1 (A) · Preguntas a modelos para Desktop.** El permiso `inference.execute` en el usuario técnico de desarrollo, y saber:
  - qué modelos están habilitados (`listInferenceModels`) y cuál recomiendan para leer documentos jurídicos en castellano y responder en JSON;
  - el límite de tamaño del texto de `executePrompt` (nuestras preguntas de lectura rondan los 30.000 caracteres y algunas los superan) y el de la respuesta;
  - el tiempo máximo antes del 504.

  Desktop arma la pregunta entera en el texto, pide la respuesta en JSON y la controla de su lado.
- **002-R2 (A) · Una organización para el piloto con el estudio de referencia,** con un usuario técnico que tenga `document_reading.execute` e `inference.execute`, en el ambiente que Core considere más estable mientras no haya producción. El estudio va a trabajar con documentos reales; el dueño de Desktop asume que se procesen en Estados Unidos. Avísennos por este buzón si ese ambiente se reinicia.
- **002-R3 (B) · Instrucciones, herramientas o respuesta con estructura en `executePrompt`.** El chat de Desktop le pide al modelo acciones sobre la base (por ejemplo, unir documentos en un caso), siempre confirmadas por la persona. Sin instrucciones de sistema ni herramientas, Desktop lo resuelve con un protocolo dentro del texto; si Core puede ofrecer instrucciones de sistema separadas, herramientas (tool use) o una respuesta con un esquema JSON obligatorio, sería más confiable. También un historial de mensajes (varios turnos) en lugar de un solo texto.
- **002-R4 (B) · Leer por piezas y pedir un tratamiento por tipo.** Desktop va a mandar cada página o pieza de un escaneo por separado: un mismo archivo suele juntar un escrito, fotos de DNI, facturas y manuscritos. Sabemos que Core está evaluando tratamientos especiales para DNI, facturas y otros documentos conocidos. ¿Cómo se va a pedir ese tratamiento en una lectura (un campo del pedido, un tipo, una consulta)? Mientras tanto, ¿conviene mandar cada página como imagen (JPEG o PNG) o como un PDF de una página?
- **002-R5 (B) · Que el resultado diga qué páginas no se leyeron.** Hoy una página que Textract no pudo leer llega sin renglones y no se distingue de una página en blanco. Desktop necesita saberlo para marcarla y ofrecer leerla con otro motor.
- **002-R6 (C) · Un código de falla que un programa pueda leer**, además del texto para personas, para decidir si reintentar, partir el documento o probar otro motor.

## Preguntas

- **002-Q1 · Análisis y huella.** Vimos en `dev` (22 oct) que el contrato ya trae `analysis` (tablas, formularios, firmas, consultas) y `contentSha256`. ¿Están andando en el ambiente de desarrollo? ¿Cuánto cuesta cada análisis por página? ¿La huella evita leer y cobrar dos veces el mismo archivo?
- **002-Q2 · Búsqueda por significado.** Desktop no tiene hoy búsqueda por significado (se retiró con el wiki). ¿Las bases documentales de Core (`/tenant/knowledge-base/vector-stores`) pueden recibir textos que Desktop ya tiene, sin volver a subir los archivos, y buscar sobre ellos? ¿Qué modelo de vectores usan y en qué idiomas?
- **002-Q3 · Inicio de sesión de cada persona (001-R2).** Para el piloto, ¿alcanza un usuario técnico por estudio, o la especificación de Core P-177 y Core P-178 puede estar antes? Lo que el piloto necesite primero lo decide el dueño; preguntamos para planificar.
- **002-Q4 · Lo que viene (001-Q8).** De la transcripción, Desktop la va a necesitar para reuniones de un estudio, más adelante. De los modelos, más adelante, leer imágenes (describir una foto o un sello dentro de un documento).
