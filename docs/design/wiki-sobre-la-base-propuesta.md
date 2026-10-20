# El wiki como presentación de la base — propuesta

**Estado:** aprobada con ajustes el 18 de octubre de 2026; en construcción (rama `feat/wiki-on-docbase`, P-142) · propuesta del 17 de octubre de 2026
**Ajustes del dueño (18 oct, D-293 a D-296):** certeza y quién confirmó cada afirmación ("con cita comprobada", no "verificada"); el grafo ordena y amplía, no recorta; una extracción principal por contenido y configuración; página común solo para versiones del mismo documento; el asunto no depende de cuántos documentos tiene; prioridad a los errores críticos con mejora demostrada. Las decisiones abiertas quedaron resueltas (sección 11).
**Alcance:** qué partes heredadas del wiki original se conservan, cuáles se rehacen sobre la base documental y la capa de conocimiento, y cuáles se quitan, para que el chat, la búsqueda y el grafo sigan funcionando sin redundancias y sin usos del modelo que no hacen falta. No describe código.

Cuando esta propuesta esté acordada, lo que quede firme pasa al [documento de diseño](base-documental.md).

## 1. El problema

El wiki se heredó de un proyecto pensado para investigación personal: el modelo lee cada documento y escribe páginas de fuentes, de entidades, de conceptos y de consultas, y las relaciones son enlaces escritos en esas páginas. Desde entonces se construyeron la base documental (qué documentos hay, cuáles son el mismo, de qué tipo son, su texto) y la capa de conocimiento (entidades, afirmaciones con cita, asuntos, relaciones con tipo). Hoy las dos capas hacen buena parte del mismo trabajo, y la del wiki lo hace con menos rigor.

Lo medido sobre el wiki generado el 17 de octubre (carpeta de prueba, 66 documentos):

| Medida | Valor |
|---|---|
| Lecturas del modelo por documento | El wiki lo lee 2 o 3 veces (unas 200 llamadas para 66 documentos) y la capa de conocimiento lo vuelve a leer entero |
| Enlaces entre páginas | 3.232; el 81 % los escribe el programa a partir de la base, el 19 % el modelo dentro del texto |
| Relaciones que ya tiene la base | 459 entidad–documento con rol; 100 entre entidades con tipo y cita; 47 asunto–documento |
| Enlaces rotos | 0 hoy, porque un control convierte en texto los que no llevan a nada; 30 a 42 en el wiki anterior, todos escritos por el modelo |
| Conceptos | 85 páginas, 56 salidas de un solo documento (cláusulas de un contrato) |
| Páginas de fuente | Una por cada copia o versión que nadie unió (por ejemplo, dos versiones de una misma cesión) |
| Búsqueda | Recorre y lee todos los archivos del wiki en cada consulta; no usa el índice de texto que la base ya tiene sobre el texto de cada documento |

## 2. Lo que se conserva

- **El chat, la búsqueda y el grafo.** Son valiosos para un estudio. Cambia de dónde toman la información, no lo que ofrecen.
- **Las páginas como forma de leer.** Una persona lee un documento, una persona, una organización o un asunto como una página con forma de artículo (sección 13 del diseño).
- **La síntesis en prosa** de lo que dicen los documentos, escrita por el modelo solo desde afirmaciones **con cita comprobada**. Que la cita exista en el documento no prueba que la lectura sea correcta: cada afirmación lleva su **certeza** y **quién la confirmó** (el modelo, una regla, una persona), y la síntesis escribe lo dudoso como dudoso. Una corrección hecha una vez se propaga a todas las páginas, porque se arman desde el registro (D-293). Construido en N2b (D-320 a D-322): cuatro niveles (con cita comprobada, confirmada, dudosa con su motivo, corregida), la síntesis escribe lo dudoso como dudoso y un cálculo lo marca si no lo hace, y las decisiones de una persona se vuelven a colocar después de cada relectura o quedan para revisar en Atención.
- **Las notas de una persona** en cada página, que la app nunca reescribe.

## 3. Principios

