# El wiki como presentación de la base — propuesta

**Estado:** propuesta para revisar · 17 de octubre de 2026 · nada de esto está construido
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
- **La síntesis en prosa** de lo que dicen los documentos, escrita por el modelo solo desde afirmaciones verificadas.
- **Las notas de una persona** en cada página, que la app nunca reescribe.

## 3. Principios

1. **Una sola fuente de verdad por cada cosa.** Lo que la base sabe (documentos, copias, versiones, tipo, datos, entidades, relaciones, asuntos) se guarda solo en la base. El wiki lo muestra; no lo vuelve a calcular ni lo guarda de otra forma.
2. **Una sola lectura del modelo por documento y por espacio.** De esa lectura salen las menciones, las afirmaciones, los datos y el resumen del documento. Nada vuelve a leer un documento con el modelo salvo que cambie su texto o la pregunta.
3. **Una página por documento, no por copia ni por versión.** Las copias y las versiones unidas son una sola página, que las lista.
4. **Las relaciones viven en la base, con tipo, cita y estado.** Los enlaces de una página se arman al mostrarla, a partir de esas relaciones.
5. **El modelo, donde agrega valor** (D-150): leer, resolver lo dudoso y redactar. No para recalcular lo que la base ya sabe.

## 4. Qué pasa con cada proceso heredado

| Proceso de hoy | Qué hace | Propuesta |
|---|---|---|
| Análisis y generación del wiki por documento (2 o 3 llamadas) | Lee el documento y escribe su página, conceptos y avisos | **Se reemplaza** por la lectura única de la capa (principio 2), que suma el resumen del documento como un derivado más |
| Lectura de la capa de conocimiento | Menciones, afirmaciones, datos | **Se conserva** y pasa a ser la única lectura |
| Página de fuente escrita por el modelo | Resumen con enlaces | **Se rehace** como página del documento armada desde la base (sección 5) |
| Conceptos por documento | Una página por cláusula de un contrato | **Se quitan.** Vuelven, si la medición lo justifica, como conceptos propuestos por cálculo (sección 8) |
| Consultas que el modelo inventa al incorporar | "Preguntas abiertas" sobre lo que falta | **Se quitan.** Lo que falta es un aviso de la base en Atención (R-23). Se conserva guardar en el wiki una respuesta del chat, a pedido de una persona |
| Avisos de revisión del wiki (contradicciones, duplicados, páginas que faltan) y su limpieza con el modelo | Avisos de texto libre | **Se quitan.** Las contradicciones y los duplicados ya los detecta la capa con evidencia y van a Atención |
| Combinación de páginas con el modelo | Une una página existente con una nueva | **Se quita** para las páginas que salen de la base; queda solo si una persona escribió la página a mano |
| Unión de duplicados de Ajustes | Ya convertida en decisiones del registro | Se conserva así |
| Búsqueda en internet e investigación web | Trae información externa | **Fuera del wiki del archivo** (contradice "sin conocimiento externo"); puede quedar como herramienta aparte, a pedido |
| Revisión de enlaces | Estructural sin modelo y, opcional, con el modelo | Se conserva la estructural para las notas; **se quita** la del modelo |
| Reconocimiento de texto de imágenes | Una vez por documento; el texto vuelve a la base | Se conserva |
| Síntesis de entidades y resumen general | Una pregunta por entidad tocada; resumen sin modelo | Se conserva |
| Representación por significado (hoy apagada) | Sobre las páginas del wiki | **Se mueve a la base**, sobre el texto de los documentos (sección 7) |

## 5. Las páginas

- **Documento.** Una por documento o familia de versiones. Arriba, lo que sabe la base: tipo y etapa, fecha, partes con su rol (enlazadas a sus páginas), asunto, copias y ubicaciones, versiones y su relación (ejemplar firmado, vista previa…), datos del tipo. Abajo, el resumen en prosa que salió de la lectura única, con referencias a las afirmaciones. Notas protegidas.
- **Persona y organización.** La que ya proyecta la capa, unificada con la pantalla Personas y organizaciones: una sola página con forma de artículo y las acciones de corrección (unir, separar, dato incorrecto) en una pestaña.
- **Asunto.** Solo cuando junta documentos distintos (no versiones de uno, que son una familia del documento). Se muestra como lo que es, el caso, no como un tipo de entidad. Está atado a cómo nace un caso (P-061): extraído de los documentos, creado desde la interfaz, o las dos cosas.
- **Grupo.** Las sociedades que lo forman y lo que no se puede atribuir a una sola.
- **Índice y resumen general.** Desde la base, sin modelo, como ya se hace.
- **Consultas guardadas.** Solo las que una persona guarda desde el chat.

