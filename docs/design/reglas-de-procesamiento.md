# Reglas del procesamiento

**Estado:** punto de partida · 14 de octubre de 2026
**Para quién:** para las personas que revisan cómo trabaja la app, y para los agentes que la programan. Toda pasada, cálculo o uso del modelo que analice documentos tiene que respetarlas.

Estas reglas dicen **cómo** la app saca conclusiones sobre los documentos: qué puede afirmar, sobre qué base, y qué nunca hace sola. Son un punto de partida: se revisan a medida que las pruebas muestren cuáles funcionan. Una regla que cambia no se borra; se marca como reemplazada y se anota por qué, igual que en el [registro de decisiones](../decisiones.md).

Nacieron del diseño de los documentos parecidos ([propuesta](parecidos-propuesta.md)), pero valen para cualquier análisis de la base documental.

## 1. Sobre las fuentes y las inferencias

**R-01 · Las fuentes son la verdad.** Una fuente es el documento original. Todo lo demás (el texto extraído, una relación, un tipo de documento, una página del wiki) es una conclusión sacada de las fuentes, y tiene que poder rastrearse hasta ellas.

**R-02 · No se infiere sobre lo inferido.** Una conclusión se saca de las fuentes, no de otra conclusión. Encadenar inferencias acumula el error de cada una. Si alguna vez hace falta apoyarse en una conclusión anterior, debe ser excepcional, y lo que resulte se verifica contra las fuentes antes de darlo por bueno.

**R-03 · Lo propuesto no alimenta lo propuesto.** Una propuesta de la app (por ejemplo, "estos dos son versiones") no sirve de base para otra propuesta. Solo sirve de base lo que confirmó una persona.

**R-04 · Los nombres de archivo y de carpeta son hipótesis.** Sirven para sugerir qué es un documento o a qué serie pertenece. Cuando el contenido dice otra cosa, manda el contenido. Ejemplo: una factura llamada "202605" puede ser del mes de consumo, no de la fecha de emisión.

**R-05 · Compartir una persona no es compartir un asunto.** Que dos documentos nombren a la misma persona o empresa no los hace parte del mismo caso. Para "mismo asunto" hace falta algo más específico: el mismo contrato, el mismo expediente, los mismos importes.

## 2. Sobre las decisiones

**R-06 · La app no une por su cuenta.** Propone. Solo es automático lo que ya existe: dos archivos con las mismas palabras son el mismo documento.

**R-07 · Ante la duda, no se une.** Unir dos cosas distintas (mezclar dos clientes) es mucho peor que dejar un duplicado sin detectar. Todos los umbrales se ajustan con ese criterio.

**R-08 · Lo que decide una persona es firme.** Un "sí" se puede usar como base. Un "no tienen relación" bloquea esa relación en todas las pasadas, salvo que aparezca evidencia nueva y distinta. Cada decisión se guarda con la evidencia que se le mostró a la persona.

**R-09 · Nada se pierde.** Lo que decidió una persona se puede deshacer. Lo que calculó la app se puede rehacer entero desde las fuentes.

## 3. Sobre las pasadas

**R-10 · Cada pasada tiene un único objetivo,** con lo que usa, lo que produce, lo que puede cambiar y cuándo se corta.

**R-11 · De lo seguro a lo incierto.** Primero lo exacto, después lo que comparte mucho texto, después lo que se infiere.

**R-12 · Una pasada solo agrega evidencia.** Escribe propuestas y les cambia la certeza; nunca une ni borra. Como la evidencia posible es finita y solo crece, el proceso termina.

**R-13 · El resultado no depende del orden.** Procesar en otro orden, o repetir una pasada sin cambios, da lo mismo.

**R-14 · Las familias se arman contra un documento principal,** no por cadena: un documento entra si se parece al principal, no a cualquier miembro. Una familia que crece o se dispersa demasiado se marca para revisión.

**R-15 · Se trabaja solo sobre lo que cambió.** Cada pasada mira únicamente lo tocado por algo nuevo (un documento, una decisión humana) y sus vecinos.