1. **Una sola fuente de verdad por cada cosa.** Lo que la base sabe (documentos, copias, versiones, tipo, datos, entidades, relaciones, asuntos) se guarda solo en la base. El wiki lo muestra; no lo vuelve a calcular ni lo guarda de otra forma.
2. **Una extracción principal por contenido y configuración.** "Una lectura" quiere decir no repetir trabajo: de esa extracción salen las menciones, las afirmaciones, los datos y el resumen del documento, y todo lo demás la reutiliza. Nada vuelve a leer un documento con el modelo salvo que cambie su texto o la configuración (modelo, pregunta, vocabulario). Una comprobación extra con el modelo (por ejemplo, de importes o de atribución) se suma solo si su mejora está medida contra las hojas de respuestas (D-293). Las decisiones de una persona (unir, separar, dato incorrecto) sobreviven a una relectura.
3. **Una página por documento, no por copia ni por versión.** Las copias y las versiones unidas son una sola página, que las lista.
4. **Las relaciones viven en la base, con tipo, cita y estado.** Los enlaces de una página se arman al mostrarla, a partir de esas relaciones.
5. **El modelo, donde agrega valor** (D-150): leer, resolver lo dudoso y redactar. No para recalcular lo que la base ya sabe.

## 4. Qué pasa con cada proceso heredado

| Proceso de hoy | Qué hace | Propuesta |
|---|---|---|
| Análisis y generación del wiki por documento (2 o 3 llamadas) | Lee el documento y escribe su página, conceptos y avisos | **Se reemplaza** por la extracción principal de la capa (principio 2), que suma el resumen del documento como un derivado más |
| Lectura de la capa de conocimiento | Menciones, afirmaciones, datos | **Se conserva** y pasa a ser la extracción principal |
| Página de fuente escrita por el modelo | Resumen con enlaces | **Se rehace** como página del documento armada desde la base (sección 5) |
| Conceptos por documento | Una página por cláusula de un contrato | **Se quitan.** Vuelven, si la medición lo justifica, como conceptos propuestos por cálculo (sección 8) |
| Consultas que el modelo inventa al incorporar | "Preguntas abiertas" sobre lo que falta | **Se quitan.** Lo que falta es un aviso de la base en Atención (R-23). Se conserva guardar en el wiki una respuesta del chat, a pedido de una persona |
| Avisos de revisión del wiki (contradicciones, duplicados, páginas que faltan) y su limpieza con el modelo | Avisos de texto libre | **Se quitan.** Las contradicciones y los duplicados ya los detecta la capa con evidencia y van a Atención |
| Combinación de páginas con el modelo | Une una página existente con una nueva | **Se quita** para las páginas que salen de la base; queda solo si una persona escribió la página a mano |
| Unión de duplicados de Ajustes | Ya convertida en decisiones del registro | Se conserva así |
| Búsqueda en internet e investigación web | Trae información externa | **Fuera del wiki del archivo** (contradice "sin conocimiento externo"): queda como herramienta aparte, a pedido, que nunca escribe en la base ni en el wiki; por ahora solo se desconecta de la generación del wiki (D-294) |
| Revisión de enlaces | Estructural sin modelo y, opcional, con el modelo | Se conserva la estructural para las notas; **se quita** la del modelo |
| Reconocimiento de texto de imágenes | Una vez por documento; el texto vuelve a la base | Se conserva |
| Síntesis de entidades y resumen general | Una pregunta por entidad tocada; resumen sin modelo | Se conserva |
| Representación por significado (hoy apagada) | Sobre las páginas del wiki | **Se mueve a la base**, sobre el texto de los documentos (sección 7) |

**Construido en N3 (D-330 a D-335):** la lectura de la capa (consigna `conocimiento-4`) da también el resumen del documento (tema y puntos, cada punto con su cita comprobada y sus afirmaciones del mismo pasaje), guardado en la base; la página de cada documento la escribe la base desde ese resumen y los importes del registro, sin modelo, hasta que N4 la arme como página del documento. En el wiki del archivo la incorporación ya no llama al modelo: no hay análisis, generación, revisión, conceptos por documento, consultas inventadas, avisos de revisión ni combinación de páginas; la limpieza de avisos con el modelo no corre allí, la revisión con el modelo se quitó y la investigación en internet guarda fuera del wiki. Llamadas del modelo para leer un documento: de 3 a 4 por documento a una lectura (75 preguntas para 66 documentos, en partes los largos). La descripción de imágenes con el modelo tampoco corre en el wiki del archivo (P-173); los demás proyectos conservan la incorporación anterior (P-172).

