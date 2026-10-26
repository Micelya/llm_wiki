# Modelo de documentos y conocimiento: propuesta del replanteo (26 oct 2026)

**Estado:** borrador para que el dueño lo revise. No es el diseño vigente ni autoriza cambios al programa. Sale de la conversación del 26 de octubre, que siguió a las [notas del replanteo](replanteo-del-proceso-notas.md) del 25. Las decisiones del dueño están en `docs/decisiones.md` (D-530 a D-545); lo que propone el agente y el dueño aceptó lo dice cada una.

El inventario de la base actual y qué pasa con cada tabla está en [base-de-datos-inventario.md](base-de-datos-inventario.md).

## 1. Para qué

**Definir el proceso mediante el cual la aplicación incorpora materiales de distintos orígenes, reconoce y representa fielmente sus documentos y construye, sobre esa base, conocimiento relacionado, verificable y actualizable** (dueño, 26 oct).

Son dos resultados, uno sobre el otro:

1. **Conocer bien el material:** qué documentos hay, dónde aparece cada uno, cómo está organizado su contenido y cómo recuperarlo; copias, versiones, parecidos y plantillas.
2. **Construir conocimiento con él:** personas, organizaciones, asuntos y cosas; cuándo distintas menciones son lo mismo; y las relaciones, respaldadas por los documentos y por el contexto de sus orígenes.

Todo lo que el sistema sostiene se puede rastrear hasta el contenido o el contexto que lo respalda.

**Y la plataforma aprende** (dueño, 26 oct): reconoce clases, relaciones e identificadores nuevos a medida que procesa documentos, con un catálogo que crece. No se entrenan modelos de IA propios.

## 2. Las piezas con identidad

Cuatro clases iniciales. Pueden aparecer clases nuevas sin rehacer la base.

| Clase | Qué es | Ejemplo |
|---|---|---|
| **Documento** | La unidad documental reconocida, esté suelta o dentro de otra cosa. | Una escritura. |
| **Entidad** | Una persona o una organización. Mismo significado en todas las verticales. | Pérez; un estudio de abogados. |
| **Asunto** | Lo que relaciona documentos, entidades y cosas. En la vertical jurídica se dice «caso». Puede tener varios expedientes. Lo crean un portal, la aplicación o el usuario. | Sucesión Pérez. |
| **Cosa** | Algo con existencia material o digital e identidad propia. | Un inmueble, un vehículo, un dominio de Internet. |

- **Un registro común de piezas.** Cada pieza recibe un identificador opaco, único en el mundo (UUID versión 7), en un registro que dice de qué clase es. Sus datos propios van en la tabla de su clase. Las relaciones apuntan a ese identificador.
- **Un URI para cruzar espacios:** `micelya:<espacio>/<clase>/<id>`. Lo usan lo que viene del espacio compartido, del catálogo general o lo que se exporta. Nunca lleva nombres.
- **El dueño de una instancia es una entidad** (en el primer escenario, A&G). Ser cliente es una relación con esa entidad, no una clase de persona.
- **Las etiquetas siguen sirviendo para clasificar y filtrar,** pero ninguna pieza es una etiqueta.

## 3. Documento, versión, aparición y ubicación

| Concepto | Qué es |
|---|---|
| **Documento** | La unidad reconocida. Todo lo que genera la aplicación es único por documento: un resumen, una lectura de personas, una ficha. |
| **Versión** | Una forma distinta del mismo documento: el original y el ejemplar firmado, un borrador y el definitivo. Una versión **prevalece**, y lo generado se basa en ella. |
| **Aparición** | Una presencia concreta de una versión en una fuente. |
| **Ubicación** | Cómo se llega a una aparición. Depende de la clase de fuente: el archivo entero; unas páginas o regiones de un PDF; una entrada de un ZIP; un adjunto de un correo; el identificador del portal. |

- **La versión que prevalece** se elige con reglas del catálogo, en este orden: la firmada sobre la no firmada; la presentada o registrada sobre el borrador; la completa sobre la incompleta; la de fecha posterior. Si no alcanzan, queda dudosa y decide el humano, que siempre puede cambiarla.
- **La evidencia puede citar cualquier versión.** La diferencia entre dos versiones se guarda en la relación entre ellas.
- **Copias en sistemas de archivos** (disco, OneDrive, Google Drive): la aplicación ofrece eliminar las copias; decide el humano; nunca borra sola. Si se borra una copia, queda registrado que esa aparición existió y de dónde vino. **Una aparición dentro de un documento compuesto o de un expediente del portal no se borra:** es parte de otra cosa.
- **Cómo se reconoce el mismo documento:** la prueba en la nube del 26 de octubre (`docs/investigacion/huellas-y-embeddings/informe.md`, en su rama) midió que la huella de la imagen de cada página reconoce el mismo documento aunque no sea idéntico byte por byte, y encuentra el original dentro del ejemplar firmado. Los documentos de una misma plantilla quedan tan cerca como las copias: se separan exigiendo que coincidan los datos clave. La huella de página vale donde hay páginas; no es la forma universal de ubicar.