**R-16 · Cada pasada tiene un límite propio:** cuántos pares evalúa, cuántas preguntas le hace al modelo, cuántas propuestas le pone en la cola a una persona.

**R-17 · Cada pasada se mide por separado** contra la hoja de respuestas: cuántas propuestas acierta, cuántas relaciones se le escapan y cuántas acepta la persona. Si empeora, se ajusta antes de volver a correrla.

## 4. Sobre el modelo de inteligencia artificial

**R-18 · Se usa donde agrega valor o donde el proceso la necesita.** Lo determinista se prefiere solo si iguala o mejora el resultado, con la misma eficiencia o una mejor. Ahorrar cupo nunca justifica una solución peor. *(Reemplaza, el 16 de octubre de 2026, a la versión que decía "se reduce su uso, no se elimina": se leía como una orden de evitar el modelo. Ver D-150 en el [registro de decisiones](../decisiones.md).)*

**R-19 · Cálculo y modelo se alternan, cada uno con un propósito.** El cálculo encuentra y mide; el modelo lee, clasifica y explica. Una pasada del modelo puede ir antes de una de cálculo cuando lo que produce (por ejemplo, el tipo de documento) mejora la medición siguiente.

**R-20 · El modelo aporta evidencia, no decide.** Su respuesta pesa como una evidencia más y se guarda junto con lo que se le preguntó y la versión del modelo. La misma pregunta no se hace dos veces.

**R-21 · Primero se agrupa, después se pregunta.** Al modelo se le pregunta por lo seleccionado, no por todo: por el principal de un grupo (y vale para todo el grupo), por un documento que quedó solo, por un par dudoso. Nunca por todos los pares posibles.

**R-22 · Los nombres confirmados quedan fijos.** Cuando una persona confirma un tipo de documento o un nombre, pasa a una lista cerrada. El modelo elige de esa lista o propone uno nuevo marcado como tal.

## 5. Sobre lo que el proceso tiene que decir

**R-23 · Lo que falta es un resultado.** Además de relacionar lo que hay, el proceso tiene que avisar lo que falta: un hueco en una serie, el ejemplar firmado de un contrato que no aparece, la carta enviada de una vista previa.

## 6. Sobre cómo se aprende de los casos de prueba

**R-24 · Ninguna regla nombra un caso.** Las reglas, el catálogo de tipos y el programa hablan de señales generales (qué tipo de dato cambió, una marca de agua, la estructura de un formulario, la forma de un número de envío). Nunca de una persona, una empresa (salvo la que emite un formulario, como un correo o un servicio de firma), un archivo o un número concreto. Ejemplo de lo que no puede existir: "si el documento es de tal persona, entonces son asuntos distintos".

Lo que se sabe de un caso puntual ("estos dos documentos de la misma persona son asuntos distintos") es la decisión de una persona sobre ese par: se guarda como dato junto al par, no como regla. Si el proceso no llega solo a esa conclusión, lo que falta es una señal general, y se busca cuál es.

Se comprueba automáticamente: una prueba revisa que ni el programa ni el catálogo contengan nombres, números u otros valores de la hoja de respuestas.

**R-25 · La hoja de respuestas mide; no enseña.** Sirve para comprobar si las reglas aciertan, no para escribirlas. Hoy una regla se da por buena si procesa bien los documentos de la carpeta de prueba. Cuando haya más documentos, se vuelven a comprobar todas, y agregar una regla o un tipo nuevo tiene que ser fácil: es una entrada más en el catálogo, no un cambio en el programa.

**R-26 · Lo aprendido de casos concretos se guarda como conocimiento, no como excepciones.** Si un caso enseña algo (cómo es la vista previa de un correo, cómo sella un servicio de firma), entra al catálogo de tipos de documento o al vocabulario, donde una persona lo puede leer y corregir. No se esconde en el programa como un caso especial.

**R-27 · Antes de preguntarle algo a una persona, se agota lo que se puede averiguar.** Leer el documento (también con la vista, si es una imagen), cruzarlo con los demás y con lo que la persona ya dijo. Se pregunta solo lo que de verdad no se puede saber, y se dice qué se averiguó.
