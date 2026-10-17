# Vista única de atención — propuesta

**Estado:** aprobada por el dueño y construida el 16 de octubre de 2026 en la rama `feat/attention-view` (sin integrar a `main`); lo firme pasó a la sección 13 del [documento de diseño](base-documental.md). Lo que quedó sin hacer está en `pendientes.json` (P-084 a P-089).
**Alcance:** dónde y cómo le avisa la app a una persona que algo requiere su atención, y cómo lo resuelve. Precisa la sección 13 del [documento de diseño](base-documental.md) ("la lista de lo que requiere atención es una sola"). No describe código.

Cuando esta propuesta esté acordada, lo que quede firme pasa al documento de diseño.

## 1. El problema

El dueño del producto lo encontró usando la app el 15 de octubre:

- **Inicio muestra líneas que no dicen qué hacer.** "4 documentos parecidos a «X» esperan una decisión" lleva a Documentos con un filtro puesto: una lista. Para saber cuál es el tema y qué acción tomar, hay que abrir cada documento y bajar hasta su sección de parecidos.
- **Inicio escala cosas que no son urgentes.** De las 137 propuestas de parecidos de la base del dueño, 115 son "misma plantilla, otros datos": documentos distintos, sin nada que unir. Solo las relaciones en que los dos son el mismo documento (prácticamente el mismo, ejemplar firmado, versión) cambian algo, porque deciden qué fuente va al wiki.
- **Los avisos están repartidos.** Unos aparecen en Inicio y otros en un panel chico al pie del costado del wiki, que además está en inglés y solo se ve en las pantallas del wiki.

## 2. Qué hay hoy

Relevado en el código el 16 de octubre.

| Dónde | Qué avisa | Qué se puede hacer ahí |
|---|---|---|
| **Inicio, "Requiere atención"** | Orígenes que no se pueden leer; archivos que no se pudieron leer; documentos sin texto, con texto en imagen o demasiado grandes; documentos de un disco en la nube que no se pueden procesar; cuántas propuestas del wiki esperan revisión; enlaces rotos del wiki; documentos parecidos que esperan una decisión | Solo hacer clic: lleva a otra pantalla, casi siempre a una lista filtrada |
| **Panel de actividad**, al pie del costado del wiki, en inglés | La cola de incorporación al wiki (avance, documentos que fallaron con su error); los cambios de archivos que no se pudieron registrar; mensajes sueltos de la incorporación | Pausar, reanudar, reintentar, cancelar y ordenar la cola; reintentar o ignorar un cambio de archivo |
| **Revisión**, pantalla del wiki, en inglés | Lo que el modelo marcó al incorporar documentos: contradicciones, páginas duplicadas, páginas que faltan, sugerencias. También lo que se manda desde la revisión de enlaces | Botones que propone el modelo (crear página, omitir), investigar, descartar |
| **Revisión de enlaces**, pantalla del wiki, en inglés | Páginas sin enlaces, enlaces rotos y, si se pide, contradicciones leídas por el modelo. Corre solo a pedido | Abrir, arreglar, borrar, mandar a Revisión |
| **Orígenes** | Si el wiki del archivo está activo, en pausa o sin activar; carpetas que no se pudieron usar como fuente del wiki; preguntas de tipo de documento sin hacer; **tipos que propuso el modelo y nadie confirmó** | Activar o pausar el wiki; reconocer tipos. Los tipos propuestos se muestran, pero **no hay forma de confirmarlos ni de rechazarlos** |
| **Detalle de un documento** | Su estado (sin texto, con error); sus documentos parecidos; si no alimenta el wiki porque se unió con otro | Unir, elegir otra relación, descartar, deshacer |
| **Ajustes, Mantenimiento** | Páginas de entidades o conceptos que podrían ser la misma, cuando se pide buscarlas | Unir o "no son la misma" |
| **Barra superior** | Hay una versión nueva de la app | Descargar o cerrar el aviso |

**Un caso que hoy no avisa nadie:** si no hay un modelo configurado, la cola del wiki se detiene y guarda el motivo ("falta configurar el modelo"), pero la pantalla no lo muestra: solo se ve que el botón dice "Reanudar".

## 3. Principios