## 5. Las páginas

- **Documento.** Una por documento o familia de versiones. La página común es **solo para versiones del mismo documento** (borrador, ejemplar firmado, vista previa, copias); documentos distintos hechos con la misma plantilla son páginas distintas. Cada afirmación dice de qué versión sale, y si las versiones dicen cosas distintas se muestran las dos (D-293). Por ahora es un **archivo generado**, como las de entidad: regenerable; pasa a armarse al mostrarse cuando la búsqueda y el grafo lean de la base (D-294). Arriba, lo que sabe la base: tipo y etapa, fecha, partes con su rol (enlazadas a sus páginas), asunto, copias y ubicaciones, versiones y su relación (ejemplar firmado, vista previa…), datos del tipo. Abajo, el resumen en prosa que salió de la extracción principal, con referencias a las afirmaciones. Notas protegidas. **Construido en N4 (D-340 a D-342):** la proyección escribe la página de cada documento haya o no página previa; las versiones (relaciones de `pages.versionRelations`, propuestas o confirmadas, nunca descartadas) comparten página, nombrada por la versión elegida o el ejemplar firmado; cada punto, dato e importe que no está en todas las versiones dice de cuáles sale; los documentos relacionados que no son versiones (misma plantilla, contenido en, constancia de firma) se enlazan; las páginas viejas de copias y versiones se absorben con sus notas y sus enlaces; todo lo que no es notas se reescribe desde la base. Sobre la carpeta de prueba: 66 páginas → 59 (6 con versiones), 18 de 19 pares de «mismo documento» de la hoja juntos, 0 pares distintos unidos, 0 páginas con texto que no sale de la base.
- **Persona y organización.** La que ya proyecta la capa, unificada con la pantalla Personas y organizaciones: una sola página con forma de artículo y las acciones de corrección (unir, separar, dato incorrecto) en una pestaña.
- **Asunto.** Junta documentos distintos (no versiones de uno, que son una familia del documento). Se muestra como lo que es, el caso, no como un tipo de entidad. **No depende de cuántos documentos tiene:** la regla de al menos dos documentos vale solo para la agrupación automática; un asunto que crea una persona puede tener uno solo. Hay **un solo concepto de asunto** (P-061) con tres formas de nacer: lo propone la extracción, lo crea una persona (la etiqueta "Caso" es una forma de crearlo) o lo propone una regla de carpeta. No se construye en esta etapa (D-293, D-294).
  **Actualización del 20 oct (D-373 a D-376):** el asunto nace solo de dos maneras, lo crea una persona o lo propone el sistema (ya no por regla de carpeta), y lo propuesto espera la confirmación de una persona. Cada página del wiki se unifica con su pantalla de la base: arriba el artículo, en una pestaña los datos y las correcciones. Ver [Asuntos: un solo concepto](asuntos-propuesta.md).
- **Grupo.** Las sociedades que lo forman y lo que no se puede atribuir a una sola.
- **Índice y resumen general.** Desde la base, sin modelo, como ya se hace.
- **Consultas guardadas.** Solo las que una persona guarda desde el chat.

## 6. El grafo y los enlaces

- **El grafo se arma desde la base:** nodos de documentos, entidades, asuntos y grupos; aristas con su tipo ("debe a", "representa a", "parte en", "aparece en", "versión de"…), su cita y su estado (propuesta, confirmada). Hoy el grafo lee los enlaces de los archivos y convierte todas las relaciones en la misma flecha sin tipo.
- **Los enlaces de una página se generan al mostrarla,** a partir de las relaciones de la base. Un enlace existe solo si la relación existe: no hay enlaces rotos posibles. Los enlaces que el modelo escriba en un resumen se reescriben desde el registro (como ya se hace).
- **Las notas de una persona** pueden tener enlaces escritos a mano; se resuelven contra la base al mostrarse.
- **Esto reemplaza D-076** (las páginas son la verdad y la base lleva un índice reconstruible) para todo lo que sale de la base. El motivo de D-076 era que el modelo escribía todas las páginas como texto; eso ya no es así. Resuelve también P-068.

