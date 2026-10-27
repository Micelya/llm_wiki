# Modelo de documentos y conocimiento: propuesta del replanteo

**Estado:** aprobado por el dueño el 27 oct para construirlo por capas (D-554), con las correcciones del agente local (D-560) y las precisiones del revisor (D-561, asumidas en la corrida desatendida). Antes: borrador del 26 oct, revisado el 27 con el dueño y otro agente. Sale de la conversación del 26 de octubre, que siguió a las [notas del replanteo](replanteo-del-proceso-notas.md) del 25. Las decisiones están en `docs/decisiones.md` (D-530 a D-545 y D-548 a D-553).

Documentos que van con este: el [inventario de la base por significado](base-de-datos-inventario.md) y la [base de destino](base-de-datos-destino.md), con el reinicio, el reproceso y los escenarios de prueba.

## 0. Alcance de la primera etapa

- **Los datos actuales de la base se pueden descartar** (D-548). La base nueva se arma desde cero y se vuelve a procesar. Se conserva el aprendizaje del descubrimiento: decisiones, reglas, ejemplos difíciles, hojas de respuesta y mediciones. Los documentos fuente no se tocan.
- **Solo la dimensión individual** (D-549). La combinación con el espacio compartido, lo que se publica y los permisos sobre conocimiento compartido quedan para su etapa. Los identificadores estables se mantienen.

## 1. Para qué

**Definir el proceso mediante el cual la aplicación incorpora materiales de distintos orígenes, reconoce y representa fielmente sus documentos y construye, sobre esa base, conocimiento relacionado, verificable y actualizable** (dueño, D-530).

1. **Conocer bien el material:** qué documentos hay, dónde aparece cada uno, cómo está organizado su contenido y cómo recuperarlo; copias, versiones, parecidos y plantillas.
2. **Construir conocimiento con él:** personas, organizaciones, asuntos y cosas; cuándo distintas menciones son lo mismo; y las relaciones, respaldadas por los documentos y por el contexto de sus orígenes.

Todo lo que el sistema sostiene se rastrea hasta el contenido o el contexto que lo respalda. Cada dato tiene una responsabilidad clara y una fuente de verdad definida.

**La plataforma aprende** (D-541) con un catálogo que crece, no entrenando modelos propios.

## 2. Las piezas con identidad

| Clase | Qué es | Ejemplo |
|---|---|---|
| **Documento** | La unidad documental reconocida, esté suelta o dentro de otra cosa. | Una escritura. |
| **Entidad** | Una persona o una organización. Mismo significado en todas las verticales. | Pérez; un estudio de abogados. |
| **Asunto** | Lo que relaciona documentos, entidades y cosas. En la vertical jurídica, «caso». Puede tener varios expedientes. Lo crean un portal, la aplicación o el usuario, y puede existir sin documentos. | Sucesión Pérez. |
| **Cosa** | Algo material o digital con identidad propia. | Un inmueble, un vehículo, un dominio de Internet. |

**Dos clases más** (D-559):
- **Expediente:** el registro de un asunto ante un organismo (organismo, número y año). Un asunto puede tener varios. Es además la procedencia y el contenedor de sus actuaciones.
- **Plantilla:** la estructura que comparten documentos distintos, con la relación «hecho con la plantilla». Nace propuesta, con su certeza, y queda firme cuando una persona la confirma o la certeza es alta (D-540).

- **Registro común de piezas** (D-532): cada pieza tiene un identificador opaco (UUID versión 7) y su clase; sus datos van en la tabla de su clase; las relaciones apuntan al registro. El URI `micelya:<espacio>/<clase>/<id>` queda para cuando las piezas crucen espacios.
- **Lo que crece con el uso y lo que no.** Un tipo de cosa, un identificador, un rol o un tipo de relación nuevos son **vocabulario**: entran al catálogo sin cambiar la base. Una **clase fundamental nueva** puede necesitar datos y comportamientos propios: el registro común evita rehacer las relaciones, pero la clase nueva se diseña y se programa. La extensibilidad no es una excusa para guardar datos sin definición.
- **El dueño de la instancia es una entidad** (un estudio de abogados); ser cliente es una relación con ella.
- **Lo que crea el usuario es válido sin documentos:** el dueño de la instancia, un caso cargado antes que sus documentos, un cliente cargado a mano. Se guarda con su procedencia (una decisión humana) y pertenece a la instancia.
- **Las etiquetas clasifican y filtran;** ninguna pieza es una etiqueta.

## 3. Recurso, documento, versión, aparición y ubicación