1. **Un solo lugar.** Todo lo que requiere a una persona termina en una sola vista, sea cual sea la parte de la app que lo detectó: la base documental, el wiki y, más adelante, plazos, tareas y aprobaciones.
2. **Se escala solo lo que requiere una acción de una persona.** Lo que es información (un documento tiene parecidos con otros de su misma plantilla) se muestra en el documento, no se escala.
3. **Cada aviso dice tres cosas:** qué pasa, por qué importa y qué hay que hacer. Y trae la acción: el botón está en la misma fila, o lleva al lugar exacto donde se hace, nunca a una lista para buscar.
4. **Tres clases, que no se mezclan:**
   - **Problemas:** algo no funciona y la app no lo puede arreglar sola. Un origen desconectado, documentos que no se pudieron leer, una incorporación que falló, falta configurar el modelo.
   - **Decisiones:** algo espera el criterio de una persona. Parecidos que cambian qué va al wiki, propuestas del wiki, tipos de documento nuevos, páginas que podrían ser la misma.
   - **En curso:** lo que la app está haciendo. La cola del wiki, el barrido de los orígenes, las preguntas al modelo. No requiere atención; se muestra para saber qué pasa y para poder pausarlo.
5. **Un aviso se va solo cuando deja de ser cierto.** Si el origen vuelve o la persona decide, el aviso desaparece. No hay que "marcarlo como leído".
6. **Lo que se resuelve en bloque, se resuelve en bloque.** Una familia de trece facturas es una decisión, no trece. Para eso existen vistas dedicadas (sección 6).
7. **En español y con el vocabulario del vertical,** como las pantallas de la base.

## 4. La vista

Una pantalla propia en la navegación de la izquierda, con un contador. El nombre sale del vocabulario; para empezar, "Atención".

```
Atención                                                 Problemas 2 · Decisiones 9

PROBLEMAS
  ● No se puede leer «Servidor del estudio»
    La carpeta no está o aparece vacía. Lo que se sabía de sus archivos se conserva.
    Revisá que el disco esté conectado.                          [Volver a barrer]
  ● 3 documentos no se pudieron incorporar al wiki
    El modelo no respondió. Ver el error de cada uno.         [Reintentar] [Ver]

DECISIONES
  ◆ 4 pares de documentos podrían ser el mismo documento
    Si se unen, al wiki va uno solo, el que elijas.       [Resolver parecidos]
  ◆ 2 tipos de documento nuevos que propuso el modelo
    Hasta que se confirmen, no entran a la lista de tipos.   [Confirmar tipos]
  ◆ 3 propuestas del wiki
    Una contradicción entre dos páginas y dos páginas que faltan.   [Revisar]

EN CURSO
  ○ Incorporando al wiki: 12 de 40 documentos                      [Pausar]
```

- **Problemas arriba, después decisiones, después lo que está en curso.**
- **Cada línea agrupa por tema** y dice cuántos son. Al desplegarla se ven los casos uno por uno, cada uno con su acción.
- **La sección "En curso" reemplaza al panel de actividad:** el avance de la cola, pausar y reanudar, reintentar lo que falló, ver el error de cada documento. Pasa a estar en español y a verse desde cualquier pantalla.
- **Inicio deja de tener la lista completa.** Muestra un resumen ("2 problemas · 9 decisiones") que lleva a esta vista, más la salud del archivo y la actividad reciente, como hoy.
- **El contador de la navegación** suma problemas y decisiones, no lo que está en curso.

## 5. Qué pasa con cada aviso de hoy

| Aviso de hoy | Clase | Qué cambia |
|---|---|---|
| Origen que no se puede leer | Problema | Con la acción "Volver a barrer" en la fila |
| Archivos que no se pudieron leer; documentos sin texto | Problema | La línea despliega los documentos; cada uno, con su motivo |
| Documentos con el texto en imagen | En curso | No es un problema: se reconocen al incorporarlos. Pasa a ser un problema solo si el reconocimiento falla |
| Documentos demasiado grandes; documentos de un disco en la nube | Problema, de baja prioridad | Se explica que quedan afuera y por qué |
| Falta configurar el modelo | Problema | **Nuevo:** hoy no se muestra. Con la acción "Ir a la configuración" |
| Incorporación que falló | Problema | Sale del panel de actividad; con reintentar y el error en palabras |
| Cambio de archivo que no se pudo registrar | Problema | Sale del panel de actividad. "Ignorar" explica que ese cambio se pierde |
| Parecidos que son el mismo documento (prácticamente el mismo, ejemplar firmado, versión) | Decisión | Lleva a la vista de parecidos (sección 6), no a una lista de documentos |
| Parecidos "misma plantilla", "contenido en", "parecidos" | No se escala | Se ven en el documento y en la vista de parecidos. Ver la decisión abierta 2 |
| Propuestas del wiki (Revisión) | Decisión | Se cuentan acá y se resuelven en Revisión, que pasa al español |
| Enlaces rotos del wiki | Decisión, de baja prioridad | Lleva a la revisión de enlaces |
| Tipos de documento que propuso el modelo | Decisión | **Nuevo:** hoy no se pueden confirmar (P-080) |
| Páginas que podrían ser la misma (Mantenimiento) | Decisión | Se suman cuando esa búsqueda se haga sola; hoy corre a pedido |
| Cola del wiki, barrido, preguntas al modelo | En curso | Sale del panel de actividad |
| Versión nueva de la app | Queda donde está | Es de la app, no del trabajo: la barra superior sigue |
| Un comando que el chat pide aprobar | Queda donde está | Se decide en el momento, dentro de la conversación |