**Construido en N5 (D-351):** el grafo del wiki del archivo sale de la base (`docbase_graph`): nodos = las páginas que escribe la base; aristas = documento y entidades que nombra (por rol), lo que dicen documentos visibles entre dos entidades (por predicado, con su certeza), documentos que relaciona el cálculo de parecidos (nunca los descartados), asunto y documentos, grupo y sociedades, cada una con tipo, etiqueta del vocabulario, cita y estado. El panel de enlaces de una página también sale de esas relaciones; los enlaces escritos en las notas se resuelven por la página que nombran o por un nombre que conoce el registro. Los demás proyectos siguen con los enlaces de sus páginas. Falta dibujar en la vista el tipo y la cita de cada arista (P-190) y pasar a la base lo que todavía lee los archivos, como el lint (P-192).

## 7. La búsqueda

Combina tres fuentes, todas en la base:

1. **Índice de texto de SQLite**, que la base ya tiene sobre el texto de cada documento: palabras exactas, nombres, números, identificadores.
2. **Vectores** de cada fragmento del texto (párrafo o cláusula), calculados una vez por documento y por espacio como un derivado más, incrementales, en un índice reconstruible (LanceDB, sección 9 del diseño): lo dicho con otras palabras.
3. **El grafo**, para **ordenar y ampliar, nunca para recortar** (D-293):
   - **Ordenar.** Una pregunta que nombra entidades o un asunto sube en el orden los documentos que el grafo relaciona con ellos. La búsqueda de texto (y de vectores, cuando estén) **siempre recorre todo lo visible**: un documento relevante que el grafo no conoce, porque una relación falta o está mal, igual aparece.
   - **Ampliar.** De los fragmentos hallados se pasa a sus entidades y asuntos, y de ahí a los documentos relacionados con evidencia.
   - **Recortar solo a propósito.** La búsqueda se limita a un conjunto únicamente si la persona lo elige (por ejemplo, un asunto). Se mide cuántos documentos relevantes se perderían si el grafo recortara (sección 10).

El modelo entra solo al final, para redactar la respuesta del chat con lo hallado y sus citas. Toda consulta recibe el conjunto de lo visible, como el resto de la base.

Los mismos vectores sirven además para proponer conceptos (sección 8), para la pasada "dice lo mismo con otras palabras" y para los candidatos difíciles de identidad: se calculan una vez y se usan cuatro veces.

**Construido en N5 (D-350, D-352), sin vectores:** el chat y la pantalla de búsqueda del wiki del archivo buscan en la base, siempre sobre lo visible: palabras del texto (las largas también por su raíz), números e identificadores en cualquier forma escrita (una columna con solo sus dígitos: con o sin puntos es el mismo número), y palabras en el resumen de cada documento, escrito en el idioma del archivo (así una pregunta en castellano encuentra un contrato en inglés). El grafo ordena y amplía: las entidades y asuntos que nombra la pregunta suben y suman sus documentos (un nombre genérico no cuenta) y los primeros hallados suman, con poco peso, los documentos que la base relaciona con ellos. Nunca recorta; solo un conjunto que elige una persona achica la búsqueda. El chat recibe una página por documento o familia de versiones y la página de la entidad más específica nombrada. Medido con las 30 preguntas de N1 (104 documentos esperados, en los 10 primeros): wiki de antes 61, índice de texto 79, índice más grafo de antes 81, búsqueda nueva 96 por documento y 95 en el chat; si el grafo recortara se perderían 4.

## 8. Conceptos propuestos por cálculo (hipótesis a medir)

**Alcance acordado (D-295):** por ahora solo la mitad de los **pasajes repetidos**, y solo como medición fuera del repositorio (cuántos grupos salen y cuántos son cláusulas reconocibles). La mitad "la misma idea con otras palabras" espera a los vectores. No se construyen pantallas.

Idea del dueño, para trabajar y verificar, no una definición:

- **Pasajes que se repiten en documentos distintos.** El cálculo de parecidos ya cuenta en cuántos documentos aparece cada secuencia de cinco palabras. Un pasaje que se repite en contratos de asuntos distintos suele ser una cláusula estándar (confidencialidad, jurisdicción, terminación, intereses): eso daría **familias de cláusulas**, útiles para comparar cómo se redactó una cláusula en todos los contratos.
- **La misma idea con otras palabras** no aparece por secuencias de palabras: hace falta la cercanía de vectores entre párrafos.
- **Recorrido:** el cálculo propone grupos de pasajes; el modelo nombra y define cada grupo una vez, mirando sus ejemplos (R-21); una persona confirma. Un concepto existe solo si atraviesa varios documentos.
- **Primero se mide** sobre la carpeta de prueba, sin pantallas: cuántos grupos salen y cuántos son cláusulas o figuras reconocibles.