## 6. El grafo y los enlaces

- **El grafo se arma desde la base:** nodos de documentos, entidades, asuntos y grupos; aristas con su tipo ("debe a", "representa a", "parte en", "aparece en", "versión de"…), su cita y su estado (propuesta, confirmada). Hoy el grafo lee los enlaces de los archivos y convierte todas las relaciones en la misma flecha sin tipo.
- **Los enlaces de una página se generan al mostrarla,** a partir de las relaciones de la base. Un enlace existe solo si la relación existe: no hay enlaces rotos posibles. Los enlaces que el modelo escriba en un resumen se reescriben desde el registro (como ya se hace).
- **Las notas de una persona** pueden tener enlaces escritos a mano; se resuelven contra la base al mostrarse.
- **Esto reemplaza D-076** (las páginas son la verdad y la base lleva un índice reconstruible) para todo lo que sale de la base. El motivo de D-076 era que el modelo escribía todas las páginas como texto; eso ya no es así. Resuelve también P-068.

## 7. La búsqueda

Combina tres fuentes, todas en la base:

1. **Índice de texto de SQLite**, que la base ya tiene sobre el texto de cada documento: palabras exactas, nombres, números, identificadores.
2. **Vectores** de cada fragmento del texto (párrafo o cláusula), calculados una vez por documento y por espacio como un derivado más, incrementales, en un índice reconstruible (LanceDB, sección 9 del diseño): lo dicho con otras palabras.
3. **El grafo**, para acotar y para ampliar:
   - **Acotar.** Una pregunta que nombra entidades o un asunto se resuelve primero en el grafo, y la búsqueda de texto y vectores corre solo sobre esos documentos. Es más rápido y más preciso que buscar en todo el archivo.
   - **Ampliar.** De los fragmentos hallados se pasa a sus entidades y asuntos, y de ahí a los documentos relacionados con evidencia.

El modelo entra solo al final, para redactar la respuesta del chat con lo hallado y sus citas. Toda consulta recibe el conjunto de lo visible, como el resto de la base.

Los mismos vectores sirven además para proponer conceptos (sección 8), para la pasada "dice lo mismo con otras palabras" y para los candidatos difíciles de identidad: se calculan una vez y se usan cuatro veces.

## 8. Conceptos propuestos por cálculo (hipótesis a medir)

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
- **Calidad:** la hoja de respuestas de entidades y el juicio de atribución a ciegas no deben empeorar.
- **Redundancia:** cada dato se guarda en un solo lugar (se comprueba con una prueba automática que el wiki no tenga datos que no salgan de la base, salvo las notas).

## 11. Decisiones abiertas

1. Si la página del documento es un archivo generado (como las de entidad) o se arma solo al mostrarse.
2. Cómo nace un asunto (P-061), antes de seguir mejorando su extracción.
3. Si la investigación en internet queda como herramienta aparte o se quita.
4. Con qué proveedor de vectores se empieza mientras no esté Core (local o configurable).

## 12. Etapas

1. **Medir** con las preguntas típicas la búsqueda y el grafo de hoy (línea de base).
2. **Una sola lectura:** la lectura de la capa produce también el resumen del documento; el wiki deja de leer documentos con el modelo; se quitan conceptos por documento, consultas inventadas y avisos de revisión del wiki.
3. **Páginas desde la base:** documento (una por familia), persona y organización unificadas con su pantalla, asunto y grupo.
4. **Grafo y enlaces desde la base;** la búsqueda usa el índice de texto de la base.
5. **Vectores** como derivado de la base y búsqueda combinada; servicios de IA agrupados con contrato por servicio.
6. **Conceptos por cálculo,** si la medición los justifica.