## 4. El contexto: contención y procedencia

Dos hechos distintos que se guardan por separado. **Los dos aportan contexto valioso para procesar y relacionar** (dueño, 26 oct).

| Hecho | Ejemplos | Qué aporta |
|---|---|---|
| **Contención:** dónde está algo | Un documento compuesto, una carpeta, un ZIP, un correo | Que alguien juntó esas cosas, y en qué orden |
| **Procedencia:** de dónde vino | Un portal judicial, un correo recibido, una carga del usuario, la aplicación que lo generó | Lo que la fuente afirma: «esta actuación es de este expediente, con esta fecha y estas fojas» |

- Un documento puede tener varias de cada una. Los contenedores se anidan: un compuesto dentro de una carpeta; un expediente del portal guardado en una carpeta local.
- Mover o copiar cambia la contención; la procedencia no se pierde.
- **El asunto no es un contenedor:** es una pieza de conocimiento que relaciona otras.
- **Cuánto pesa cada contexto se evalúa en cada relación**, según su evidencia. No hay un nivel de certeza fijo por clase de contenedor (reemplaza en parte la tabla de la sección 2.3 de las notas del 25).
- **Las carpetas no significan nada por sí solas.** El operador puede asociar un nodo del árbol a un asunto o a una entidad; los descendientes heredan el criterio, con excepciones; un nodo puede solo organizar. Decidir qué entra al procesamiento es aparte de interpretar lo que entra. La maquinaria de reglas por carpeta de las etiquetas se reutiliza, pero lo que crea son relaciones con origen «regla de carpeta».

## 5. Las relaciones

Todas con la misma forma:

| Parte | Qué es |
|---|---|
| Sujeto | Una pieza de cualquier clase |
| Relación | Un tipo del catálogo, que dice qué clases puede conectar |
| Objeto | Otra pieza, o un valor (una fecha, un monto) |
| Evidencia | Dónde se apoya: un fragmento con su ubicación, un dato del portal, un orden dentro de un compuesto, una decisión del operador |
| Vigencia | **Desde** y **hasta**, cada extremo con su evidencia y su precisión (día, mes o año). «Hasta desconocido» no es «sigue vigente» |
| Origen | La lectura de un documento, el contexto, una regla o el operador |
| Certeza | Se calcula con la evidencia |
| Estado | Propuesta, confirmada o rechazada |

- **El desde y el hasta son una sola relación.** El documento que trae el fin completa la relación que ya existe. Ejemplo: «Lucía Ferraro trabajó en Talleres Andinos desde marzo de 2021 hasta junio de 2024», con el inicio de un documento y el fin de otro.
- **Relaciones iniciales:** cliente de, participa en un asunto con un rol, titular de, describe a, incluido en, versión de, hecho con la plantilla, más las 27 que ya tiene el catálogo. **Se suman las que se descubran** (sección 8).
- **Lo que hoy está repartido en varias tablas pasa a esta sola:** las afirmaciones entre entidades, los pares de copias y versiones, la pertenencia de un documento a un asunto, los grupos de organizaciones (ver el inventario).

## 6. Certeza, uniones y contradicciones

| Certeza | Qué hace la aplicación |
|---|---|
| Alta | Une o afirma sola, con el porqué registrado, y se puede deshacer |
| Media | La deja dudosa, con la evidencia a favor y en contra, para el humano |
| Baja | No propone nada |

- La aplicación no deja todo al humano ni une sin elementos suficientes.
- **Romper una unión es siempre potestad del humano.** La ruptura queda registrada: la aplicación no vuelve a unir lo mismo y aprende del caso.
- Los umbrales son por clase de decisión y se miden contra una hoja de respuestas.
- **Contradicciones:** la aplicación resuelve las triviales con reglas explícitas (el mismo dato escrito distinto; datos que valen en períodos distintos, que no son contradicción sino cambio). El resto queda abierto, con las dos afirmaciones y su evidencia, para el humano.