| Concepto | Qué es | Ejemplo |
|---|---|---|
| **Recurso** | Una secuencia de bytes, por su huella. | `Contrato.docx`; `Contrato.pdf`. |
| **Documento** | La unidad reconocida. | El contrato. |
| **Versión** | Un estado del contenido del documento que cambia algo que importa. | El borrador; el ejemplar firmado. |
| **Representación** | La misma versión en otro formato, sin cambios que importen. | El Word y su PDF con el mismo texto. |
| **Aparición** | Una presencia concreta de una versión en un recurso. | La escritura suelta; la escritura en las páginas 4 a 9 de un compuesto. |
| **Ubicación** | La selección dentro del recurso que recupera la aparición, según la clase de fuente. | El archivo entero; unas páginas o regiones; una entrada de un ZIP; un adjunto; un identificador del portal. |

- **Versión o representación:** dos recursos son la misma versión cuando su contenido coincide en lo que importa (texto, datos clave, firmas); son versiones distintas cuando cambia algo de eso. El Word y su PDF con el mismo texto: una versión, dos representaciones. El original y el firmado: dos versiones (D-537). Hoy la base mezcla todo esto en `content`; la base nueva lo separa.
- **Una ubicación siempre está atada al recurso, por su huella.** Si el archivo cambia, es otro recurso: la ubicación vieja no pasa a señalar otro contenido.
- **Una ficha principal por documento** (D-536, precisado en D-551): un resumen y una ficha. **Las lecturas y la evidencia se guardan por versión**, y cada afirmación sabe de qué versión salió. Si el borrador nombra a un representante y el firmado a otro, se pueden explicar los dos.
- **La versión que prevalece** se usa para: el resumen y la ficha principal, el tipo y los datos que se muestran del documento, y la elección de qué leer primero. **No** se usa para borrar ni trasladar la evidencia de otras versiones. Cuando cambia la versión que prevalece se recalculan el resumen, la ficha y los datos mostrados; la evidencia de cada versión queda.
- **Reglas de la que prevalece** (catálogo): firmada sobre no firmada; presentada o registrada sobre borrador; completa sobre incompleta; posterior sobre anterior. Si no alcanzan, decide el humano, que siempre puede cambiarla.
- **Cómo se reconoce el mismo documento** (D-560, con las pruebas del 26 oct):
  - primero el texto de cada versión, con el método que la app ya tiene (texto canónico, MinHash, la medida exacta sin el texto de rutina y la clasificación de diferencias): en la hoja acierta 53 de 57 pares y reconoce el mismo documento entre Word y PDF;
  - se compara **cada documento incluido por separado**, no el archivo entero: lo que faltaba era partir los compuestos antes de comparar;
  - la huella de la imagen de cada página (pHash) desempata entre «mismo documento» y «versión» y propone candidatos entre escaneos sin texto; **nunca es prueba sola:** en páginas cortas se equivoca;
  - los documentos de una misma plantilla quedan tan cerca como las copias: se separan exigiendo que coincidan los datos clave. **Dos formularios casi idénticos completados para personas distintas no son el mismo documento.**
- **Copias en sistemas de archivos:** la aplicación ofrece eliminarlas y decide el humano. Lo que está dentro de un compuesto o de un expediente no se borra.
- **El documento compuesto es un documento propio** (D-560; D-512, D-527). Su versión aparece en el archivo entero, y la relación «incluye», con su orden, dice qué documentos contiene. Así:
  - tiene su propio resumen, que habla de las relaciones que crea y de por qué existe, no de la suma de lo que incluye;
  - entra entero a un asunto, con sus incluidos; un incluido puede entrar solo a otro asunto;
  - se analiza como un conjunto.
- **El corte** (D-556):
  - cada documento incluido es una aparición «páginas x a y» sobre el recurso del compuesto;
  - cada corte dice quién lo hizo (una regla, el modelo o una persona) y si está propuesto o confirmado;
  - se confirma solo, salvo lo que cambia algo: dos documentos de personas o tipos distintos pegados, uno partido al medio, un corte que contradice lo que anuncia el escrito. Eso lo revisa una persona;
  - lo que se apoya en un corte propuesto: la lectura no espera; las propuestas nuevas (copias, asuntos, plantillas) sí;
  - por ahora se corta por páginas; las regiones de una página (dos fotos de documento en una hoja) quedan para después.
- **El texto según la forma de la página** (D-558, D-522): la capa de texto, con pdfium; los formularios aplanados y las tablas sin líneas, leídos por posición (pares rótulo-valor); lo impreso escaneado, con RapidOCR en la PC. Las tablas con líneas y lo manuscrito, con un proveedor (Core) cuando lo tenga. Docling no entra a Desktop.
- **Lo manuscrito y las firmas** (D-518): se detectan y se marcan por página y, más adelante, por región; no se transcriben solos. Una lectura pedida por una persona queda marcada «sin confirmar».

## 4. Contención y procedencia

