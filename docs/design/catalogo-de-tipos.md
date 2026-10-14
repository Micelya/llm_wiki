# Catálogo de tipos de documento — primera versión

**Estado:** borrador · 14 de octubre de 2026 · nada de esto está construido
**Qué es:** el conocimiento que la app va a usar para reconocer qué es cada documento y en qué etapa está. Es la sección 5 bis de la [propuesta de parecidos](parecidos-propuesta.md), con contenido.

## Cómo se lee y cómo crece

- **Cada entrada describe un tipo de documento en términos generales:** cómo se reconoce, en qué etapas puede estar, qué datos interesan, con qué otros documentos suele aparecer y cómo se compara. Vale para cualquier documento de ese tipo, de cualquier estudio.
- **Nunca nombra un caso:** ni personas, ni empresas que no sean las que emiten el formulario (un correo, un servicio de firma), ni números concretos. Lo que se sabe de un caso puntual ("estos dos documentos de tal persona son asuntos distintos") no es conocimiento del catálogo: es la decisión de una persona sobre ese par, y se guarda como dato junto a ese par ([reglas](reglas-de-procesamiento.md), R-24).
- **"Señales" son lo que se verifica sin modelo,** leyendo el texto o la estructura del archivo. Cuando no alcanzan, el modelo elige el tipo de esta lista.
- **Crece con el uso:** un tipo nuevo, o una variante nueva de uno existente (otro correo, otro servicio de firma), se agrega como una entrada más. No requiere cambiar el programa.
- **Cada entrada dice sobre qué se verificó.** Esta primera versión sale de la carpeta de prueba.

## Carta documento

**Qué es:** comunicación fehaciente enviada por un servicio postal habilitado, con constancia de texto, fecha y entrega.

**Etapas:**

| Etapa | Qué es | Cómo se reconoce |
|---|---|---|
| Borrador | El texto escrito en un procesador de texto, antes de cargarlo en el sistema del correo | Archivo de procesador de texto; no tiene formulario, número de envío ni sellos |
| Vista previa | Lo que genera el sistema del correo antes de despacharla. Sin valor legal | Formulario del correo con la marca de agua "VISTA PREVIA - SIN VALOR LEGAL"; en lugar del número de envío y el código de barras, casilleros vacíos |
| Enviada | La copia que queda del lado de quien la envió, ya procesada | Formulario completo con número de envío y código de barras; leyenda de copia fiel del original |
| Recibida | La copia que llega al destinatario | Formulario con sello del correo fechado y firma del remitente; marca de copia para el destinatario |

**Variantes por correo** (verificadas en la carpeta de prueba):

| Señal | Correo Argentino | OCA |
|---|---|---|
| Encabezado | Logo amarillo "Correo Argentino"; título "CARTA DOCUMENTO" y, arriba, "A.R. - CARTA DOCUMENTO" (el aviso de recibo) | Logo "OCA cartadocumento"; título "CARTA DOCUMENTO OCA" |
| Número de envío | Dos letras, nueve dígitos y "AR" (por ejemplo, la forma `HD000000000AR`) | Letra, cinco dígitos, dos letras y tres dígitos (la forma `C00000AA000`) |
| Remitente y destinatario | Casilleros "REMITENTE" y "DESTINATARIO" en horizontal | Rótulos "REMITENTE" y "DESTINATARIO" en vertical, al costado |
| Sello | Sello circular "Correo Oficial de la República Argentina S.A. – Sector Carta Documento", con fecha cuando se procesó | Sello circular "OCA C.C.D." con fecha |
| Firma | Leyenda "La identidad del firmante está validada ante RENAPER" (envío electrónico) | Firma manuscrita, aclaración y DNI del remitente |
| Otras marcas | "LA PRESENTE ES COPIA FIEL DEL ORIGINAL"; fondo de líneas amarillas | "Copia Destinatario" en el margen |

**Datos que interesan:** remitente, destinatario, fecha, número de envío, a qué responde (fecha o número de envío de la carta anterior), qué reclama o rechaza (facturas, importes, plazos).

**Suele aparecer con:** su borrador, su vista previa, su acuse de recibo, la carta que responde y la que la responde.

**Cómo se compara:**

- Dos cartas que comparten el formulario y fórmulas legales pero reclaman cosas distintas son **documentos distintos**; el formulario no cuenta como parecido.
- Si cambia un dato (número de factura, fecha, importe), son documentos distintos aunque compartan casi todo el texto.
- **"Respuesta a"** se reconoce cuando una carta cita la fecha o el número de envío de otra.

## Acuse de recibo de carta documento

**Qué es:** la constancia del correo de que la carta llegó a su destinatario.

**Señales:** la parte "A.R." del formulario (Correo Argentino) con el recuadro "RECIBÍ CONFORME EL ENVÍO REFERENTE A ESTE AVISO" completo: fecha, hora y aclaración de quien recibió.

**Datos que interesan:** número de envío, fecha y hora de entrega, quién recibió.

**Se relaciona con:** la carta de ese número de envío ("acuse de recibo de").

## PDF emitido por un servicio de firma electrónica

**Qué es:** un documento cargado en un servicio de firma electrónica, en cualquier etapa: enviado a firmar, firmado por una parte, firmado por todas.

**Señales:**

- Una firma digital incrustada en el PDF, que **el servicio pone en cada versión que emite**. No indica por sí sola que alguna parte haya firmado.
- En las páginas, un identificador del proceso de firma repetido en el margen ("Envelope Id", en uno de los servicios de la carpeta de prueba).
- Al final, una página de historial o de auditoría ("Historial", "Final audit report", "Firmado electrónicamente", "Transaction ID", en el otro servicio).
- Los PDF de uno de los servicios vienen cifrados con permisos: se leen sin contraseña.

**Etapas:** sin firmar, firmado por una parte, firmado por todas. Se distinguen leyendo la página de historial (quién firmó y cuándo) o el certificado de cada firma; no alcanza con contar firmas.

**Cómo se compara:** el ejemplar firmado contiene entero al documento sin firmar y le suma la página de historial. Esa página no cuenta como diferencia de contenido.

## Constancia de firma

**Qué es:** el resumen que emite un servicio de firma electrónica sobre un proceso de firma: qué documento, quiénes firmaron, cuándo.

**Señales:** títulos como "Certificate Of Completion", "Envelope Summary Events", "Signer Events", "Record Tracking" (en uno de los servicios de la carpeta de prueba).

**Datos que interesan:** el documento firmado, las partes, las fechas.

**Se relaciona con:** el documento cuyo proceso resume ("constancia de firma de").

**Cómo se compara:** dos constancias del mismo servicio comparten la plantilla y son documentos distintos.

## Formulario PDF completable

**Qué es:** un PDF con casilleros donde se escriben las respuestas.

**Señales:** el PDF trae campos de formulario. La página tiene la plantilla; los campos, los datos.

**Datos que interesan:** el contenido de cada campo, que la base ya guarda aparte (PR #38).

**Cómo se compara:** dos formularios con la misma página y distintos campos son **misma plantilla, otros datos**. Un formulario guardado "aplanado" (con las respuestas impresas en la página) no trae campos: se compara por texto.