## 9. Servicios de inteligencia artificial aislados

- Micelya Core va a ofrecer los servicios de IA (generativa y no generativa: preguntas al modelo, lectura de imágenes, vectores), y ahí se van a configurar los proveedores y modelos. La app tiene que poder usar Core **y** mantener la configuración de proveedores propia.
- **Preparación, sin que sea el foco ahora:** toda la configuración y el uso de servicios de IA pasa por un solo lugar de la app, con un contrato por servicio (preguntar con respuesta estructurada, leer una imagen, calcular vectores, redactar). Hoy ya existe ese contrato para preguntar y para leer imágenes (motores intercambiables); se suma el de vectores y se agrupa la configuración. Cuando llegue Core, es un proveedor más de esos contratos.
- Esto permite además elegir modelo y nivel por paso del proceso (P-140).

## 10. Cómo sabemos que funciona

- **Preguntas típicas:** 20 a 30 preguntas de un abogado sobre la carpeta de prueba, con los documentos que deberían aparecer en cada una. Se compara: búsqueda por palabras sola; con el grafo de enlaces de hoy; con el grafo de la base; con vectores (cuando estén). La primera medición decide si los enlaces guardados en archivos aportan algo.
- **Costo:** preguntas al modelo por documento antes y después (hoy unas 3 del wiki más las de la capa; objetivo: una lectura).
- **Calidad:** la hoja de respuestas de entidades y el juicio de atribución a ciegas no deben empeorar; las uniones falsas siguen en 0.
- **Errores críticos: demostrar mejora, no solo no empeorar** (D-296). Para los errores conocidos (atribución, importes, uniones falsas; después estabilidad y documentos perdidos en la búsqueda) hay casos de aceptación concretos y se comparan **dos corridas comparables** (mismo modelo, nivel y carpeta), antes y después.
- **Documentos perdidos por el grafo:** para cada pregunta típica se cuenta cuántos documentos relevantes quedarían afuera si el grafo recortara la búsqueda. Es la medida que justifica que el grafo ordene y no recorte (sección 7).
- **Redundancia:** cada dato se guarda en un solo lugar (se comprueba con una prueba automática que el wiki no tenga datos que no salgan de la base, salvo las notas).

## 11. Decisiones (resueltas el 18 de octubre, D-294)

1. **Página del documento:** archivo generado por ahora (como las de entidad; regenerable; notas protegidas). Se arma al mostrarse cuando la búsqueda y el grafo lean de la base.
2. **Cómo nace un asunto:** un solo concepto de asunto (P-061) que lo propone la extracción, lo crea una persona o lo propone una regla de carpeta; la etiqueta "Caso" es una forma de crearlo; no depende de cuántos documentos tiene. No se construye todavía. *Reemplazado el 20 oct por [Asuntos: un solo concepto](asuntos-propuesta.md) (D-373).*
3. **Investigación en internet:** herramienta aparte, a pedido, que nunca escribe en la base ni en el wiki del archivo. Por ahora solo se desconecta de la generación del wiki.
4. **Proveedor de vectores:** no se decide todavía; los vectores no entran en esta etapa.

**Orden de prioridad (D-296):** (1) atribución, importes y uniones falsas; (2) estabilidad y documentos perdidos en la búsqueda; (3) corregir y deshacer visibles en la app; (4) presentación.

## 12. Etapas

1. **Medir** con las preguntas típicas la búsqueda y el grafo de hoy (línea de base).
2. **Una extracción principal:** la lectura de la capa produce también el resumen del documento; el wiki deja de leer documentos con el modelo; se quitan conceptos por documento, consultas inventadas y avisos de revisión del wiki.
3. **Páginas desde la base:** documento (una por familia), persona y organización unificadas con su pantalla, asunto y grupo.
4. **Grafo y enlaces desde la base;** la búsqueda usa el índice de texto de la base.
5. **Vectores** como derivado de la base y búsqueda combinada; servicios de IA agrupados con contrato por servicio.
6. **Conceptos por cálculo,** si la medición los justifica.