| Hecho | Qué responde | Ejemplos |
|---|---|---|
| **Contención** | Qué está dentro de qué ahora, en qué orden | Una carpeta contiene un archivo; un compuesto contiene la escritura en las páginas 4 a 9 |
| **Procedencia** | Cómo se obtuvo algo y qué dijo la fuente | El portal judicial dio esta actuación del expediente X, con fecha y fojas; un correo de tal remitente trajo este adjunto; el usuario lo cargó; la aplicación lo generó desde una plantilla |

- **Las dos sobreviven por separado.** Un documento bajado del portal y guardado en una carpeta: la carpeta dice dónde está (y se actualiza si se mueve); el portal, el expediente y la actuación dicen de dónde vino (y no cambian).
- **El origen** (`origin`) es solo la fuente configurada (una carpeta, un proveedor). La procedencia es un registro propio por recurso o documento, con la fuente, cuándo y los datos que dio.
- Los contenedores se anidan; un documento tiene varias contenciones y varias procedencias.
- **El asunto no es un contenedor:** es una pieza que relaciona otras.
- **El peso del contexto se evalúa en cada relación,** según su evidencia (D-538).
- **Las carpetas no significan nada por sí solas.** El operador puede asociar un nodo a un asunto o una entidad, con herencia y excepciones; eso crea relaciones con origen «regla de carpeta».

## 5. Relaciones, evidencia y decisiones

Tres registros distintos (D-550):

| Registro | Qué es |
|---|---|
| **Relación** | Lo que el sistema sostiene: sujeto, tipo, objeto, contexto, período, certeza y estado actual |
| **Evidencia** | Cada respaldo, con su procedencia: un fragmento de una versión con su ubicación, un dato del portal, un orden dentro de un compuesto, una regla de carpeta. Cada una **apoya o cuestiona** la relación |
| **Decisión** | Lo que hizo una persona: confirmar, rechazar, unir, separar, corregir. Con su motivo y cuándo |

- **La relación se calcula de su evidencia y de las decisiones.** Al reprocesar, la evidencia cambia; las decisiones no. **A qué se ancla cada decisión** (D-560, D-561):
  - sobre una pieza que creó una persona: a su identificador, que no cambia;
  - sobre algo que la aplicación encontró en los documentos (una persona, una unión): a sus menciones;
  - sobre una interpretación de un documento: también a la versión y a la evidencia. Lo marcado como erróneo en el borrador no pasa al firmado;
  - las menciones y las versiones se identifican por la huella de los bytes del recurso, las páginas y el nombre o la cita normalizados, nunca por ids de filas. Hoy las anclas llevan el número de fila del contenido (`decide.rs`), que cambia al rehacer la base o al volver a cortar un compuesto;
  - si al reprocesar el ancla no encuentra nada, **o encuentra más de un destino posible**, la decisión pasa a revisión y no se aplica a una suposición, como ya hace hoy `anchors.rs`. Así una unión rechazada no vuelve.
- **El contexto se conserva explícitamente.** Una relación puede llevar:
  - **dentro de qué pieza vale:** el cargo de director **en** tal organización; la persona representa a una empresa **en** tal asunto;
  - **un rol:** parte, apoderado, testigo;
  - **una calificación:** un domicilio constituido no es el real;
  - **datos del valor:** un monto con su moneda y su concepto.

  Hoy eso lo hacen `context_entity` y `qualifier`; en la base nueva el contexto apunta al registro común, así puede ser un asunto o una organización. Lo que cambia de contexto no es contradicción: una persona directora de una y empleada de otra son dos relaciones.
- **Períodos:** una relación tiene **desde** y **hasta**, cada uno con su evidencia y su precisión. «Hasta desconocido» no es «sigue vigente». **El mismo tipo de relación entre las mismas partes puede darse en varios períodos:** quien sale de una organización y vuelve a entrar tiene dos relaciones de empleo. Un fin completa la relación cuyo período encaja; un inicio posterior a un fin conocido abre otra.
- **Tipos de relación:** cliente de, participa en un asunto con un rol, titular de, describe a, incluido en, versión de, hecho con la plantilla, más los que ya tiene el catálogo. Cada tipo dice qué clases conecta y qué datos pide. Se suman los que se descubran.
- **Cuándo dos evidencias hablan del mismo hecho** (D-561): lo declara cada tipo de relación en el catálogo, no el programa:
  - **qué datos lo identifican.** Por ejemplo, en «participa en un asunto», la persona, el asunto y el rol: la misma persona con dos roles son dos participaciones;
  - **si admite un valor o varios a la vez.** Con uno solo (el número de documento, la fecha de nacimiento), un valor distinto es una contradicción sobre el mismo hecho; con varios (los roles, los cargos), es otro hecho.

  Ya hay un caso así: el mismo expediente se reconoce por número y año (D-526).

## 6. Certeza, uniones y contradicciones