## 7. Las piezas y los espacios

Vale la regla de los espacios privado y compartido (D-044): el compartido solo conoce lo compartido; lo que ve una persona es su espacio privado más lo que puede ver del compartido, armado en su PC; una base por espacio; lo privado puede usar lo compartido y no al revés; publicar es un acto explícito del usuario. Una pieza o una relación se ve solo a través de documentos visibles.

## 8. Aprender con el catálogo

**Qué se aprende:**
1. **El vocabulario:** clases, tipos de relación, identificadores (la matrícula de un inmueble), roles.
2. **Las reglas de reconocimiento:** cómo detectar cada cosa.
3. **Los criterios de unión y de versión que prevalece,** ajustados con las decisiones del humano.

**Cada regla dice cómo se aplica:** con código determinista, con IA, o con IA solo cuando el código no alcanza. Se elige lo que logra la misma eficacia o una mejor. Hoy el catálogo ya sirve a las dos: al modelo le da las descripciones; al código, listas de palabras y valores.

**El ciclo:**
1. **Descubrir:** la IA encuentra algo que el catálogo no tiene y lo propone con su evidencia.
2. **Validar:** el humano lo acepta, lo corrige o lo rechaza.
3. **Consolidar:** pasa al catálogo y desde ahí se reconoce en todos los documentos.
4. **Medir:** contra la hoja de respuestas, para comprobar que lo nuevo no empeoró lo anterior.
5. **Reprocesar solo lo afectado:** lo que la aplicación había concluido sola se reemplaza; lo que decidió un humano no se pisa; cada afirmación guarda con qué versión del catálogo y de qué evidencia salió.

Las decisiones del humano en el uso diario alimentan el mismo ciclo.

**Tres capas de catálogo:**

| Capa | Dónde vive | Quién aprueba |
|---|---|---|
| General, por vertical | Viaja con el instalador; opcionalmente se baja actualizado de un servicio de Micelya (la aplicación funciona sin él) | Micelya. En el entrenamiento actual, el dueño |
| Compartida | En la base del espacio compartido | El operador de esa instancia |
| Personal | En la base de la persona | La persona |

La personal ve la general, la compartida y la suya; la compartida ve la general y la suya. Si una regla personal choca con una compartida, la personal vale solo para esa persona y la compartida no cambia (recomendación aceptada).

**Lo que aprende una instancia es de su usuario.** Solo pasa al catálogo general con su consentimiento explícito y anonimizado. Cada cosa aprendida lleva de qué capa salió y si tiene consentimiento.

**Lo que enseñamos nosotros ahora va al catálogo general, anonimizado.** Incluye todo lo de la carpeta de prueba, también el caso de la familia que se usa de ejemplo, aunque sea público en el portal.

**Los ejemplos en el catálogo:** si agregan valor se mide antes de decidir (pendiente). Si lo agregan, van anonimizados de forma coherente (el mismo nombre falso para la misma persona en todos los documentos, números con formato válido), y los documentos que dan ejemplos nunca son los que se usan para medir. Eso pide cambiar la regla de que ni el programa ni el catálogo tengan casos reales (R-24), que se propone aparte.

**Las hojas de respuesta:** un conjunto de documentos con la respuesta correcta escrita. Las arma el agente con las mediciones y las revisa y corrige el dueño (D-101). Sirven para medir, nunca para escribir reglas (R-25). Con el entrenamiento hay que separar siempre lo que enseña de lo que mide.

**El modo entrenamiento:** la misma aplicación y el mismo motor, con un indicador que la lleva a una base propia de entrenamiento (un espacio más). Ahí se cargan expedientes del portal y documentos elegidos; el dueño aprueba lo que se descubre; salen entradas del catálogo general, ejemplos anonimizados y entradas de la hoja de respuestas.

## 9. Lo que sigue abierto

1. **El expediente:** ¿es una pieza con identidad propia (una clase «registro» de un asunto ante un organismo), además de procedencia y contenedor de sus actuaciones? El agente propone que sí.
2. **La plantilla:** ¿es una pieza (una estructura documental) con la relación «hecho con la plantilla»? El agente propone que sí.
3. **Los tipos de cosa iniciales** y sus identificadores.
4. **Qué parte de la propuesta de documentos compuestos (D-525) sigue en pie:** el agente todavía no la revisó contra este modelo.
5. **El rediseño de la base** (ver el inventario): se diseña la base de destino y se migra con respaldo, sin parches.