## 6. Vistas dedicadas para resolver

Algunas decisiones no se resuelven bien de a una fila. Para esas, la vista de atención lleva a una pantalla hecha para resolverlas:

- **Documentos parecidos.** Ordenada por familia, con el documento principal y sus miembros, lo que cambia entre ellos, las diferencias lado a lado (P-077) y los botones para unir, elegir otra relación o descartar. Primero lo que cambia qué va al wiki. El detalle de cada documento sigue mostrando sus parecidos, como hoy.
- **Tipos de documento.** Los tipos que propuso el modelo, con sus documentos, para confirmarlos, renombrarlos o rechazarlos (P-080).
- **Revisión del wiki,** la que ya existe, en español.

## 7. Qué hace falta por debajo

Hoy cada aviso sale de un lugar distinto: la base documental, la cola del wiki, el archivo de revisión de cada proyecto, la revisión de enlaces. La propuesta es que **cada parte de la app siga detectando lo suyo**, pero lo entregue con la misma forma: clase, tema, título, explicación, qué hacer, la acción y cuándo deja de ser cierto. La vista solo junta y ordena. Así, sumar más adelante plazos, tareas o aprobaciones es agregar una fuente, no rehacer la vista.

## 8. Decisiones abiertas

*Resueltas por el dueño el 16 de octubre: (1) se llama "Atención", desde el vocabulario; (2) "misma plantilla" se confirma de a una familia, con un clic, en la vista de parecidos, sin escalarse; (3) la revisión estructural de enlaces corre sola al vaciarse la cola del wiki (solo la estructural, sin modelo); (4) la vista muestra los avisos del archivo y los del proyecto abierto; (5) por ahora no se puede posponer.*

1. **El nombre de la vista** ("Atención", "Pendientes", "Bandeja"). Sale del vocabulario, así que puede cambiar por vertical.
2. **"Misma plantilla" en el proceso.** No requiere una decisión urgente, pero las pasadas que vienen (mismo asunto, series y faltantes) solo se apoyan en lo que confirmó una persona (R-03). Dos caminos: confirmarla de a una familia por vez, con un clic, en la vista de parecidos, sin escalarla; o tratar "misma plantilla" como dato calculado que no necesita confirmación para esas pasadas, lo que exige revisar R-03.
3. **La revisión de enlaces automática.** Hoy corre solo a pedido, y sus números no coinciden con los enlaces rotos que cuenta Inicio. Correrla sola al terminar la cola haría que la vista diga siempre lo mismo que la revisión.
4. **Los avisos de otros proyectos de wiki.** La revisión del wiki es por proyecto. Si se abre un proyecto que no es el archivo, ¿la vista muestra los avisos de ese proyecto, los del archivo o los dos?
5. **Posponer.** Si una persona puede decir "ahora no" a una decisión y hacerla desaparecer por un tiempo, o si la vista muestra siempre todo lo pendiente.

## 9. Cómo lo construiríamos

En pasos chicos, cada uno comprobable:

1. **Sacar de Inicio lo que no requiere acción.** Inicio deja de escalar "misma plantilla" y las demás relaciones que no cambian el wiki, y la línea de parecidos dice qué hacer.
2. **La vista con lo que ya existe:** problemas y decisiones de la base documental y del wiki, cada uno con su acción. Inicio pasa a mostrar el resumen.
3. **"En curso":** la cola del wiki y su control, en español, y el aviso de que falta configurar el modelo. El panel de actividad deja de hacer falta.
4. **Las vistas dedicadas:** parecidos por familia y confirmación de tipos.

Esto va antes del marco visual de toda la app (P-057) o junto con él: la vista nace con el estilo de hoy, como las demás pantallas de la base, y el marco la alcanza después.