| Certeza | Qué hace la aplicación |
|---|---|
| Alta | Une o afirma sola, con el porqué registrado, y se puede deshacer |
| Media | La deja dudosa, con la evidencia a favor y en contra |
| Baja | No propone nada |

- Romper una unión es siempre potestad del humano, y se recuerda.
- Umbrales por clase de decisión, medidos contra la hoja de respuestas.
- **Contradicciones:** las triviales se resuelven con reglas (el mismo dato escrito distinto; contextos o períodos distintos, que no son contradicción). El resto queda abierto, con las dos afirmaciones y su evidencia.

## 7. Visibilidad

En la dimensión individual todo lo de la base es visible para su usuario, incluido lo que creó sin documentos. **Para la etapa compartida (diferida):** vale la regla de los espacios (D-044) y habrá que definir cómo se combinan referencias y permisos y cómo se evita que una conclusión revele datos de fuentes que el usuario no puede ver.

## 8. Aprender con el catálogo

**El catálogo expresa conocimiento configurable; los motores lo ejecutan.** Sirve al LLM y al código determinista:

| Entrada del catálogo | El LLM | El código |
|---|---|---|
| Tipo de documento | Lo reconoce e interpreta | Valida tipo y campos requeridos |
| Tipo de identificador | Lo encuentra en texto o imagen | Lo normaliza y valida su formato |
| Tipo de relación | Reconoce el vínculo | Verifica clases y datos requeridos |
| Criterio de identidad | Evalúa lo ambiguo | Compara identificadores y aplica condiciones |

Ejemplo: el LLM encuentra una matrícula inmobiliaria; el código la normaliza, valida su estructura y busca el inmueble ya reconocido. **Una entrada nueva usa capacidades existentes; una capacidad nueva (leer un formato nuevo) hay que programarla.**

**El ciclo:** descubrir (la IA propone lo que el catálogo no tiene, con evidencia) → validar (el humano) → consolidar (entra al catálogo) → medir (contra la hoja de respuestas) → reprocesar lo afectado.

**Qué se guarda de lo aprendido:** cada entrada con su revisión, su procedencia (de qué documentos o decisiones salió), su estado (propuesta, aprobada, corregida, rechazada) y quién la aprobó. Cada resultado guarda la revisión del catálogo y de las instrucciones, y el motor, con que se produjo.

**Reprocesar, en la primera etapa, por documento y por etapa** (D-552). Cuando cambia una entrada del catálogo se recalcula la etapa que la usa (lectura, resolución de identidad, relaciones, tipos) en los documentos donde aplica (por tipo de documento, o todos si no se puede acotar). No se promete una selección más fina. Lo que decidió un humano no se pisa.

**Lo que cambia se propaga a las etapas siguientes** (D-561). Las etapas tienen un orden fijo: texto → páginas → documentos → lectura → identidad → relaciones → asuntos → resúmenes. Cuando una etapa cambia un resultado, se marcan para recalcular las que dependen de él, con un alcance conservador: si cambia quién es una persona, todos los documentos y páginas donde aparece, aunque vengan de otros documentos. La cola acepta documentos y piezas (por ejemplo, la página de una persona). Con la caché de respuestas (D-555), recalcular le vuelve a preguntar al modelo solo cuando la pregunta cambió.

**Los vectores** (D-557): la búsqueda por significado y los vecinos para proponer parecidos usan vectores por versión, que calcula un proveedor (Core, en AWS) cuando esté. Desktop no los calcula en la PC; sin proveedor, la búsqueda es por palabras.

**Dos fuentes de catálogo en esta etapa** (D-549): el que viene con la aplicación (por vertical, con el instalador y opcionalmente actualizable desde un servicio de Micelya) y lo que agrega o ajusta la instancia, que es de su usuario (D-543). La capa compartida queda para su etapa.

**Ejemplos:** si agregan valor se mide antes de decidir (D-544). Lo que enseña se separa siempre de lo que mide.

**Las hojas de respuesta:** un conjunto de documentos con la respuesta correcta escrita; las arma el agente con las mediciones y las corrige el dueño (D-101); miden, no enseñan (R-25).

**El modo entrenamiento** (D-545): la misma aplicación y el mismo motor, con una base propia de entrenamiento.

## 9. Lo que sigue abierto

1. **Los tipos de cosa iniciales** y sus identificadores.
2. **Las regiones de una página** (dos documentos en una hoja): después del corte por páginas.
3. **Lo que propone el compuesto como conjunto:** las relaciones entre personas que salen del conjunto y el control contra lo que anuncia el escrito (preguntas 9 y 10 de la propuesta de compuestos, D-525). Van con la etapa de relaciones.

Resuelto el 27 oct: el expediente y la plantilla como clases propias (D-559); el encaje de la propuesta de compuestos (D-556, D-560).
