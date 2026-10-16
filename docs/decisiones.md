# Registro de decisiones

**Proyecto:** Micelya Desktop (repositorio `Micelya/llm_wiki`) · **Al 16 de octubre de 2026**

## Qué es y cómo usarlo

Este registro junta, en un solo lugar, cada decisión de producto, de diseño y de forma de trabajo tomada desde que empezó el proyecto. Está pensado para alguien que no participó de las conversaciones: cada entrada cuenta primero qué problema obligó a decidir y después qué se decidió y por qué, sin depender de otras entradas.

- **Orden:** cronológico. El tema de cada decisión es un campo, no un capítulo.
- **Nada se borra.** Si una decisión cambia, se agrega una entrada nueva y la vieja queda marcada "reemplazada por D-xxx". Cuando el cambio afecta solo una parte, se marca "reemplazada en parte por D-xxx" y se dice qué parte sigue valiendo.
- **Estados:** *vigente* (vale hoy), *reemplazada por D-xxx*, *en revisión* (tomada, pero falta confirmarla o quedó en espera).
- **Quién decidió:** *dueño del producto*; *agente* (el asistente de IA que programa) cuando el dueño lo pidió o lo aceptó; *agente sin consulta* cuando el asistente decidió solo mientras el dueño no estaba, con su estado de confirmación según el pendiente P-066.
- **Fechas de la sesión 01:** esa sesión abarcó del 5 al 8 de octubre y no registra el día exacto de cada decisión; esas entradas dicen "5 al 8 oct".

**Palabras que se repiten**, para no explicarlas en cada entrada:

- **Upstream:** el proyecto original de código abierto del que se partió (`nashsu/llm_wiki`). **Fork:** nuestra copia propia de ese proyecto, que evoluciona por separado.
- **PR (pedido de integración):** un paquete de cambios de código que se revisa antes de entrar a la versión principal (`main`). Se identifica por número (#7, #33…).
- **P-xxx:** un pendiente anotado en `pendientes.json`. **F-xxx:** una diferencia con upstream anotada en `FORK_CHANGES.md`.
- **Modelo / cupo:** el modelo de lenguaje (la IA) que lee documentos y escribe el wiki; cada uso consume cupo de una suscripción o cuesta dinero.
- **Ingesta o incorporación:** el paso en que un documento se le pasa al modelo para que escriba páginas del wiki. **Cola:** la fila de documentos esperando ese paso.
- **Huella:** un número calculado a partir del contenido de un archivo o de un texto; si dos huellas coinciden, el contenido es idéntico. SHA-256 es el método de cálculo usado.
- **Base documental:** el componente que sabe qué archivos hay, dónde está cada copia y cuáles son el mismo documento. Es el cimiento del producto desde el 9 de octubre.

## Índice

| ID | Fecha | Tema | Decisión en una línea | Estado |
|---|---|---|---|---|
| D-001 | 5 al 8 oct | Producto | El fork se convierte en Micelya Desktop, producto para estudios jurídicos | vigente |
| D-002 | 5 al 8 oct | Base documental | Los documentos quedan en su carpeta original, sin copiarse | vigente |
| D-003 | 5 al 8 oct | Base documental | El mismo documento en varias rutas o formatos se procesa una vez | vigente |
| D-004 | 5 al 8 oct | Parecidos | El parecido solo propone una relación, nunca fusiona; importa qué cambió | vigente |
| D-005 | 5 al 8 oct | Parecidos | Versiones como familias; "vigente" opcional; todos van al wiki | reemplazada en parte por D-098 |
| D-006 | 5 al 8 oct | Base documental | Separar el documento (contenido) del archivo en una ruta (ubicación) | vigente |
| D-007 | 5 al 8 oct | Base documental | Motores de lectura y orígenes de documentos intercambiables | vigente |
| D-008 | 5 al 8 oct | Parecidos | Comparar por secuencias de cinco palabras y medida de contención | vigente |
| D-009 | 5 al 8 oct | IA | Proveedor de IA: Codex en desarrollo; para clientes, suscripción propia o servicio de Amazon detrás de Micelya | vigente, con P-029 abierto |
| D-010 | 5 al 8 oct | IA | Leer con un motor intercambiable las páginas escaneadas y las fotos | vigente |
| D-011 | 5 al 8 oct | Upstream | La arquitectura pesa más que evitar choques con upstream | reemplazada por D-034 |
| D-012 | 5 al 8 oct | Proceso de trabajo | Registros: diferencias con upstream, pendientes, traspasos de sesión | reemplazada en parte por D-152 |
| D-013 | 5 al 8 oct | Proceso de trabajo | Todo cambio entra por PR contra el fork, con `main` protegida | vigente |
| D-014 | 5 al 8 oct | Proceso de trabajo | Explicaciones en español, simples, autocontenidas y verificadas | vigente |
| D-015 | 5 al 8 oct | Seguridad | Endurecer los procesos automáticos de GitHub | vigente |
| D-016 | 5 al 8 oct | Seguridad | Actualización mensual de las acciones de GitHub, revisada con demora | vigente |
| D-017 | 5 al 8 oct | Seguridad | No cargar claves de firma hasta proteger su uso | vigente |
| D-018 | 5 al 8 oct | Distribución | Identidad y numeración de versiones propias | vigente |
| D-019 | 8 oct | Base documental | Catálogo: se aceptan las cuatro propuestas (P-039) | vigente |
| D-020 | 8 oct | Base documental | "Mismo documento" = mismo texto normalizado; una cifra distinta es otro | vigente |
| D-021 | 8 oct | Base documental | Calcular la huella del texto al empezar la ingesta, no al importar | reemplazada por D-024 |
| D-022 | 8 oct | Proceso de trabajo | Avanzar con todos los pendientes de código, en commits parciales | vigente |
| D-023 | 8 oct | Base documental | Un origen vacío o desconectado no es un origen sin archivos | vigente |
| D-024 | 8 oct | Base documental | Huella del archivo primero, antes de encolar | vigente |
| D-025 | 8 oct | IA | Texto reconocido guardado por huella, por proyecto, en `raw/`, en JSON | reemplazada en parte por D-035 |
| D-026 | 8 oct | IA | El reconocimiento corre solo dentro de la ingesta, con avance por página | vigente |
| D-027 | 8 oct | Interfaz | Nombre del producto con un único reemplazo al traducir | vigente |
| D-028 | 8 oct | IA | Chat con proveedores de línea de comandos: busca y recibe el texto de hasta 10 páginas | vigente |
| D-029 | 8 oct | Interfaz | Buscador que ignora palabras vacías del español | vigente |
| D-030 | 8 oct | Interfaz | Grafo con etiquetas solo en los nodos principales | en revisión |
| D-031 | 8 oct | Interfaz | Aviso de importación informativo cuando no hubo error | vigente |
| D-032 | 8 oct | Proceso de trabajo | Programar en una carpeta separada de la que corre la app | vigente |
| D-033 | 9 oct | Producto | Cambio de rumbo: la base documental es el cimiento | vigente |
| D-034 | 9 oct | Upstream | Las necesidades propias pesan más que la compatibilidad con upstream | reemplazada por D-151 |
| D-035 | 9 oct | Base documental | La unidad es el espacio (privado o compartido), no el proyecto | vigente |
| D-036 | 9 oct | Base documental | La base documental es un componente separado | vigente |
| D-037 | 9 oct | Seguridad | Autorización al estilo UMA 2.0 | reemplazada por D-045 |
| D-038 | 9 al 11 oct | Base documental | Tres niveles: ubicación, contenido, familia | vigente |
| D-039 | 9 al 11 oct | Base documental | Cuatro capas; el wiki le pregunta a la base solo tres cosas | vigente |
| D-040 | 9 al 11 oct | Base documental | SQLite local como fuente de verdad; búsqueda por significado aparte | vigente |
| D-041 | 9 al 11 oct | Base documental | Orígenes admitidos; documentos nativos de Google detectados, no procesados | vigente |
| D-042 | 9 al 11 oct | IA | Determinista primero; IA una vez por grupo | reemplazada en parte por D-150 |
| D-043 | 9 al 11 oct | Base documental | Sin ninguna ubicación, se borran los derivados | vigente |
| D-044 | 9 al 11 oct | Seguridad | Reglas de los espacios privado y compartido | vigente |
| D-045 | 11 oct | Seguridad | Autorización: el origen es el techo y el proyecto es el alcance | vigente |
| D-046 | 9 al 11 oct | Distribución | Despliegue: servicio compartido opcional, un solo escritor, IA por instancia | vigente |
| D-047 | 9 al 11 oct | Wiki | La generación del wiki sigue corriendo en la ventana de la app | vigente |
| D-048 | 9 al 11 oct | Producto | Cinco etapas, empezando por el espacio privado | vigente |
| D-049 | 9 al 11 oct | Base documental | Proyectos de prueba descartables; carpetas originales intocables | vigente |
| D-050 | 9 al 11 oct | Proceso de trabajo | Reglas de colaboración con el dueño del producto | vigente |
| D-051 | 9 al 11 oct | Seguridad | Tres reglas de autorización en espera (P-052 a P-054) | en revisión |
| D-052 | 11 oct | Producto | Producto mínimo: etapa 1 más el puente con el wiki | vigente |
| D-053 | 11 oct | Proceso de trabajo | Producto mínimo primero; lo posterior queda "en espera" | vigente |
| D-054 | 11 oct | Interfaz | Cuatro pantallas de la base, con el estilo actual de la app | vigente |
| D-055 | 11 oct | Interfaz | Marco visual y marca: para toda la app, en un turno dedicado | vigente |
| D-056 | 11 oct | Interfaz | Pantallas nuevas en español; traducción completa aparte | vigente |
| D-057 | 11 oct | Etiquetas | "Cliente" y "caso" son etiquetas, no proyectos | vigente |
| D-058 | 11 oct | Upstream | No proponer contribuciones a upstream hasta nuevo aviso | reemplazada por D-151 |
| D-059 | 11 oct | IA | La base no le manda nada al modelo por su cuenta | vigente |
| D-060 | 11 oct | Wiki | Asignar el proyecto general apaga su incorporación automática | reemplazada por D-071 |
| D-061 | 11 oct | Wiki | El proyecto general se elige entre los existentes | reemplazada por D-069 |
| D-062 | 11 oct | Interfaz | La app abre en Inicio | en revisión |
| D-063 | 11 oct | Wiki | Quitar una carpeta de la base deja las páginas ya generadas | en revisión |
| D-064 | 11 oct | Base documental | Vigilar solo por barrido periódico | en revisión |
| D-065 | 11 oct | Wiki | "Qué cambió" lo sigue detectando el vigilante del wiki | en revisión |
| D-066 | 11 oct | Base documental | Archivos que el inventario ignora | en revisión |
| D-067 | 11 oct | Interfaz | Mensajes de error del motor en inglés, por ahora | en revisión |
| D-068 | 11 oct | Etiquetas | Dejar vacía una etiqueta corregida vuelve al valor de la carpeta | en revisión |
| D-069 | 12 oct | Producto | La app crea sola el proyecto general y lo llama "Archivo" | vigente |
| D-070 | 12 oct | Producto | Nombres técnicos neutros; la carpeta de trabajo se pregunta una vez | vigente |
| D-071 | 12 oct | Wiki | El wiki del Archivo se activa a mano y después se alimenta solo | vigente |
| D-072 | 12 oct | Producto | Términos del dominio en un vocabulario por profesión e idioma | vigente |
| D-073 | 12 oct | Base documental | "Excluir" en vez de "borrar" | vigente |
| D-074 | 12 oct | Interfaz | Orígenes como árbol, con acciones por fila | vigente |
| D-075 | 12 oct | Etiquetas | Grupos y etiquetas, reglas por carpeta, lo manual manda | vigente |
| D-076 | 12 oct | Wiki | Las páginas son la verdad; la base lleva un índice reconstruible | en revisión |
| D-077 | 12 oct | Interfaz | Si había otro proyecto abierto, la app lo reabre | en revisión |
| D-078 | 12 oct | Base documental | "Volver a barrer" en una carpeta barre todo su origen | en revisión |
| D-079 | 12 oct | Wiki | Excluir no quita del wiki lo ya incorporado | en revisión |
| D-080 | 12 oct | Wiki | Enlaces emparejados sin distinguir mayúsculas, espacios ni guiones | en revisión |
| D-081 | 12 oct | Etiquetas | El color de cada etiqueta lo pone la app | en revisión |
| D-082 | 12 oct | Wiki | El índice de enlaces todavía no lo usan grafo, búsqueda ni revisión | en revisión |
| D-083 | 12 oct | Producto | Un solo vertical, fijo, por ahora | en revisión |
| D-084 | 12 oct | Distribución | Probar el instalador en otra máquina antes de entregarlo | vigente |
| D-085 | 13 oct | Idioma | El Archivo nace con un idioma de wiki fijo | en revisión |
| D-086 | 13 oct | Idioma | Un idioma fijo por estudio, con el ajuste que la app ya tiene | vigente |
| D-087 | 13 oct | Idioma | El wiki en varios idiomas se posterga mucho | en revisión |
| D-088 | 13 oct | Idioma | Detección de idioma sin IA pero confiable; idioma como dato de la base | vigente |
| D-089 | 13 oct | Wiki | "Pausar" el wiki también frena la cola | en revisión |
| D-090 | 13 oct | Etiquetas | Caja de etiquetas: Enter no crea por accidente; sin nombres fijos de clientes | en revisión |
| D-091 | 13 oct | Distribución | Una sola copia de la app corriendo a la vez | vigente |
| D-092 | 13 oct | Distribución | Instaladores: corrida manual sin publicar; ninguna versión pública todavía | vigente |
| D-093 | 13 oct | Wiki | Para rehacer un wiki se aparta la carpeta, nunca se borra | vigente |
| D-094 | 13 oct | Parecidos | Diseñar el recorrido antes de construir; ante la duda, no se une | vigente |
| D-095 | 13 oct | Parecidos | Los parecidos los une siempre una persona | vigente |
| D-096 | 13 oct | Parecidos | Se muestran en el detalle del documento, con botones para unir | vigente |
| D-097 | 13 oct | Parecidos | Relaciones ampliables y con nombres configurables | vigente |
| D-098 | 13 oct | Parecidos | Al unir como el mismo documento, queda una sola fuente del wiki | vigente |
| D-099 | 13 oct | IA | Reducir el uso de IA, no eliminarlo | reemplazada en parte por D-150 |
| D-100 | 13 oct | Base documental | Procesar en pasadas sobre todo el archivo | en revisión |
| D-101 | 13 oct | Parecidos | El conjunto de prueba lo arma el asistente y lo corrige el dueño | vigente |
| D-102 | 13–14 oct | Proceso de trabajo | Esfuerzo estimado solo en términos relativos, nunca en tiempo | vigente |
| D-103 | 13 oct | Base documental | Corregir los formularios tomados por el mismo documento (P-074) | vigente |
| D-104 | 14 oct | Base documental | Un formulario PDF entrega su plantilla y sus datos: los campos se guardan aparte | vigente |
| D-105 | 14 oct | Base documental | Guardar el texto una vez por espacio no expone nada nuevo | vigente |
| D-106 | 14 oct | Parecidos | Las relaciones entre documentos también se infieren, no solo se miden | vigente |
| D-107 | 14 oct | Parecidos | "Ejemplar firmado" solo si la firma se puede verificar; vista previa como relación propia | vigente |
| D-108 | 14 oct | Base documental | Un catálogo de tipos de documento que crece de a poco | vigente |
| D-109 | 14 oct | Base documental | Las firmas electrónicas se reconocen con reglas | vigente |
| D-110 | 14 oct | Base documental | Los nombres de archivo son hipótesis; manda el contenido | vigente |
| D-111 | 14 oct | IA | El tipo de documento necesita el modelo y va temprano; cálculo y modelo se alternan | vigente |
| D-112 | 14 oct | Parecidos | Compartir una persona no es compartir un asunto | vigente |
| D-113 | 14 oct | Proceso de trabajo | No se hacen inferencias sobre inferencias | vigente |
| D-114 | 14 oct | Proceso de trabajo | Las reglas del procesamiento se documentan aparte y se revisan | vigente |
| D-115 | 14 oct | Parecidos | El proceso tiene que decir qué falta | vigente |
| D-116 | 14 oct | Parecidos | Texto de rutina: lo que aparece en la mitad de los documentos o más, y en al menos diez | en revisión |
| D-117 | 14 oct | Parecidos | "Comparten" se mide palabra por palabra sobre el texto alineado; las secuencias de cinco palabras solo buscan candidatos | en revisión |
| D-118 | 14 oct | Parecidos | Misma plantilla si se reemplaza un identificador o un importe, o si lo reemplazado es mayormente datos | en revisión |
| D-119 | 14 oct | Parecidos | Firma de una parte: la que no es del servicio, o la que registra el historial del servicio | en revisión |
| D-120 | 14 oct | Base documental | El catálogo de tipos y relaciones es un archivo de datos dentro de la app, reemplazable sin compilar | en revisión |
| D-121 | 14 oct | Parecidos | El principal de una familia es el firmado por una parte; si no, el de más texto | en revisión |
| D-122 | 14 oct | Parecidos | Alineación propia, sin sumar dependencias | en revisión |
| D-123 | 14 oct | Parecidos | Solo se comparan documentos con todo su texto; los que esperan reconocimiento quedan afuera | en revisión |
| D-124 | 14 oct | Proceso de trabajo | Las pruebas existentes dejan de usar nombres de la carpeta de prueba | en revisión |
| D-125 | 14 oct | Parecidos | Los nombres de las relaciones salen del vocabulario, con la frase que dice su dirección; una relación sin nombre se muestra por su identificador | en revisión |
| D-126 | 14 oct | Base documental | El vocabulario admite términos por país, que reemplazan solo lo que cambian y solo sobre su propio idioma | en revisión |
| D-127 | 14 oct | Parecidos | Qué cuenta como "espera una decisión" y cómo lo muestra Inicio: una línea por familia (hasta cinco) y una para el resto | en revisión |
| D-128 | 14 oct | Parecidos | Las relaciones que se pueden elegir: las del catálogo, medidas e inferidas, sin la automática ni "parecidos" | en revisión |
| D-129 | 14 oct | Parecidos | Qué fuente del wiki sugiere la app al unir dos documentos como el mismo, y cómo se elige | en revisión |
| D-130 | 14 oct | Parecidos | Ver las diferencias lado a lado queda para después, con el botón visible y deshabilitado | en revisión |
| D-131 | 14 oct | Parecidos | Cómo se ve cada parecido en el detalle de un documento: tarjeta, medidas en palabras y unir en un clic cuando no hay nada que elegir | en revisión |
| D-132 | 14 oct | Parecidos (catálogo de tipos) | Un servicio de firma o un formulario no es el tipo de un documento | en revisión |
| D-133 | 14 oct | Parecidos (catálogo de tipos) | Los tipos que se agregaron al catálogo | en revisión |
| D-134 | 14 oct | Parecidos (uso del modelo) | Qué pregunta se le hace al modelo por cada familia | en revisión |
| D-135 | 14 oct | Parecidos (uso del modelo) | Cuándo las marcas alcanzan y no se pregunta | en revisión |
| D-136 | 14 oct | Parecidos (uso del modelo) | La misma pregunta no se repite: cómo se reconoce | en revisión |
| D-137 | 14 oct | Parecidos (uso del modelo) | Respuesta del modelo en JSON estricto, con el camino de Codex que ya existía | en revisión |
| D-138 | 14 oct | Parecidos (uso del modelo) | Cuándo se le pregunta al modelo | en revisión |
| D-139 | 14 oct | Parecidos (pasada 5) | Cómo se buscan los datos de cada tipo | en revisión |
| D-140 | 14 oct | Parecidos (pasada 6) | Cómo corrigen las relaciones las reglas del tipo | en revisión |
| D-141 | 14 oct | Parecidos (medición) | El texto de las imágenes en la prueba real | en revisión |
| D-142 | 14 oct | Parecidos (catálogo de tipos, uso del modelo) | Las etapas se describen en el catálogo, y eso va en la pregunta | en revisión |
| D-143 | 14 oct | Parecidos | Al unir dos documentos como el mismo, el no elegido deja de ser fuente del wiki en todas sus ubicaciones; lo ya incorporado queda | en revisión |
| D-144 | 14 oct | Parecidos | Después de unir o deshacer, la cola sigue la decisión: sale lo que espera y ya no alimenta el wiki, y entra la fuente que nunca se incorporó | en revisión |
| D-145 | 14 oct | Parecidos | El aviso "no alimenta el wiki" va en la sección "En el wiki" del detalle, y vale cualquier par unido en que se eligió al otro | en revisión; reemplazada en parte por D-148 |
| D-146 | 14 oct | Base documental | En un formulario PDF, de cada casilla o botón marcado se escribe qué opción es | en revisión |
| D-147 | 14 oct | Base documental | El preprocesado recibe la ruta completa de la fuente, la misma que lee el ingest | en revisión |
| D-148 | 14 oct | Parecidos | Una elección de fuente que dejaría sin ninguna fuente a documentos unidos como el mismo se rechaza, con un mensaje que dice cómo salir | en revisión |
| D-149 | 15 oct | Distribución | La comprobación de los PR y los instaladores, solo en Windows por ahora | vigente |
| D-150 | 15 oct | IA | La IA se usa donde agrega valor o el proceso la necesita; lo determinista, solo si iguala o mejora el resultado | vigente |
| D-151 | 15 oct | Upstream | No se sincroniza con upstream: el proyecto diverge y la compatibilidad deja de ser un criterio | vigente |
| D-152 | 16 oct | Proceso de trabajo | `FORK_CHANGES.md` se congela: sin sincronización, ya no hace falta | vigente |
| D-153 | 16 oct | Proceso de trabajo | Estado "a verificar" para lo construido que solo espera que el dueño lo vea en la app | vigente |
| D-154 | 16 oct | Producto | El producto mínimo no está cerrado: el wiki generado es inservible y la experiencia de uso no alcanza | vigente |
| D-155 | 16 oct | Interfaz | El contador de Atención cuenta problemas por línea y decisiones por decisión; lo de baja prioridad no cuenta | en revisión (agente sin consulta, P-066) |
| D-156 | 16 oct | Interfaz | Las fuentes de la vista de atención son piezas de la pantalla, con una forma común; el motor da los datos | en revisión (agente sin consulta, P-066) |
| D-157 | 16 oct | Interfaz | El panel de actividad del wiki queda reducido a una línea que lleva a Atención | en revisión (agente sin consulta, P-066) |
| D-158 | 16 oct | Interfaz | En qué clase y con qué prioridad va cada aviso que no fijaba la propuesta | en revisión (agente sin consulta, P-066) |
| D-159 | 16 oct | Base documental | La vista de parecidos separa los pares fuera de las familias; confirmar una familia une solo al principal con sus miembros | en revisión (agente sin consulta, P-066) |
| D-160 | 16 oct | Base documental | Las diferencias lado a lado se calculan en la pantalla, palabra por palabra, sin guardarse | en revisión (agente sin consulta, P-066) |
| D-161 | 16 oct | Base documental | Los tipos confirmados se guardan en la base y se suman a la lista cerrada solo en las preguntas nuevas | en revisión (agente sin consulta, P-066) |
| D-162 | 16 oct | Base documental | Rechazar un tipo propuesto deja sus documentos sin tipo y se vuelven a preguntar diciendo que no lo proponga | en revisión (agente sin consulta, P-066) |
| D-163 | 16 oct | Wiki | La revisión de enlaces que corre sola reemplaza los resultados estructurales y conserva los del modelo | en revisión (agente sin consulta, P-066) |
| D-164 | 16 oct | Interfaz | Con otro proyecto abierto, los avisos del wiki del archivo se muestran sin botones | en revisión (agente sin consulta, P-066) |
| D-170 | 16 oct | Conocimiento | Una capa de conocimiento propia, dentro de la base del espacio, entre la base documental y el wiki (alternativa B) | vigente |
| D-171 | 16 oct | Conocimiento | Toda mención y afirmación del modelo trae su cita textual, y un cálculo comprueba que está en el texto; si no está, se descarta | vigente |
| D-172 | 16 oct | Conocimiento | Tipos iniciales persona, organización y asunto, con el rol de cada mención; tipos, roles y predicados son datos | vigente |
| D-173 | 16 oct | Conocimiento | Identidad en cascada con abstención; el mismo identificador une solo y se puede deshacer; uno distinto nunca une | vigente |
| D-174 | 16 oct | Conocimiento | Las decisiones de una persona sobre identidades son firmes, reversibles y no las pisa un recálculo | vigente |
| D-175 | 16 oct | Conocimiento | La página de una entidad es una proyección del registro; la incorporación del wiki deja de crear páginas de entidad | vigente |
| D-176 | 16 oct | Conocimiento | Sin derivaciones por reglas por ahora; todo incremental; costo de modelo aprobado | vigente |

---

## Entradas

### D-001 · Micelya Desktop, un producto para estudios jurídicos

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Producto
- **Estado:** vigente
- **El problema:** se partió de una aplicación de escritorio de código abierto que lee documentos y arma con IA un wiki (un conjunto de páginas enlazadas que resumen lo leído). Había que decidir si se la usaba tal cual o se la convertía en algo propio.
- **Qué se decidió:** la copia propia (el fork `Micelya/llm_wiki`) se convierte en **Micelya Desktop**, un producto aparte orientado a estudios de abogados.
- **Por qué:** la base sirve, pero el uso jurídico pide capacidades que la app original no tiene (manejo de fuentes, duplicados, documentos parecidos).
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** todo el repositorio; nombre e identidad en D-018.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 01, sección 1.

### D-002 · Los documentos quedan en su carpeta original, sin copiarse

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Base documental
- **Estado:** vigente (hoy es el principio 2 del diseño: "los originales no se tocan")
- **El problema:** la app original copia cada documento dentro de la carpeta del proyecto. En un estudio eso duplica miles de archivos y deja dos versiones que pueden divergir.
- **Qué se decidió:** las fuentes quedan donde están. La app las lee desde su ubicación original y solo guarda lo que obtiene al procesarlas (texto, resúmenes). Los originales nunca se escriben, mueven ni borran.
- **Por qué:** evitar copias y no arriesgar los documentos del cliente.
- **Alternativas descartadas:** la importación con copia de la app original.
- **Dónde se aplica:** primero como "volumen de fuentes" (PR #7, F-002, botón *Mount folder*); hoy en la base documental (`src-tauri/src/docbase/`).
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 01, sección 1, objetivo 1; `base-documental.md`, sección 2.

### D-003 · El mismo documento se procesa una sola vez

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** en un estudio el mismo documento aparece muchas veces: en varias carpetas, en Word y en PDF, enviado por mail y guardado con otro nombre. Procesar cada copia con IA gasta cupo y llena el wiki de repeticiones.
- **Qué se decidió:** detectar las copias y procesar el documento una sola vez, conservando todas sus ubicaciones.
- **Por qué:** costo y orden; además, saber dónde están todas las copias tiene valor propio.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** catálogo (PR #15, #17, F-011) y luego la base documental (PR #24 a #31).
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 01, sección 1, objetivo 2.

### D-004 · El parecido solo propone; importa qué cambió, no cuánto

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Parecidos
- **Estado:** vigente (lo retoman D-094 a D-098)
- **El problema:** dos contratos pueden compartir casi todo el texto y ser documentos distintos (mismo modelo, otro cliente). Si la app los tratara como duplicados mezclaría clientes.
- **Qué se decidió:** el parecido entre documentos nunca los fusiona: solo propone una relación. Para clasificarla se mira **qué** cambió entre ellos (una cifra, un nombre, redacción), no el porcentaje de texto en común.
- **Por qué:** el cuidado legal: en un estudio, dos documentos casi iguales no son duplicados.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-015; `docs/design/parecidos-propuesta.md`.
- **Quién decidió:** dueño del producto (objetivo 3); la formulación "nunca fusiona" figura entre las "decisiones de diseño ya tomadas" sin decir quién la propuso.
- **Fuente:** sesión 01, secciones 1 y 7.

### D-005 · Versiones como familias; "vigente" opcional; todos van al wiki

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Parecidos
- **Estado:** reemplazada en parte por D-098 (sigue valiendo lo de la familia y la etiqueta "vigente"; cambió "todos los miembros se ingieren")
- **El problema:** no siempre se puede saber cuál de varias versiones de un documento es la vigente ni cuál vino primero.
- **Qué se decidió:** lo seguro es la *familia* de documentos y el tipo de relación entre ellos. "Vigente" es una etiqueta opcional: la app la sugiere si hay señales claras y, si no, queda sin definir. Todos los miembros de una familia van al wiki; solo los duplicados exactos se procesan una vez.
- **Por qué:** no afirmar lo que el cálculo no puede saber.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-015; `parecidos-propuesta.md`, sección 6 ("la app no decide cuál versión es la vigente").
- **Quién decidió:** No registrado (figura como "decisión de diseño ya tomada").
- **Fuente:** sesión 01, sección 7.

### D-006 · Separar el documento (contenido) del archivo en una ruta (ubicación)

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Base documental
- **Estado:** vigente (ampliado en D-038)
- **El problema:** "archivo" se usaba para dos cosas distintas: el documento en sí y cada lugar donde está guardado. Sin separarlas no se puede decir "este documento está en tres carpetas".
- **Qué se decidió:** el catálogo distingue *contenido* (un documento único) de *ubicación* (un archivo en una ruta). Un contenido puede tener varias ubicaciones.
- **Por qué:** es la condición para deduplicar (D-003) sin perder ninguna copia.
- **Alternativas descartadas:** identificar documentos por su ruta, como hacía la app original.
- **Dónde se aplica:** catálogo (F-011) y base documental.
- **Quién decidió:** No registrado.
- **Fuente:** sesión 01, sección 7.

### D-007 · Motores de lectura y orígenes intercambiables

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** los documentos van a venir de lugares distintos (carpetas locales, servidores, nubes) y los escaneos se pueden leer con servicios distintos, que cambian de precio y calidad.
- **Qué se decidió:** dos piezas reemplazables: una para "de dónde vienen las fuentes" y otra para "quién lee las imágenes". Cambiar de proveedor no obliga a tocar el resto.
- **Por qué:** no quedar atado a un proveedor y poder comparar (P-026).
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `src-tauri/src/source_volume/` (`SourceProvider`, `TextRecognizer`); diseño, sección 7 ("motores de lectura intercambiables").
- **Quién decidió:** No registrado.
- **Fuente:** sesión 01, sección 7.

### D-008 · Comparar por secuencias de cinco palabras y medida de contención

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Parecidos
- **Estado:** vigente
- **El problema:** hace falta un método de cálculo, sin IA, para saber cuánto texto comparten dos documentos y si uno está dentro del otro.
- **Qué se decidió:** cortar cada texto en secuencias de cinco palabras seguidas y compararlas con un resumen numérico (MinHash), más una medida de *contención* (qué parte del documento corto está dentro del largo). Se usó así en una prueba previa.
- **Por qué:** funcionó en la prueba de concepto y es barato de calcular.
- **Alternativas descartadas:** otro método de resumen (SimHash): "no hace falta".
- **Dónde se aplica:** P-015; `parecidos-propuesta.md`, sección 5 (firma y casilleros).
- **Quién decidió:** No registrado.
- **Fuente:** sesión 01, sección 7.

### D-009 · Proveedor de IA: desarrollo y clientes

- **Fecha:** 5 al 8 oct 2026 (estudio del 7 oct)
- **Tema:** IA
- **Estado:** vigente, con una consulta abierta (P-029)
- **El problema:** hay que decidir con qué servicio de IA se leen los documentos, cuidando qué permiten los términos de cada proveedor y que el contenido no se use para entrenar.
- **Qué se decidió:** durante el desarrollo se usa Codex con la suscripción ChatGPT del dueño (con el entrenamiento con contenido desactivado). Para clientes, o aportan su propia suscripción (sujeto a consultar los términos, P-029) o se usa el servicio de IA de Amazon (Bedrock) detrás de la API de Micelya.
- **Por qué:** el estudio de términos mostró que usar suscripciones personales en un producto comercial no está claramente permitido; Bedrock tiene términos comerciales y no entrena con el contenido.
- **Alternativas descartadas:** la suscripción de claude.ai, porque Anthropic prohíbe que aplicaciones de terceros usen credenciales de planes Free, Pro o Max.
- **Dónde se aplica:** Configuración de la app; P-026, P-029; más adelante Micelya Core (P-063).
- **Quién decidió:** No registrado explícitamente; figura como "decisión vigente" tras un estudio pedido por el dueño.
- **Fuente:** sesión 01, sección 6 ("Proveedores para el reconocimiento de texto").

### D-010 · Leer con un motor intercambiable las páginas escaneadas y las fotos

- **Fecha:** 5 al 8 oct 2026
- **Tema:** IA
- **Estado:** vigente
- **El problema:** un PDF escaneado o una foto no tienen texto que extraer; sin leerlos, quedan afuera del wiki.
- **Qué se decidió:** las páginas de PDF con menos de 80 caracteres y las imágenes PNG/JPEG se leen con un motor intercambiable (primero Codex). Lo leído se guarda por página; un fallo nunca deja una página como vacía; el texto leído lleva una marca con el motor y el modelo que lo leyó. Se activa, de forma provisoria, con el ajuste *Image captioning* (P-028 pide un ajuste propio).
- **Por qué:** incluir los escaneos sin perder rastro de que su texto es menos confiable (la auditoría encontró un 6 % de cifras mal leídas, P-030).
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #7, F-006; `src-tauri/src/source_volume/recognition*.rs`.
- **Quién decidió:** No registrado.
- **Fuente:** sesión 01, sección 4.

### D-011 · Upstream: la arquitectura pesa más que evitar choques

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Upstream
- **Estado:** reemplazada por D-034
- **El problema:** cada cambio propio dentro de archivos del proyecto original hace más difícil traer después sus mejoras ("choques").
- **Qué se decidió:** la calidad de la arquitectura pesa más que evitar choques; los choques se abaratan, no se evitan a toda costa. La lógica nueva va en archivos propios y lo que se toca en archivos originales son solo llamadas a nuestros módulos.
- **Por qué:** poder seguir trayendo cambios del original sin sacrificar el diseño.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** forma de programar; `FORK_CHANGES.md`.
- **Quién decidió:** dueño del producto (regla de trabajo acordada).
- **Fuente:** sesión 01, sección 3.

### D-012 · Registros: diferencias con upstream, pendientes y traspasos

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** reemplazada en parte por D-152 (el registro de diferencias con upstream se congela; los pendientes y los traspasos siguen)
- **El problema:** cada conversación con el asistente empieza sin memoria de las anteriores; sin registros escritos se pierde qué se hizo y qué falta.
- **Qué se decidió:** cada cambio respecto del original se anota por propósito en `FORK_CHANGES.md`; cada tarea que no se resuelve en el momento, en `pendientes.json`; cada sesión cierra con un documento de traspaso en `docs/sessions/`; `CLAUDE.md` le indica al asistente que al empezar lea el último traspaso, el diseño y los pendientes, y liste los pendientes abiertos.
- **Por qué:** continuidad entre sesiones sin releer conversaciones.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `FORK_CHANGES.md`, `pendientes.json`, `docs/sessions/`, `CLAUDE.md` (PR #2 y #4).
- **Quién decidió:** dueño del producto (reglas acordadas).
- **Fuente:** sesión 01, secciones 3 y 4; `CLAUDE.md`.

### D-013 · Todo cambio entra por PR contra el fork

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente
- **El problema:** sin control, un cambio sin revisar puede romper la versión principal; además, la herramienta de GitHub por defecto abre los pedidos contra el proyecto original.
- **Qué se decidió:** `main` está protegida y todo entra por PR con la comprobación automática en verde; cambios no relacionados van en PR separados; los PR se abren explícitamente contra `Micelya/llm_wiki`; nunca se suben las etiquetas del proyecto original (`git push --tags`); las pruebas se corren a mano antes de cada PR. El dueño integra y borra las ramas, y pide verificar el repositorio después de cada integración.
- **Por qué:** revisión, posibilidad de volver atrás y no mandar nada por error al proyecto original.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** GitHub (regla de protección configurada por el dueño); `CLAUDE.md`. Si la comprobación automática debe además correr las pruebas sigue abierto (P-046).
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 01, sección 3; sesión 03, sección 8.

### D-014 · Explicaciones en español, simples, autocontenidas y verificadas

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente
- **El problema:** el dueño del producto no es especialista en el código; una explicación técnica o que depende de contexto anterior no le sirve para decidir.
- **Qué se decidió:** explicaciones en español, en términos simples y autocontenidas; primero el entendimiento (el problema), después la propuesta. Toda afirmación se verifica contra los proyectos reales antes de sostenerla.
- **Por qué:** que el dueño pueda decidir con conocimiento.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** toda la comunicación y documentación.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 01, sección 3; sesión 03, sección 8.

### D-015 · Endurecer los procesos automáticos de GitHub

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Seguridad
- **Estado:** vigente
- **El problema:** los procesos automáticos de GitHub (*workflows*, que compilan y publican) usan piezas de terceros; una pieza comprometida podría robar secretos o alterar lo que se publica. Además, las etiquetas traídas del original podían disparar publicaciones acá.
- **Qué se decidió:** fijar cada acción de terceros a una versión exacta e inmutable, dar los permisos mínimos y publicar versiones solo si existe la variable `ENABLE_TAG_RELEASE`. El dueño configuró a mano la protección de `main`, la obligación de versiones exactas, la aprobación para colaboradores externos y verificó que no haya secretos en la organización.
- **Por qué:** una revisión de seguridad no encontró ataques, pero sí puntos débiles.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #3, F-003; `.github/workflows/ci.yml` y `build.yml`.
- **Quién decidió:** agente propuso; dueño integró y configuró.
- **Fuente:** sesión 01, sección 4.

### D-016 · Actualización mensual de las acciones de GitHub, revisada con demora

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Seguridad
- **Estado:** vigente
- **El problema:** fijar las acciones a versiones exactas (D-015) las congela: no reciben arreglos.
- **Qué se decidió:** un servicio de GitHub (Dependabot) propone una vez por mes un PR agrupado con versiones nuevas, solo para acciones. No se integra el mismo día: se esperan unos días por si la versión nueva resulta comprometida.
- **Por qué:** equilibrio entre estar al día y no tomar una versión maliciosa recién publicada.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #6, F-005; `.github/dependabot.yml`; P-005.
- **Quién decidió:** agente propuso; dueño integró.
- **Fuente:** sesión 01, sección 4; P-005.

### D-017 · No cargar claves de firma hasta proteger su uso

- **Fecha:** 6 oct 2026
- **Tema:** Seguridad
- **Estado:** vigente
- **El problema:** para distribuir en Mac hay que cargar en GitHub las claves de firma de Apple; durante la compilación quedan visibles para todas las piezas de terceros.
- **Qué se decidió:** no cargar claves ni secretos hasta crear un entorno con aprobación manual, restringido a etiquetas, y con certificado y contraseña exclusivos del fork.
- **Por qué:** reducir el daño si una pieza de terceros está comprometida.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-002. Consecuencia: el instalador de Mac no se firma (D-092). Firmar los de Windows es una decisión abierta (P-022).
- **Quién decidió:** dueño del producto (pendiente a su cargo).
- **Fuente:** P-002.

### D-018 · Identidad y numeración de versiones propias

- **Fecha:** 5 al 8 oct 2026
- **Tema:** Distribución
- **Estado:** vigente
- **El problema:** con el nombre y el identificador del original, la app compartiría su configuración con la app original instalada y recibiría avisos de sus actualizaciones.
- **Qué se decidió:** nombre "Micelya Desktop", identificador `com.micelya.desktop`, versión propia desde `0.1.0`, etiquetas de versión `mi-desktop-vX.Y.Z` (las de prueba con `-rc.N`; la primera será `mi-desktop-v0.1.0-rc.1`) y aviso de actualizaciones contra este repositorio.
- **Por qué:** que las dos apps no se pisen y que el producto tenga identidad propia.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #5 y #11, F-004; P-006.
- **Quién decidió:** No registrado; integrado por el dueño.
- **Fuente:** sesión 01, sección 4; P-006.

### D-019 · Catálogo: cuatro propuestas aceptadas (P-039)

- **Fecha:** 8 oct 2026
- **Tema:** Base documental
- **Estado:** vigente (el catálogo por proyecto luego pasó a la base del espacio, D-035)
- **El problema:** antes de construir la detección de duplicados había cuatro preguntas abiertas: cuándo calcular, qué hacer con un duplicado, si contar el texto leído de escaneos y dónde hacer el cálculo.
- **Qué se decidió:** (1) las huellas se calculan antes de gastar cupo del modelo; (2) un duplicado exacto tiene una sola página de fuente, que lista todas sus ubicaciones (el archivo se agrega a las fuentes de todas las páginas que produjo el documento, así una página se borra solo cuando no le queda ninguna fuente); (3) el texto leído de imágenes entra en la comparación, marcado como menos confiable; (4) el cálculo corre en el motor de la app (Rust).
- **Por qué:** que un duplicado nunca llegue al modelo y que el borrado siga funcionando.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #15, F-011.
- **Quién decidió:** dueño del producto (aceptó las propuestas del agente).
- **Fuente:** sesión 02, secciones 2 y 4.

### D-020 · "Mismo documento" = mismo texto normalizado

- **Fecha:** 8 oct 2026
- **Tema:** Base documental
- **Estado:** vigente (con el defecto de formularios de D-103)
- **El problema:** un Word y el PDF que salió de él tienen bytes distintos pero son el mismo documento; hay que poder reconocerlo sin confundir documentos distintos.
- **Qué se decidió:** el texto se normaliza (minúsculas, sin tildes, solo letras y dígitos) y se calcula su huella; dos archivos con la misma huella de texto son un documento. Textos de menos de 30 palabras no se catalogan. Solo la coincidencia exacta une: una cifra distinta es otro documento.
- **Por qué:** seguridad legal; lo "casi igual" queda para la etapa de parecidos.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #15; luego la base documental (PR #27). Aclarado el 13 oct (P-015): las diferencias de formato ya no cuentan, pero una sola palabra distinta sí.
- **Quién decidió:** No registrado (figura en "decisiones de diseño tomadas en la sesión").
- **Fuente:** sesión 02, sección 4; P-015.

### D-021 · Calcular la huella del texto al empezar la ingesta

- **Fecha:** 8 oct 2026
- **Tema:** Base documental
- **Estado:** reemplazada por D-024 (la comparación por texto sí sigue dentro de la ingesta)
- **El problema:** lo acordado (D-019) era calcular la huella al montar o importar, pero un escaneo todavía no tiene texto en ese momento (se lee después, D-026).
- **Qué se decidió:** calcular la huella al empezar la tarea de ingesta de cada fuente, después de leer su texto y antes de llamar al modelo. Efecto visible: el duplicado aparece en la cola y se resuelve ahí.
- **Por qué:** es el único punto por el que pasan todos los caminos y compara exactamente el texto que se ingiere; un duplicado igual nunca llega al modelo.
- **Alternativas descartadas:** calcular al importar (lo acordado originalmente).
- **Dónde se aplica:** PR #15.
- **Quién decidió:** agente sin consulta; quedó "pendiente de visto bueno" y el dueño respondió con una propuesta propia (D-024).
- **Fuente:** sesión 02, sección 2 ("Una diferencia con lo decidido") y sección 9.

### D-022 · Avanzar con todos los pendientes de código, en commits parciales

- **Fecha:** 8 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente para esa etapa (después la prioridad pasó al producto mínimo, D-053)
- **El problema:** tras la demostración del 8 de octubre había muchas tareas de código acumuladas.
- **Qué se decidió:** avanzar con todas, en cambios guardados por partes, para poder volver atrás alguno.
- **Por qué:** avanzar rápido sin perder control.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #10 a #16.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 02, sección 2.

### D-023 · Un origen vacío o desconectado no es un origen sin archivos

- **Fecha:** 8 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** si un disco en la nube tarda en arrancar o se desconecta, su carpeta parece vacía. Si la app lo tomara como "se borraron todos los archivos", borraría las páginas del wiki.
- **Qué se decidió:** una carpeta que no está, o que aparece vacía cuando se le conocían archivos, se trata como "no disponible" y se conserva lo que se sabía. Consecuencia aceptada: vaciar a propósito una carpeta entera no borra páginas; hay que quitar la carpeta de la app.
- **Por qué:** no perder trabajo por una desconexión.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #12 (`source_volume/watch.rs`); luego la base documental (sesión 05, sección 4); diseño, sección 5.
- **Quién decidió:** No registrado ("consecuencia aceptada", sin decir por quién).
- **Fuente:** sesión 02, sección 3 ("Detección de cambios").

### D-024 · Huella del archivo primero, antes de encolar

- **Fecha:** 8 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** con D-021 un archivo idéntico a otro ya procesado entraba a la cola y se leía antes de descartarse. Además, al reiniciar, tres imágenes se volvieron a leer con IA, salieron distintas y se reprocesaron.
- **Qué se decidió:** primero se compara la huella de los bytes del archivo (SHA-256), antes de encolarlo y sin leerlo. La comparación por texto queda en la ingesta, para el mismo documento en otro formato o escaneado dos veces.
- **Por qué:** más barato y resuelve la diferencia de D-021: para archivos idénticos vuelve a valer "comparar al importar".
- **Alternativas descartadas:** solo comparar por texto dentro de la ingesta (D-021).
- **Dónde se aplica:** PR #17, F-011; hoy en la base (sesión 05: niveles ubicación, archivo y contenido).
- **Quién decidió:** dueño del producto (propuesta suya).
- **Fuente:** sesión 02, sección 9; P-014.

### D-025 · Texto leído de imágenes guardado por huella, por proyecto, en `raw/`, en JSON

- **Fecha:** 8 oct 2026
- **Tema:** IA
- **Estado:** reemplazada en parte por D-035 (sigue valiendo guardar por huella y que no es descartable; el lugar pasó del proyecto a la base del espacio)
- **El problema:** el texto leído de escaneos se guardaba atado a la ruta; un archivo movido o renombrado se volvía a leer con IA, con costo y resultados distintos.
- **Qué se decidió:** guardar el texto leído por huella del archivo en `raw/recognized/` dentro del proyecto, una entrada por página o imagen, con el motor que la leyó. Lo ya leído se adopta sin releer.
- **Por qué:** por proyecto y no global, para no mezclar texto de distintos clientes; en `raw/` y no en la carpeta de caché, porque no es descartable; en JSON, para que la herramienta de notas que abre el proyecto no lo muestre como notas.
- **Alternativas descartadas:** guardado global, en la carpeta de caché, en archivos de notas `.md`.
- **Dónde se aplica:** PR #17, P-047. Con la base, el texto leído vuelve a la base (sesión 04 puente; P-071).
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 02, sección 9; P-047.
- **Nota (14 oct):** la preocupación de no mezclar clientes quedó respondida en D-105: guardar el texto una vez no cambia quién puede verlo; lo que importa es qué recibe junto el modelo, y eso lo gobierna el alcance de cada proyecto.

### D-026 · El reconocimiento corre solo dentro de la ingesta, con avance

- **Fecha:** 8 oct 2026
- **Tema:** IA
- **Estado:** vigente para el wiki (la base, por su parte, no lee imágenes por su cuenta: D-059)
- **El problema:** al montar una carpeta la pantalla quedaba bloqueada mientras se leían los escaneos, sin avance visible; algunas imágenes se leían dos veces.
- **Qué se decidió:** la lectura de imágenes corre solo dentro de la tarea de ingesta de cada fuente, con avance por página ("página 3 de 7"); la importación ya no espera.
- **Por qué:** no bloquear la app y no leer dos veces.
- **Alternativas descartadas:** leer al importar (comportamiento anterior).
- **Dónde se aplica:** PR #14, P-034.
- **Quién decidió:** No registrado (pendiente surgido de la prueba del dueño).
- **Fuente:** sesión 02, secciones 1 y 4.

### D-027 · Nombre del producto con un único reemplazo al traducir

- **Fecha:** 8 oct 2026
- **Tema:** Interfaz
- **Estado:** vigente
- **El problema:** la interfaz seguía diciendo "LLM Wiki" en muchos textos, repartidos en cuatro archivos de idioma que el proyecto original cambia seguido.
- **Qué se decidió:** un único reemplazo del nombre al momento de mostrar los textos, sin editar los archivos de idioma. Se dejan a propósito el crédito al proyecto original, los textos que se le envían al modelo, la extensión `.llmwiki.zip` y una variable técnica.
- **Por qué:** cambiar el nombre sin generar choques con el original.
- **Alternativas descartadas:** editar los cuatro archivos de idioma.
- **Dónde se aplica:** PR #11, P-021, F-004 (`src/i18n/product-name.ts`).
- **Quién decidió:** agente.
- **Fuente:** sesión 02, sección 4; P-021.

### D-028 · Chat con proveedores de línea de comandos: busca y recibe el texto de las páginas

- **Fecha:** 8 oct 2026
- **Tema:** IA
- **Estado:** vigente (con límites en P-043)
- **El problema:** con Codex el chat respondía "no se encontró evidencia": la búsqueda en el wiki se apagaba y, además, el modelo recibía solo el título y unos 160 caracteres de cada página.
- **Qué se decidió:** con estos proveedores no se envían las "skills" del modo automático (apagaban la búsqueda); se agrega al pedido el texto de las páginas encontradas; se piden hasta 10 páginas en todos los modos salvo el rápido.
- **Por qué:** que el chat responda con el contenido real del wiki. El dueño confirmó que ya responde con importes.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #8 y #13, F-007, P-031, P-032.
- **Quién decidió:** agente; resultado confirmado por el dueño.
- **Fuente:** sesión 02, secciones 1 y 3.

### D-029 · Buscador que ignora palabras vacías del español

- **Fecha:** 8 oct 2026
- **Tema:** Interfaz
- **Estado:** vigente (falta medir la mejora en la app)
- **El problema:** una pregunta en español arrastraba palabras como "por", "qué", "de", que aparecen en casi todas las páginas y desordenaban los resultados.
- **Qué se decidió:** el buscador descarta esas palabras (con y sin tilde) y los signos ¿ ¡ « ». La lista vive en un módulo propio; el buscador original solo la consulta.
- **Por qué:** mejores resultados en español, con un cambio mínimo en código del original.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #13, F-009, P-033.
- **Quién decidió:** agente.
- **Fuente:** sesión 02, sección 4.

### D-030 · Grafo con etiquetas solo en los nodos principales

- **Fecha:** 8 oct 2026
- **Tema:** Interfaz
- **Estado:** en revisión (el dueño tiene que verlo y decir si se ajustan los umbrales)
- **El problema:** en el grafo (el dibujo de las páginas y sus enlaces) los nombres se superponían y no se podían leer.
- **Qué se decidió:** solo los nodos más conectados llevan nombre sin acercar la vista; los títulos largos se acortan; un clic resalta el nodo y sus vecinos.
- **Por qué:** legibilidad.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #9, F-008, P-037.
- **Quién decidió:** agente.
- **Fuente:** sesión 02, sección 1; P-037.

### D-031 · Aviso de importación informativo cuando no hubo error

- **Fecha:** 8 oct 2026
- **Tema:** Interfaz
- **Estado:** vigente (falta verlo en la app)
- **El problema:** el aviso "Importados 31, omitidos 2" se mostraba en rojo aunque las omisiones eran correctas (archivos de sistema).
- **Qué se decidió:** estilo informativo cuando todas las omisiones se deben a las reglas de exclusión; cualquier otro motivo sigue en estilo de error.
- **Por qué:** no alarmar sin motivo.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #10, F-010, P-038.
- **Quién decidió:** agente.
- **Fuente:** sesión 02, sección 1.

### D-032 · Programar en una carpeta separada de la que corre la app

- **Fecha:** 8 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente
- **El problema:** la app de prueba corre desde la carpeta del repositorio y se recarga ante cualquier cambio de archivo; programar ahí la interrumpía.
- **Qué se decidió:** el código se trabaja en otra carpeta (`llm_wiki_wt`) del mismo repositorio. Tiene enlaces hacia la principal: no se borra con un borrado común.
- **Por qué:** poder programar con la app abierta.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** entorno de trabajo.
- **Quién decidió:** agente.
- **Fuente:** sesión 02, sección 6; sesión 03, sección 9.

### D-033 · Cambio de rumbo: la base documental es el cimiento

- **Fecha:** 9 oct 2026
- **Tema:** Producto
- **Estado:** vigente
- **El problema:** el producto venía creciendo como "una app de wiki con manejo de fuentes", con todo atado a cada proyecto. Eso no responde a lo que necesita un estudio: saber qué documentos tiene, dónde está cada copia y cuáles son el mismo.
- **Qué se decidió:** el cimiento pasa a ser una **base documental** centrada en el archivo. El wiki, el chat y el grafo se montan encima. Las tareas pensadas por proyecto (P-012, P-015, P-016, P-040, P-044) se replantean sobre la base.
- **Por qué:** el valor central para un estudio es el control de sus documentos; el wiki es uno de sus usos.
- **Alternativas descartadas:** seguir con el modelo por proyecto.
- **Dónde se aplica:** `docs/design/base-documental.md` (fuente de verdad del rumbo); PR #18 a #20.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 02, sección 10; sesión 03, secciones 1 y 4.

### D-034 · Las necesidades propias pesan más que la compatibilidad con upstream

- **Fecha:** 9 oct 2026
- **Tema:** Upstream
- **Estado:** reemplazada por D-151 (antes reemplazó a D-011)
- **El problema:** con el cambio de rumbo, el fork se aleja mucho del original; cuidar la compatibilidad empezaba a condicionar el diseño.
- **Qué se decidió:** las necesidades propias pesan más que la compatibilidad. Las novedades del original se analizan una por una; ya no se da por hecho que entren con las herramientas automáticas de GitHub. El original no condiciona el diseño.
- **Por qué:** el producto propio es lo prioritario.
- **Alternativas descartadas:** la regla anterior (D-011).
- **Dónde se aplica:** diseño, principio 8 y sección 9 ("Upstream").
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 02, sección 10; sesión 03, sección 8.

### D-035 · La unidad es el espacio, no el proyecto

- **Fecha:** 9 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** hasta ahí cada proyecto guardaba su propio catálogo, su texto leído y su detección de cambios. El mismo escaneo usado en dos proyectos se leía dos veces.
- **Qué se decidió:** la unidad es el **espacio**: privado (de una persona) o compartido (de varias). Cada documento se procesa una vez por espacio. Lo construido por proyecto es la semilla y migra a la base del espacio.
- **Por qué:** procesar una vez y tener una sola vista del archivo de cada persona o estudio.
- **Alternativas descartadas:** datos por proyecto (lo que existía, incluido D-025).
- **Dónde se aplica:** diseño, secciones 2, 9 y 12; base documental (`docbase/private.sqlite3`).
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 02, sección 10; sesión 03, sección 4.

### D-036 · La base documental es un componente separado

- **Fecha:** 9 oct 2026 (consolidado el 11 oct)
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** si la base se mezcla con el código del wiki, no se puede reutilizar ni cambiar por separado.
- **Qué se decidió:** la base es un componente separado, con datos propios, código en un lugar propio y una interfaz angosta (D-039). Reutiliza extractores de texto, proveedores de IA, cola, búsqueda e interfaz; reemplaza la importación con copia, la vigilancia de fuentes y la identificación por ruta.
- **Por qué:** poder evolucionarla sin depender del wiki.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `src-tauri/src/docbase/`, `src/components/docbase/`; F-013.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 02, sección 10; diseño, secciones 4 y 9. **Ambigüedad:** el 9 oct se escribió "componente separado del fork; el fork es su primer cliente", y el diseño consolidado dice "separado dentro de la misma app". No consta si es un cambio de decisión o una precisión.

### D-037 · Autorización al estilo UMA 2.0

- **Fecha:** 9 oct 2026
- **Tema:** Seguridad
- **Estado:** reemplazada por D-045
- **El problema:** en un espacio compartido, cada persona debe ver solo lo que tiene permitido.
- **Qué se decidió:** prever desde el principio una autorización al estilo del estándar UMA 2.0 (un servicio que administra permisos). Las investigaciones de autorización y de red entre instancias quedaron a cargo del dueño (P-048).
- **Por qué:** pensar la seguridad desde el inicio.
- **Alternativas descartadas:** No registradas en ese momento.
- **Dónde se aplica:** diseño (luego cambiado).
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 02, sección 10.

### D-038 · Tres niveles: ubicación, contenido, familia

- **Fecha:** 9 al 11 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** "archivo" se usa para cosas distintas; mezclarlas produce errores de diseño.
- **Qué se decidió:** *ubicación* (un archivo en una ruta; lo que se vigila), *contenido* (un documento exacto; de él cuelgan los derivados como texto y resumen) y *familia* (contenidos emparentados; de ella cuelgan las relaciones y la plantilla). En la construcción (sesión 05) se agregó un nivel intermedio, *archivo* (los mismos bytes exactos), entre ubicación y contenido.
- **Por qué:** amplía D-006 para soportar parecidos y familias.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** diseño, sección 3; base documental.
- **Quién decidió:** dueño del producto (diseño acordado).
- **Fuente:** sesión 03, sección 4; diseño, sección 3; sesión 05, sección 4.

### D-039 · Cuatro capas; el wiki le pregunta a la base solo tres cosas

- **Fecha:** 9 al 11 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** había que ordenar cómo se conectan los orígenes, la base y el wiki sin rehacer toda la parte del wiki heredada del original.
- **Qué se decidió:** cuatro capas: orígenes (solo lectura), base documental, volumen virtual (cada proyecto ve una selección de documentos) y usos (wiki, chat, grafo, pantallas). El wiki le pregunta a la base solo: qué fuentes hay, cuál es el texto de una y qué cambió. Las páginas siguen nombrando sus fuentes por una ruta lógica.
- **Por qué:** una conexión angosta deja que el wiki siga pareciéndose al original.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** diseño, sección 4.
- **Quién decidió:** dueño del producto (diseño acordado).
- **Fuente:** diseño, sección 4.

### D-040 · SQLite local como fuente de verdad; búsqueda por significado aparte

- **Fecha:** 9 al 11 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** había que elegir dónde guardar los datos de la base.
- **Qué se decidió:** una base SQLite (un archivo de base de datos local) es la fuente de verdad, con su búsqueda por palabras; la búsqueda por significado (vectores) sigue en el componente que ya tenía la app, como índice reconstruible. Las bases van siempre en una carpeta local de la app, nunca en una carpeta de red ni sincronizada con la nube.
- **Por qué:** simple, local y sin servidor; una base en carpeta sincronizada puede corromperse.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `%APPDATA%\com.micelya.desktop\docbase\private.sqlite3`; PR #24.
- **Quién decidió:** dueño del producto (diseño acordado).
- **Fuente:** sesión 03, sección 4; diseño, secciones 8 y 9.

### D-041 · Orígenes admitidos; documentos nativos de Google detectados, no procesados

- **Fecha:** 9 al 11 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** los estudios guardan documentos en carpetas locales, servidores y nubes. Los documentos nativos de Google, vistos desde la unidad montada, son accesos directos de 190 bytes sin contenido.
- **Qué se decidió:** se admiten carpetas locales, el servidor del estudio y Drive y OneDrive como unidad montada. Los documentos nativos de Google se registran como "detectado, no procesable", para que se vea qué queda afuera. Conectarse a Drive y OneDrive por sus propias interfaces queda para más adelante.
- **Por qué:** conectarse por sus interfaces exige registrar la aplicación ante cada proveedor, trámite que el dueño decidió no encarar todavía; percepción a validar: los estudios argentinos usan poco esos formatos.
- **Alternativas descartadas (postergadas):** conexión directa a Drive/OneDrive (P-056).
- **Dónde se aplica:** diseño, sección 5; P-056.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 03, secciones 4 y 5; P-056.

### D-042 · Determinista primero; IA una vez por grupo

- **Fecha:** 9 al 11 oct 2026
- **Tema:** IA
- **Estado:** reemplazada en parte por D-150. Sigue valiendo: cuando un grupo de documentos comparte la respuesta, se le pregunta al modelo una vez por grupo. Deja de valer: "determinista primero" como orden; ahora el principio es usar la herramienta que da el mejor resultado
- **El problema:** la IA cuesta, tarda e inventa; muchas cosas se pueden saber con cálculo exacto.
- **Qué se decidió:** primero herramientas deterministas (cálculo que da siempre el mismo resultado). La IA entra donde el cálculo no alcanza, y una vez por grupo de documentos, no por documento. La clasificación por tipo de documento se hace con IA, una vez por grupo.
- **Por qué:** costo, velocidad y certeza.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** diseño, principio 5 y secciones 7 y 9.
- **Quién decidió:** dueño del producto (diseño acordado).
- **Fuente:** diseño, secciones 2, 7 y 9.

### D-043 · Sin ninguna ubicación, se borran los derivados

- **Fecha:** 9 al 11 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** si un documento desaparece de todos lados, qué pasa con su texto, resumen y demás.
- **Qué se decidió:** cuando desaparecen todas las ubicaciones de un contenido, se borran sus derivados.
- **Por qué:** no guardar lo que ya no tiene respaldo.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** diseño, secciones 7 y 9.
- **Quién decidió:** dueño del producto (diseño acordado).
- **Fuente:** diseño, sección 9.

### D-044 · Reglas de los espacios privado y compartido

- **Fecha:** 9 al 11 oct 2026
- **Tema:** Seguridad
- **Estado:** vigente (se construye en la etapa 4)
- **El problema:** una persona trabaja con lo suyo y con lo del estudio; lo privado no debe filtrarse.
- **Qué se decidió:** el espacio compartido solo conoce lo compartido; lo que ve una persona es su espacio privado más lo que puede ver del compartido, armado en su propia PC; hay una base por espacio; un proyecto tiene una sola visibilidad (uno privado puede usar documentos compartidos, no al revés). Si un documento privado también está en el compartido, se usa una referencia y se compara localmente, sin revelar lo privado. "Publicar" es mover o copiar el archivo a una carpeta compartida, por cuenta del usuario.
- **Por qué:** no delatar lo privado y no procesar dos veces.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** diseño, sección 6.
- **Quién decidió:** dueño del producto (diseño acordado).
- **Fuente:** diseño, sección 6.

### D-045 · Autorización: el origen es el techo y el proyecto es el alcance

- **Fecha:** 11 oct 2026
- **Tema:** Seguridad
- **Estado:** vigente (reemplaza a D-037)
- **El problema:** cómo decidir qué ve cada persona del espacio compartido sin administrar dos veces los permisos que ya existen en las carpetas.
- **Qué se decidió:** la app nunca muestra más de lo que la carpeta de origen dejaría leer a esa persona (el *techo*); lo verifica con una *prueba de lectura*: le pide que demuestre que puede leer tramos de archivos al azar y, si coincide, le da un pase temporal. Lo que sale de las fuentes lo gobierna el techo; lo que existe solo en la app (wiki, síntesis) lo gobierna además la membresía del proyecto. Los proyectos nacen abiertos y se pueden reservar; hay un administrador del espacio; cada instancia se empareja una vez con un código. Límites aceptados: restringir en la app no protege los archivos; lo ya recibido no se revoca hacia atrás; protege contra exposición accidental, no contra quien altere su app.
- **Por qué:** sirve para cualquier origen y no duplica la fuente de verdad de los permisos.
- **Alternativas descartadas:** servicio al estilo UMA 2.0 (agrega un servidor que administrar); solo permisos del origen (no permite reservar en estudios con cuenta única); derivados escritos en el sistema de archivos compartido (poco práctico); solo permisos en la app (duplica la fuente de verdad y no protege archivos).
- **Dónde se aplica:** diseño, sección 6; P-048.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 03, sección 4 ("Cómo se llegó a la autorización"); P-048.

### D-046 · Despliegue: servicio compartido opcional, un solo escritor, IA por instancia

- **Fecha:** 9 al 11 oct 2026
- **Tema:** Distribución
- **Estado:** vigente (etapa 4)
- **El problema:** un estudio chico no tiene servidor; uno mediano sí.
- **Qué se decidió:** el servicio compartido puede correr en la PC de un abogado o en un servidor; hay un solo escritor por base compartida; cada instancia usa su propia cuenta de IA, con proveedores configurables; el rango de usuarios va de un abogado solo a un estudio mediano. Se parte de que la app ya queda en la bandeja del sistema, puede arrancar con Windows y levanta servicios locales.
- **Por qué:** no exigir infraestructura extra.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** diseño, sección 8.
- **Quién decidió:** dueño del producto (diseño acordado).
- **Fuente:** diseño, sección 8.

### D-047 · La generación del wiki sigue corriendo en la ventana de la app

- **Fecha:** 9 al 11 oct 2026
- **Tema:** Wiki
- **Estado:** vigente para el producto mínimo
- **El problema:** la generación del wiki con IA está escrita en la parte visual de la app (unas 5.000 líneas), no en su motor: necesita una sesión de Windows abierta y no puede correr como servicio.
- **Qué se decidió:** alcanza para el producto mínimo; convertirla en servicio queda en espera.
- **Por qué:** es mucho trabajo y no hace falta todavía.
- **Alternativas descartadas (postergadas):** generación como servicio (P-055).
- **Dónde se aplica:** diseño, secciones 8 y 10; P-055.
- **Quién decidió:** dueño del producto (diseño acordado).
- **Fuente:** sesión 03, sección 5; diseño, sección 8.

### D-048 · Cinco etapas, empezando por el espacio privado

- **Fecha:** 9 al 11 oct 2026
- **Tema:** Producto
- **Estado:** vigente
- **El problema:** el diseño completo es grande; el espacio compartido multiplica cada decisión.
- **Qué se decidió:** etapas: 1) base del espacio privado; 2) parecidos, familias y tipo; 3) volumen virtual; 4) espacio compartido; 5) plantillas y documentos nuevos. Desde la etapa 1 se dejan preparadas cuatro cosas para no rehacer: cada ubicación guarda origen, carpeta y etiquetas; toda consulta recibe "lo visible"; cada proyecto tiene miembros ("abierto" al inicio); existe el proyecto general.
- **Por qué:** probar la base antes de compartirla.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** diseño, sección 11.
- **Quién decidió:** dueño del producto (diseño acordado).
- **Fuente:** sesión 03, sección 4; diseño, sección 11.

### D-049 · Proyectos de prueba descartables; carpetas originales intocables

- **Fecha:** 9 al 11 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** los proyectos de prueba se armaron con el modelo viejo y había que decidir si migrarlos.
- **Qué se decidió:** los proyectos de prueba y la carpeta de trabajo se pueden descartar y rehacer; sus carpetas originales (en la unidad de Drive) no se alteran nunca. Las fuentes copiadas dentro de un proyecto se tratan como un origen más. El 12 oct se borraron los dos proyectos viejos a pedido del dueño (con copia de la configuración); las carpetas originales no se tocaron.
- **Por qué:** rehacer es más simple que migrar; los originales son datos reales.
- **Alternativas descartadas:** migrar los proyectos.
- **Dónde se aplica:** diseño, sección 9; P-051.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 03, sección 8; sesión 05, sección 12; P-051.

### D-050 · Reglas de colaboración con el dueño del producto

- **Fecha:** 9 al 11 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente
- **El problema:** en la sesión del cambio de rumbo aparecieron roces de método: análisis traídos de otras conversaciones tomados como definiciones, cambios al diseño sin visto bueno.
- **Qué se decidió:** los análisis que el dueño trae de otras conversaciones son hipótesis: se verifican contra el código y lo decidido, y se avisa si contradicen algo. Antes de cambiar el documento de diseño se muestra la propuesta y se espera visto bueno. "No toques el repo" significa solo análisis en el chat. En el documento de diseño no se nombran productos de terceros.
- **Por qué:** que el diseño refleje decisiones del dueño, no suposiciones.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** toda la colaboración.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 03, sección 8.

### D-051 · Tres reglas de autorización en espera

- **Fecha:** 9 al 11 oct 2026
- **Tema:** Seguridad
- **Estado:** en revisión (en espera hasta después del producto mínimo)
- **El problema:** casos borde del espacio compartido.
- **Qué se decidió:** a priori: al reservar un proyecto, ocultar también sus carpetas en las búsquedas de los demás, activado por defecto y con aviso de que no protege los archivos (P-052); si una carpeta está en un proyecto abierto y en uno reservado, gana el menos restrictivo y la app avisa (P-053); la prueba de lectura es siempre activa por ahora, pero tiene que poder desactivarse donde todos ven todo (P-054).
- **Por qué:** coherencia con D-045 sin construir todavía.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-052, P-053, P-054; diseño, sección 10.
- **Quién decidió:** dueño del producto ("propuesta aceptada a priori" en P-052; las otras dos sin indicación de quién).
- **Fuente:** diseño, sección 10; P-052 a P-054.

### D-052 · Producto mínimo: etapa 1 más el puente con el wiki

- **Fecha:** 11 oct 2026
- **Tema:** Producto
- **Estado:** vigente
- **El problema:** el diseño decía "etapa 1" y el dueño hablaba de "MVP" sin definirlo; sin acordarlo no se podía planificar.
- **Qué se decidió:** el producto mínimo es la etapa 1 (base del espacio privado: inventario, vigilancia, identidad, texto procesado una vez y pantallas) más un puente mínimo: el wiki del proyecto general toma sus fuentes de la base.
- **Por qué:** que se vea el valor de la base y que el wiki ya la use.
- **Alternativas descartadas:** solo la etapa 1.
- **Dónde se aplica:** P-049; PR #22 a #31.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 04, sección 3; P-049.

### D-053 · Producto mínimo primero; lo posterior queda "en espera"

- **Fecha:** 11 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente
- **El problema:** se tendía a definir en detalle cosas que todavía no se podían ver funcionando.
- **Qué se decidió:** no definir en detalle lo que va después del producto mínimo; se anota en `pendientes.json` con el estado nuevo `en-espera`, que no se empieza ni se diseña antes.
- **Por qué:** decidir con algo a la vista.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `pendientes.json` (PR #22); diseño, sección 1.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 03, sección 8; sesión 04, sección 3.

### D-054 · Cuatro pantallas de la base, con el estilo actual de la app

- **Fecha:** 11 oct 2026
- **Tema:** Interfaz
- **Estado:** vigente
- **El problema:** había que acordar qué se ve de la base, porque eso define cuánto construir.
- **Qué se decidió:** cuatro pantallas: Inicio (lo que requiere atención, salud del archivo, actividad reciente, búsqueda rápida), Orígenes, Documentos (inventario en tabla con buscador y filtros) y Detalle de un documento (con forma de artículo: texto, ficha, ubicaciones, páginas del wiki que lo citan). Se construyen con el estilo que la app ya tiene. La lista de "requiere atención" es una sola y en el producto mínimo la alimentan la base y la revisión existente (P-059).
- **Por qué:** lo mínimo para ver y controlar el archivo.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `src/components/docbase/`; PR #25 a #28; diseño, sección 13.
- **Quién decidió:** dueño del producto ("pantallas acordadas").
- **Fuente:** P-049; sesión 04, sección 4.

### D-055 · Marco visual y marca: para toda la app, en un turno dedicado

- **Fecha:** 11 oct 2026
- **Tema:** Interfaz
- **Estado:** vigente (sin empezar, P-057)
- **El problema:** el dueño quiere llevar la interfaz hacia sus bocetos de mayo (aspecto de enciclopedia en línea) y a la marca Micelya, pero hacerlo pantalla por pantalla dejaría la app despareja.
- **Qué se decidió:** un marco común para toda la app (barra superior con marca, buscador y usuario; franja que dice en qué estudio y caso se está y si es privado; navegación a la izquierda), páginas con forma de artículo de enciclopedia, y colores, tipografías y logo de Micelya. Se aplica a toda la app de una vez, en un turno dedicado y con una pasada de diseño previa. Los bocetos de mayo usan colores que no son de la marca: hay que rehacerlos.
- **Por qué:** coherencia visual; un formato que cualquiera sabe leer.
- **Alternativas descartadas:** aplicarlo de a una pantalla.
- **Dónde se aplica:** P-057; diseño, sección 13.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 04, secciones 3 y 8; P-057.

### D-056 · Pantallas nuevas en español; traducción completa aparte

- **Fecha:** 11 oct 2026
- **Tema:** Interfaz
- **Estado:** vigente
- **El problema:** la app original trae inglés, italiano, ruso y chino, no español; traducir todo antes de construir demoraría la base.
- **Qué se decidió:** las pantallas nuevas nacen en español; traducir toda la app al español, y las pantallas nuevas a los otros idiomas, es un trabajo aparte.
- **Por qué:** los usuarios son estudios argentinos; no frenar la base.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-058; diseño, sección 13.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 04, sección 3.

### D-057 · "Cliente" y "caso" son etiquetas, no proyectos

- **Fecha:** 11 oct 2026
- **Tema:** Etiquetas
- **Estado:** vigente
- **El problema:** había que decidir cómo se representa en la app a quién pertenece un documento y a qué asunto.
- **Qué se decidió:** "cliente" y "caso" son etiquetas (grupos de etiquetas, D-075), no proyectos. Cada profesión sugiere los suyos y el usuario los nombra como quiera. Cómo nace un caso (de una carpeta, a mano o extraído del contenido) queda para más adelante (P-061).
- **Por qué:** un documento puede pertenecer a varios agrupamientos; los proyectos son puntos de vista, no carpetas.
- **Alternativas descartadas:** un proyecto por cliente o por caso.
- **Dónde se aplica:** diseño, secciones 3 y 13; P-061.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 04, sección 3; sesión 05, sección 5.

### D-058 · No proponer contribuciones a upstream hasta nuevo aviso

- **Fecha:** 11 oct 2026
- **Tema:** Upstream
- **Estado:** reemplazada por D-151
- **El problema:** algunos arreglos propios (por ejemplo F-001 y F-003) también servirían al original; proponérselos ahorra mantenerlos, pero lleva trabajo y expone el proyecto.
- **Qué se decidió:** no se proponen contribuciones al original hasta nuevo aviso. P-008 queda en suspenso.
- **Por qué:** No registrado.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-008; columna "Upstream" de `FORK_CHANGES.md` (los "candidatos" no se proponen).
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 04, sección 3; P-008.

### D-059 · La base no le manda nada al modelo por su cuenta

- **Fecha:** 11 oct 2026
- **Tema:** IA
- **Estado:** vigente
- **El problema:** la base recorre todos los documentos del estudio; si usara IA por su cuenta, gastaría cupo y sacaría documentos de la máquina sin que nadie lo pida.
- **Qué se decidió:** la base obtiene el texto una vez por documento, solo por extracción directa (PDF, Office, texto). Ningún motor de lectura de imágenes corre por su cuenta: un escaneo queda marcado "Texto en imagen". Cuando el wiki lee un escaneo, ese texto vuelve a la base para no leerlo dos veces.
- **Por qué:** control del costo y de qué sale de la máquina.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #27 y #29; P-049.
- **Quién decidió:** agente al construir (no figura en las listas de decisiones sin consulta); coherente con D-042.
- **Fuente:** sesión 04, sección 4; P-049.

### D-060 · Asignar el proyecto general apaga su incorporación automática

- **Fecha:** 11 oct 2026
- **Tema:** Wiki
- **Estado:** reemplazada por D-071 (la idea de no mandar todo al modelo sin pedirlo se conserva: el proyecto nace con la incorporación automática apagada hasta activar el wiki)
- **El problema:** la app original incorpora automáticamente todo archivo nuevo de un proyecto. Al conectar las carpetas de la base al wiki, cientos de archivos se verían como nuevos e irían al modelo sin que nadie lo pida.
- **Qué se decidió:** asignar un proyecto como general desactiva su incorporación automática; la incorporación se pide carpeta por carpeta, con confirmación y aviso de uso del modelo.
- **Por qué:** evitar un gasto masivo no pedido.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #29.
- **Quién decidió:** agente sin consulta; cambiado por el dueño el 12 oct (P-066).
- **Fuente:** sesión 04, sección 6, punto 1; P-066.

### D-061 · El proyecto general se elige entre los existentes

- **Fecha:** 11 oct 2026
- **Tema:** Wiki
- **Estado:** reemplazada por D-069
- **El problema:** el diseño decía que el proyecto general se crea al incorporar carpetas, pero la app no tenía cómo crearlo sola.
- **Qué se decidió:** el usuario elige un proyecto de wiki ya abierto para que represente al proyecto general.
- **Por qué:** era lo más simple sobre la app existente.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #29.
- **Quién decidió:** agente sin consulta; cambiado por el dueño el 12 oct (P-066).
- **Fuente:** sesión 04, sección 6, punto 2; P-066.

### D-062 · La app abre en Inicio

- **Fecha:** 11 oct 2026
- **Tema:** Interfaz
- **Estado:** en revisión (sin confirmar, P-066)
- **El problema:** la app original abre en el wiki; con la base, el punto de partida natural es el estado del archivo.
- **Qué se decidió:** la app abre en la pantalla Inicio.
- **Por qué:** mostrar primero lo que requiere atención.
- **Alternativas descartadas:** abrir en el wiki.
- **Dónde se aplica:** PR #28.
- **Quién decidió:** agente sin consulta; el dueño la vio funcionando el 12 oct pero no consta que la haya confirmado.
- **Fuente:** sesión 04, sección 6, punto 3; sesión 05, sección 8, punto 1; P-066.

### D-063 · Quitar una carpeta de la base deja las páginas ya generadas

- **Fecha:** 11 oct 2026
- **Tema:** Wiki
- **Estado:** en revisión
- **El problema:** al quitar una carpeta de la base, qué hacer con las páginas del wiki que ya salieron de ella.
- **Qué se decidió:** se desmonta del wiki general, pero las páginas generadas quedan.
- **Por qué:** No registrado.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #29.
- **Quién decidió:** agente sin consulta. **Ambigüedad:** figura en P-066 como "a confirmar", pero el estado del 12 oct no la nombra ni entre las cambiadas ni entre las sin revisar.
- **Fuente:** sesión 04, sección 6, punto 4; P-066.

### D-064 · Vigilar solo por barrido periódico

- **Fecha:** 11 oct 2026
- **Tema:** Base documental
- **Estado:** en revisión (el aviso en vivo está pendiente, P-064)
- **El problema:** el diseño dice "aviso en vivo del sistema más barrido de respaldo"; el aviso en vivo agregaba trabajo.
- **Qué se decidió:** la base solo barre (recorre las carpetas): al arrancar, cada cinco minutos y a pedido. Un archivo nuevo puede tardar hasta cinco minutos en aparecer.
- **Por qué:** alcanzaba para el producto mínimo.
- **Alternativas descartadas (postergadas):** aviso en vivo (P-064).
- **Dónde se aplica:** PR #25 (`src-tauri/src/docbase/worker.rs`).
- **Quién decidió:** agente sin consulta (sesión 04 punto 5 y sesión 05 punto 10).
- **Fuente:** sesión 04, sección 6; sesión 05, sección 8; P-064.

### D-065 · "Qué cambió" lo sigue detectando el vigilante del wiki

- **Fecha:** 11 oct 2026
- **Tema:** Wiki
- **Estado:** en revisión (es parte de lo que falta de la etapa 3, P-012)
- **El problema:** la base sabe qué cambió, pero conectar eso al wiki era más trabajo que el puente mínimo.
- **Qué se decidió:** el puente cubre "qué fuentes hay" y "cuál es el texto de una"; "qué cambió" lo sigue detectando el vigilante propio del proyecto de wiki.
- **Por qué:** acotar el puente al mínimo.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #29; P-012.
- **Quién decidió:** agente sin consulta.
- **Fuente:** sesión 04, sección 6, punto 6.

### D-066 · Archivos que el inventario ignora

- **Fecha:** 11 oct 2026
- **Tema:** Base documental
- **Estado:** en revisión (sin revisar, P-066)
- **El problema:** las carpetas tienen archivos de sistema y temporales que no son documentos.
- **Qué se decidió:** el inventario deja afuera carpetas ocultas (las que empiezan con punto), `desktop.ini`, `Thumbs.db`, temporales de Office (`~$…`) y descargas a medio hacer.
- **Por qué:** no llenar el inventario de ruido.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #25.
- **Quién decidió:** agente sin consulta.
- **Fuente:** sesión 04, sección 6, punto 7; sesión 05, sección 8, punto 2; P-066.

### D-067 · Mensajes de error del motor en inglés, por ahora

- **Fecha:** 11 oct 2026
- **Tema:** Interfaz
- **Estado:** en revisión (se resuelve con la traducción, P-058)
- **El problema:** los errores que vienen del motor de montaje están en inglés y aparecen dentro de pantallas en español.
- **Qué se decidió:** dejarlos así hasta la traducción completa.
- **Por qué:** no adelantar el trabajo de traducción.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-058.
- **Quién decidió:** agente sin consulta.
- **Fuente:** sesión 04, sección 6, punto 8.

### D-068 · Dejar vacía una etiqueta corregida vuelve al valor de la carpeta

- **Fecha:** 11 oct 2026
- **Tema:** Etiquetas
- **Estado:** en revisión
- **El problema:** si el usuario corrige una etiqueta de un documento y la deja vacía, puede querer decir "sin etiqueta" o "volver a lo que dice la carpeta".
- **Qué se decidió:** vuelve al valor de la carpeta.
- **Por qué:** No registrado.
- **Alternativas descartadas:** dejar el documento sin esa etiqueta.
- **Dónde se aplica:** PR #26.
- **Quién decidió:** agente sin consulta. **Ambigüedad:** P-066 la lista a confirmar; el estado del 12 oct no la menciona, y el modelo de etiquetas cambió después (D-075), así que no está claro si sigue aplicando.
- **Fuente:** sesión 04, sección 6, punto 9; P-066.

### D-069 · La app crea sola el proyecto general y lo llama "Archivo"

- **Fecha:** 12 oct 2026
- **Tema:** Producto
- **Estado:** vigente (reemplaza a D-061)
- **El problema:** con D-061 el usuario tenía que elegir un proyecto para que hiciera de general; además la app exigía un proyecto abierto para mostrar cualquier pantalla, incluso las de la base.
- **Qué se decidió:** la app crea sola el proyecto general y abre ahí. Para un estudio jurídico se llama "Archivo" (el nombre sale del vocabulario, D-072).
- **Por qué:** es el archivo del estudio, no un proyecto más; el usuario no debería tener que armarlo.
- **Alternativas descartadas:** elegirlo entre los existentes (D-061).
- **Dónde se aplica:** PR #31; `src-tauri/src/docbase/bridge.rs`; diseño, secciones 3 y 9.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 05, secciones 4 y 5; P-066.

### D-070 · Nombres técnicos neutros; la carpeta de trabajo se pregunta una vez

- **Fecha:** 12 oct 2026
- **Tema:** Producto
- **Estado:** vigente
- **El problema:** el nombre visible ("Archivo") cambia según la profesión y el idioma; si se usara también como nombre de carpeta en disco, cambiaría con ellos. Además, la app nunca guardaba una "carpeta de trabajo": lo que el dueño recordaba haber elegido era la carpeta de su primer proyecto.
- **Qué se decidió:** los nombres técnicos (carpetas, identificadores) son neutros y no cambian con la profesión: el proyecto general vive en la carpeta `default`, dentro de la carpeta de trabajo de la app. La app pregunta una vez cuál es la carpeta de trabajo y la guarda.
- **Por qué:** separar lo que ve el usuario de lo que guarda la máquina.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #31; diseño, sección 13.
- **Quién decidió:** dueño del producto (nombres neutros); la pregunta de la carpeta de trabajo surgió al construir y no consta quién la propuso.
- **Fuente:** sesión 05, secciones 4, 5 y 6.

### D-071 · El wiki del Archivo se activa a mano y después se alimenta solo

- **Fecha:** 12 oct 2026
- **Tema:** Wiki
- **Estado:** vigente (reemplaza a D-060)
- **El problema:** con D-060 había que pedir la incorporación carpeta por carpeta. Pero dejar el wiki encendido desde el inicio mandaría todo al modelo sin aviso.
- **Qué se decidió:** el wiki no arranca solo: se activa desde Orígenes, que avisa cuántos documentos van a ir al modelo. Una vez activado se alimenta solo con lo que se agrega, cambia o se quita. Se puede pausar. Hasta que se activa, el proyecto `default` tiene apagada la incorporación automática.
- **Por qué:** consentimiento explícito para usar el modelo, sin trabajo manual después.
- **Alternativas descartadas:** incorporación carpeta por carpeta (D-060).
- **Dónde se aplica:** PR #31; pantalla Orígenes (panel "Wiki del archivo"); diseño, secciones 3 y 9.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 05, secciones 4, 5 y 6; P-066.

### D-072 · Términos del dominio en un vocabulario por profesión e idioma

- **Fecha:** 12 oct 2026
- **Tema:** Producto
- **Estado:** vigente
- **El problema:** palabras como "Archivo", "Cliente" o "Caso" son de abogados; una escribanía o un estudio contable usan otras. Si se escriben en el código, cada profesión obliga a reprogramar.
- **Qué se decidió:** los términos del dominio no van escritos en el código: salen de un vocabulario por vertical (profesión) y, dentro de ella, por idioma. Después de los estudios jurídicos vienen escribanías y estudios contables.
- **Por qué:** el producto apunta a varias profesiones.
- **Alternativas descartadas:** términos fijos en las pantallas.
- **Dónde se aplica:** `src/vocabulary/`; diseño, secciones 9 y 13; P-069.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 05, sección 5.

### D-073 · "Excluir" en vez de "borrar"

- **Fecha:** 12 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** el usuario necesita sacar de la base algunos archivos o carpetas, pero la app nunca toca los originales; "borrar" sugeriría lo contrario.
- **Qué se decidió:** la acción se llama "Excluir": el archivo o la carpeta siguen en su lugar y a la vista, pero la base no los procesa ni los entrega al wiki. No toca nada en disco y se puede deshacer.
- **Por qué:** coherencia con "los originales no se tocan" (D-002).
- **Alternativas descartadas:** "borrar".
- **Dónde se aplica:** PR #31; pantalla Orígenes; diseño, secciones 3 y 9.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 05, secciones 4 y 5.

### D-074 · Orígenes como árbol, con acciones por fila

- **Fecha:** 12 oct 2026
- **Tema:** Interfaz
- **Estado:** vigente
- **El problema:** la pantalla Orígenes mostraba solo cada carpeta incorporada; no se podía actuar sobre una subcarpeta o un archivo.
- **Qué se decidió:** cada origen se recorre como árbol de carpetas y archivos. En cada fila: abrir, incorporar al wiki (solo con el wiki activo), excluir o volver a incluir; en las carpetas, además, volver a barrer y definir reglas de etiquetas.
- **Por qué:** pedido por el dueño tras ver la primera versión en la app.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #31; diseño, sección 13.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 05, sección 4; P-049.

### D-075 · Grupos y etiquetas, reglas por carpeta, lo manual manda

- **Fecha:** 12 oct 2026
- **Tema:** Etiquetas
- **Estado:** vigente
- **El problema:** la estructura de carpetas indica a qué cliente o caso pertenece cada documento, pero rara vez es pareja; y siempre hay archivos mal guardados.
- **Qué se decidió:** un *grupo* es la pregunta ("Cliente") y una *etiqueta* la respuesta ("BGH"); también hay etiquetas sueltas. Se guardan aparte con un identificador, así renombrar una no toca ningún archivo. En cualquier carpeta se ponen reglas que heredan las subcarpetas y lo que aparezca después: "todo lo de acá lleva esta etiqueta" o "cada subcarpeta de acá es un…". Lo puesto a mano en un archivo manda sobre lo heredado.
- **Por qué:** reflejar el orden real del estudio, con correcciones a mano.
- **Alternativas descartadas:** indicar al incorporar cómo está ordenado todo el origen (queda como opción solo si la estructura es pareja).
- **Dónde se aplica:** PR #31; pantallas Orígenes y Documentos; diseño, sección 3.
- **Quién decidió:** dueño del producto (pedido suyo, inspirado en una herramienta que usa).
- **Fuente:** sesión 05, secciones 4 y 6; diseño, sección 3.

### D-076 · Las páginas son la verdad; la base lleva un índice reconstruible

- **Fecha:** 12 oct 2026
- **Tema:** Wiki
- **Estado:** en revisión (el dueño no lo objetó ni lo confirmó)
- **El problema:** los enlaces del wiki viven solo dentro del texto de cada página; para saber qué enlaza a qué hay que releer todas, con al menos seis reglas distintas. Renombrar una página rompe los enlaces que le apuntan. El dueño preguntó por qué no pasar los enlaces a la base con identificadores.
- **Qué se decidió:** las páginas siguen siendo la verdad (las escribe el modelo como texto y todo el wiki lee y escribe esos archivos). La base lleva un índice de páginas, enlaces y documento de origen, que se reconstruye entero a partir de ellas. Lo que se buscaba con identificadores (que renombrar no rompa enlaces) se resuelve en P-068.
- **Por qué:** no rehacer toda la capa de wiki heredada.
- **Alternativas descartadas:** enlaces guardados en la base con identificadores (reabrir si el dueño lo prefiere).
- **Dónde se aplica:** PR #31; `src-tauri/src/docbase/wiki.rs`; P-068; diseño, secciones 4 y 9.
- **Quién decidió:** agente recomendó; dueño sin confirmar.
- **Fuente:** sesión 05, secciones 4 y 9; P-068.

### D-077 · Si había otro proyecto abierto, la app lo reabre

- **Fecha:** 12 oct 2026
- **Tema:** Interfaz
- **Estado:** en revisión (sin revisar, P-066)
- **El problema:** la app crea y abre el Archivo, pero el usuario puede estar trabajando en otro proyecto.
- **Qué se decidió:** si el último proyecto abierto es distinto del Archivo, la app reabre ese.
- **Por qué:** No registrado.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #31.
- **Quién decidió:** agente sin consulta.
- **Fuente:** sesión 05, sección 8, punto 3.

### D-078 · "Volver a barrer" en una carpeta barre todo su origen

- **Fecha:** 12 oct 2026
- **Tema:** Base documental
- **Estado:** en revisión (sin revisar, P-066)
- **El problema:** el botón está en cada carpeta del árbol, pero el barrido funcionaba por origen completo.
- **Qué se decidió:** pedirlo en una carpeta barre todo el origen al que pertenece.
- **Por qué:** No registrado.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #31.
- **Quién decidió:** agente sin consulta.
- **Fuente:** sesión 05, sección 8, punto 4.

### D-079 · Excluir no quita del wiki lo ya incorporado

- **Fecha:** 12 oct 2026
- **Tema:** Wiki
- **Estado:** en revisión (sin revisar, P-066)
- **El problema:** al excluir un documento que ya generó páginas, qué pasa con esas páginas.
- **Qué se decidió:** quedan; la exclusión solo evita que se procese o entregue de nuevo.
- **Por qué:** No registrado.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #31.
- **Quién decidió:** agente sin consulta.
- **Fuente:** sesión 05, sección 8, punto 5.

### D-080 · Enlaces emparejados sin distinguir mayúsculas, espacios ni guiones

- **Fecha:** 12 oct 2026
- **Tema:** Wiki
- **Estado:** en revisión (sin revisar, P-066)
- **El problema:** el modelo escribe los enlaces a mano y no siempre con el nombre exacto de la página.
- **Qué se decidió:** el índice empareja un enlace con su página sin distinguir mayúsculas, espacios, guiones ni guiones bajos.
- **Por qué:** No registrado (se infiere: tolerar variaciones de escritura).
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #31; índice del wiki.
- **Quién decidió:** agente sin consulta.
- **Fuente:** sesión 05, sección 8, punto 6.

### D-081 · El color de cada etiqueta lo pone la app

- **Fecha:** 12 oct 2026
- **Tema:** Etiquetas
- **Estado:** en revisión (el dueño pidió poder elegirlo, anotado en P-057)
- **El problema:** cada etiqueta necesita un color para distinguirse.
- **Qué se decidió:** la app lo asigna sola.
- **Por qué:** No registrado.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #31; P-057.
- **Quién decidió:** agente sin consulta.
- **Fuente:** sesión 05, sección 8, punto 7; P-057.

### D-082 · El índice de enlaces todavía no lo usan grafo, búsqueda ni revisión

- **Fecha:** 12 oct 2026
- **Tema:** Wiki
- **Estado:** en revisión (P-068)
- **El problema:** pasar el grafo, la búsqueda, el panel de enlaces y la revisión automática del wiki al índice nuevo implica tocar código original sensible.
- **Qué se decidió:** construir el índice ahora y pasar cada función a usarlo después, de a una.
- **Por qué:** cuidado con código original.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-068.
- **Quién decidió:** agente sin consulta.
- **Fuente:** sesión 05, sección 8, punto 8.

### D-083 · Un solo vertical, fijo, por ahora

- **Fecha:** 12 oct 2026
- **Tema:** Producto
- **Estado:** en revisión (en espera, P-069)
- **El problema:** el vocabulario (D-072) admite varias profesiones, pero no hay dónde elegirla.
- **Qué se decidió:** por ahora hay una sola, estudios jurídicos, fija en el código. Falta definir dónde se elige, si un estudio puede cambiar términos y los vocabularios de otras profesiones.
- **Por qué:** alcanza para el estudio de referencia.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `src/vocabulary/`; P-069.
- **Quién decidió:** agente sin consulta.
- **Fuente:** sesión 05, sección 8, punto 9; P-069.

### D-084 · Probar el instalador en otra máquina antes de entregarlo

- **Fecha:** 12 oct 2026
- **Tema:** Distribución
- **Estado:** vigente (continúa en D-092)
- **El problema:** nunca se había generado un instalador; el repositorio es público y una versión publicada la puede bajar cualquiera.
- **Qué se decidió:** generar el instalador y probarlo en otra máquina del dueño antes de pasarlo a los abogados del estudio de referencia. Antes de entregarlo quedan tres decisiones: si la descarga puede ser pública, qué proveedor de IA usa cada uno (P-029) y si la primera entrega va con el wiki sin activar (lo que garantiza que ningún documento salga de la máquina).
- **Por qué:** no entregar algo no probado.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-006; `.github/workflows/build.yml`.
- **Quién decidió:** dueño del producto.
- **Fuente:** sesión 05, sección 10; P-006.

### D-085 · El Archivo nace con un idioma de wiki fijo

- **Fecha:** 13 oct 2026
- **Tema:** Idioma
- **Estado:** en revisión (a confirmar por el dueño, P-066)
- **El problema:** la app creó el Archivo sola, sin el diálogo de creación, así que su idioma de salida quedó en "automático". La detección automática tomó el español por francés y el wiki entero salió en francés (se frenó con 12 documentos incorporados y quedó mezclado).
- **Qué se decidió:** el Archivo nace con un idioma de wiki fijo: la preferencia general de idioma de salida de la app o, si no hay, el idioma del vocabulario.
- **Por qué:** que el primer wiki salga en el idioma del estudio sin depender de la detección.
- **Alternativas descartadas:** dejarlo en "automático".
- **Dónde se aplica:** PR #33; `src/lib/default-project.ts`; P-051.
- **Quién decidió:** agente sin consulta.
- **Fuente:** conversación del 13-14 oct; P-051; P-066.

### D-086 · Un idioma fijo por estudio, con el ajuste que la app ya tiene

- **Fecha:** 13 oct 2026
- **Tema:** Idioma
- **Estado:** vigente
- **El problema:** en modo "automático" cada página se escribe en el idioma del documento del que salió. Un estudio argentino tiene documentos en castellano, portugués, inglés e italiano: el wiki saldría mezclado aunque la detección acertara siempre.
- **Qué se decidió:** el wiki se escribe en un idioma fijo por estudio, usando el ajuste por proyecto que la app ya tiene (Configuración, *Output Preferences*). "Automático" sigue existiendo como opción. No hace falta cambiar el documento de diseño.
- **Por qué:** un wiki coherente; usar lo que la plataforma ya ofrece.
- **Alternativas descartadas:** la propuesta del agente de escribirlo en el documento de diseño como regla nueva; el idioma por documento.
- **Dónde se aplica:** P-070 (cerrado); `src/lib/output-language.ts`; PR #33.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 13-14 oct; P-070.

### D-087 · El wiki en varios idiomas se posterga mucho

- **Fecha:** 13 oct 2026
- **Tema:** Idioma
- **Estado:** en revisión (en espera, P-073)
- **El problema:** el dueño prevé que el estudio exponga funciones por la web y que cada persona elija en qué idioma ver las cosas.
- **Qué se decidió:** postergarlo para mucho más adelante. Entonces se definirá si se generan páginas en varios idiomas, si se traducen al pedirlas o si hay un idioma de base y los demás se derivan.
- **Por qué:** no hace falta para el producto mínimo.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-073.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 13-14 oct; P-073.

### D-088 · Detección de idioma sin IA pero confiable; idioma como dato de la base

- **Fecha:** 13 oct 2026
- **Tema:** Idioma
- **Estado:** vigente (el arreglo inmediato está hecho; lo de fondo es P-072)
- **El problema:** la detección de idioma heredada son reglas hechas a mano sobre letras y palabras cortas: tomaba el español por francés con solo ver "la" y una letra acentuada. Y la base no guarda en qué idioma está cada documento.
- **Qué se decidió:** la detección sigue sin usar IA, pero tiene que ser confiable. Arreglo inmediato: entre español, italiano, portugués y francés gana la lengua que junta más palabras y letras propias. De fondo: el idioma de cada documento pasa a ser un dato de la base (sirve para filtrar y, más adelante, para elegir el motor de lectura); evaluar un detector estadístico local que dé además un grado de confianza.
- **Por qué:** sin IA es más rápida y barata; con el idioma del wiki fijo (D-086), la detección ya no decide en qué idioma se escribe.
- **Alternativas descartadas:** detección con IA.
- **Dónde se aplica:** PR #33, F-014 (`src/lib/detect-language.ts`); P-072.
- **Quién decidió:** dueño del producto (pedido); arreglo hecho por el agente.
- **Fuente:** conversación del 13-14 oct; P-072; F-014.

### D-089 · "Pausar" el wiki también frena la cola

- **Fecha:** 13 oct 2026
- **Tema:** Wiki
- **Estado:** en revisión (a confirmar por el dueño, P-066)
- **El problema:** al pausar el wiki del Archivo solo dejaban de entrar documentos nuevos: la cola seguía procesando lo que ya tenía y gastando cupo del modelo. Así fue difícil frenar el wiki que salió en francés.
- **Qué se decidió:** "Pausar" detiene también la cola (el documento en curso vuelve a la cola); "Activar" la reanuda; al abrir la app, la cola del Archivo queda detenida mientras su wiki esté apagado.
- **Por qué:** que pausar signifique de verdad dejar de gastar.
- **Alternativas descartadas:** el comportamiento anterior.
- **Dónde se aplica:** PR #33; panel "Wiki del archivo" en Orígenes.
- **Quién decidió:** agente sin consulta.
- **Fuente:** conversación del 13-14 oct; P-066; P-051.

### D-090 · Caja de etiquetas: Enter no crea por accidente; sin nombres fijos de clientes

- **Fecha:** 13 oct 2026
- **Tema:** Etiquetas
- **Estado:** en revisión (las dos primeras partes, a confirmar por el dueño en P-066)
- **El problema:** en el uso real, Enter creaba etiquetas sin querer; una etiqueta renombrada se duplicaba al escribirla (se buscaba por el nombre viejo); y los textos de ejemplo de la caja tenían escrito el nombre de un cliente real.
- **Qué se decidió:** Enter toma la etiqueta escrita entera y solo crea una nueva si nada coincide; al escribir una etiqueta o un grupo se busca por el nombre que se ve, sin distinguir mayúsculas; los textos de ejemplo salen del vocabulario.
- **Por qué:** evitar duplicados y cumplir D-072 (nada de términos ni datos del dominio fijos en el código).
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #33; caja de etiquetas (detalle de documento, reglas de carpeta). Las mejoras de explicación de la caja siguen en P-057.
- **Quién decidió:** agente sin consulta.
- **Fuente:** conversación del 13-14 oct; P-066.

### D-091 · Una sola copia de la app corriendo a la vez

- **Fecha:** 13 oct 2026
- **Tema:** Distribución
- **Estado:** vigente
- **El problema:** cerrar la ventana no cierra la app, solo la esconde en la bandeja del sistema. Al volver a abrirla se arrancaba otra copia, que competía con la primera por los puertos locales; las copias se acumulaban en segundo plano.
- **Qué se decidió:** solo puede correr una copia; abrirla de nuevo trae al frente la ventana de la que ya corre.
- **Por qué:** evitar conflictos y consumo duplicado.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** PR #34, F-015.
- **Quién decidió:** el dueño del producto: señaló el problema y pidió corregirlo; el agente eligió cómo.
- **Fuente:** conversación del 13-14 oct; F-015.

### D-092 · Instaladores: corrida manual sin publicar; ninguna versión pública todavía

- **Fecha:** 13 oct 2026
- **Tema:** Distribución
- **Estado:** vigente
- **El problema:** para probar el instalador (D-084) hay que generarlo; publicar una versión con etiqueta la deja descargable para cualquiera, porque el repositorio es público.
- **Qué se decidió:** primero una corrida manual del proceso de compilación, que deja los instaladores como archivos de esa corrida sin publicar nada. Ninguna etiqueta de versión pública hasta que el dueño pruebe el instalador en otra máquina. Resultado: Windows y Linux compilaron bien; Mac falló al firmar por falta de certificado de Apple, lo esperado (D-017). La variable que habilita publicar ya está creada, por eso subir una etiqueta publicaría de inmediato.
- **Por qué:** no exponer una versión sin probar.
- **Alternativas descartadas (postergadas):** publicar `mi-desktop-v0.1.0-rc.1`.
- **Dónde se aplica:** P-006; `.github/workflows/build.yml`.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 13-14 oct; P-006.

### D-093 · Para rehacer un wiki se aparta la carpeta, nunca se borra

- **Fecha:** 13 oct 2026
- **Tema:** Wiki
- **Estado:** vigente (acción integrada pendiente, P-071)
- **El problema:** el wiki salió mal (en francés y mezclado) y la app no tiene forma de descartar lo generado y empezar de nuevo.
- **Qué se decidió:** se cierra la app y se aparta la carpeta del proyecto generado (renombrada o movida), nunca se borra; la app vuelve a crear el Archivo al arrancar. El texto leído de escaneos no se pierde porque está en la base. Queda pendiente ofrecer "rehacer el wiki" como acción de la app, con aviso del cupo que consume.
- **Por qué:** poder volver atrás si hiciera falta.
- **Alternativas descartadas:** borrar la carpeta.
- **Dónde se aplica:** P-071; P-051 (la carpeta mezclada quedó apartada fuera del repositorio).
- **Quién decidió:** el dueño autorizó borrar todo y reincorporar; el agente eligió apartar la carpeta en vez de borrarla, y el dueño no objetó.
- **Fuente:** conversación del 13-14 oct; P-051; P-071.

### D-094 · Diseñar el recorrido antes de construir; ante la duda, no se une

- **Fecha:** 13 oct 2026
- **Tema:** Parecidos
- **Estado:** vigente
- **El problema:** hoy la base solo reconoce como "el mismo documento" lo idéntico. Alcanza una palabra distinta (un pie agregado, una errata, dos lecturas del mismo escaneo) para que no los reconozca; y a la vez dos contratos del mismo modelo para clientes distintos comparten casi todo el texto y son documentos distintos.
- **Qué se decidió:** definir el recorrido completo por escrito antes de construir. Criterio de partida: el porcentaje en común no alcanza, hay que mirar qué cambió; si difieren cifras, fechas, nombres o identificadores son documentos distintos de una misma plantilla; si difiere texto de relleno, es el mismo documento. Unir dos cosas distintas (mezclar clientes) es mucho peor que dejar un duplicado sin detectar: ante la duda, no se une.
- **Por qué:** en un estudio es muy común que dos contratos compartan casi todo y sean distintos.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-015; `docs/design/parecidos-propuesta.md` (secciones 1 y 6), en revisión como propuesta.
- **Quién decidió:** dueño del producto (pedido y criterio "acordado"). **Ambigüedad:** la frase "ante la duda, no se une" está en la propuesta, sección 1; no figura entre las definiciones del dueño de la sección 3.
- **Fuente:** P-015; `parecidos-propuesta.md`, sección 1.

### D-095 · Los parecidos los une siempre una persona

- **Fecha:** 13 oct 2026
- **Tema:** Parecidos
- **Estado:** vigente
- **El problema:** la app podría unir sola documentos que calcula como casi iguales, pero un error mezcla clientes.
- **Qué se decidió:** los parecidos nunca se unen automáticamente; siempre decide una persona, al menos hasta tener experiencia práctica. La decisión se guarda y no se vuelve a preguntar; una relación descartada no se vuelve a proponer.
- **Por qué:** el riesgo legal de una unión equivocada.
- **Alternativas descartadas:** unión automática por encima de un umbral.
- **Dónde se aplica:** `parecidos-propuesta.md`, secciones 3, 5 y 10; P-015.
- **Quién decidió:** dueño del producto.
- **Fuente:** `parecidos-propuesta.md`, sección 3; conversación del 13-14 oct.

### D-096 · Se muestran en el detalle del documento, con botones para unir

- **Fecha:** 13 oct 2026
- **Tema:** Parecidos
- **Estado:** vigente
- **El problema:** hay que decidir dónde ve la persona los parecidos y cómo decide.
- **Qué se decidió:** en el detalle de un documento, como hoy se muestran sus copias, aparecen sus documentos parecidos con una descripción de en qué consiste el parecido (por ejemplo, "mismo texto de base; cambian el nombre de la parte y tres importes") y botones para unirlos eligiendo la relación.
- **Por qué:** reutilizar un lugar que el usuario ya conoce.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** pantalla Detalle de un documento; `parecidos-propuesta.md`, secciones 3 y 9.
- **Quién decidió:** dueño del producto.
- **Fuente:** `parecidos-propuesta.md`, sección 3.

### D-097 · Relaciones ampliables y con nombres configurables

- **Fecha:** 13 oct 2026
- **Tema:** Parecidos
- **Estado:** vigente
- **El problema:** la lista de tipos de relación entre documentos va a cambiar a medida que se vean archivos de distintos estudios, y cómo se nombra cada relación depende de la profesión, el idioma y el país.
- **Qué se decidió:** la lista de relaciones es un dato de configuración: agregar una no obliga a rehacer nada. Los nombres no van escritos en el programa: salen del vocabulario por vertical, idioma y país. Punto de partida aceptado: mismo documento, prácticamente el mismo, ejemplar firmado, versión, contenido en, misma plantilla con otros datos, parecidos. Conviene validar los nombres con un estudio (por ejemplo, "plantilla" o "modelo").
- **Por qué:** no reprogramar por cada estudio o país; coherencia con D-072.
- **Alternativas descartadas:** lista fija en el código.
- **Dónde se aplica:** `parecidos-propuesta.md`, secciones 3 y 4; `src/vocabulary/`.
- **Quién decidió:** dueño del producto.
- **Fuente:** `parecidos-propuesta.md`, secciones 3 y 4; conversación del 13-14 oct.

### D-098 · Al unir como el mismo documento, queda una sola fuente del wiki

- **Fecha:** 13 oct 2026
- **Tema:** Parecidos
- **Estado:** vigente (reemplaza en parte a D-005)
- **El problema:** si dos archivos son en realidad el mismo documento (por ejemplo, el contrato y su ejemplar firmado), mandar los dos al wiki repite información.
- **Qué se decidió:** en principio todos los documentos van al wiki. Pero cuando una persona une dos como el mismo documento (relaciones "prácticamente el mismo", "ejemplar firmado" y "versión"), queda uno solo como fuente del wiki, elegido por la persona; la app, a lo sumo, sugiere cuál. **No** vale para "misma plantilla, otros datos" ni para "contenido en", que son documentos distintos y siguen los dos en el wiki. Las páginas del wiki que salieron de ellos podrían unirse también; eso no está diseñado.
- **Por qué:** evitar repeticiones sin perder documentos distintos.
- **Alternativas descartadas:** que todos los miembros de una familia sigan yendo al wiki (D-005).
- **Dónde se aplica:** `parecidos-propuesta.md`, secciones 3, 4, 12 y 13 (qué pasa con lo ya incorporado sigue abierto).
- **Quién decidió:** dueño del producto.
- **Fuente:** `parecidos-propuesta.md`, secciones 3 y 4; conversación del 13-14 oct.

### D-099 · Reducir el uso de IA, no eliminarlo

- **Fecha:** 13 oct 2026
- **Tema:** IA
- **Estado:** reemplazada en parte por D-150. Sigue valiendo: ahorrar cupo nunca es razón para elegir un camino peor. Deja de valer: "reducir" el uso de la IA y "primero medir el cálculo y después sumar usos del modelo" como regla general
- **El problema:** "determinista primero" (D-042) podía leerse como "evitar la IA a toda costa", incluso cuando aporta.
- **Qué se decidió:** el uso de IA se reduce, no se elimina. Se usa donde agrega valor y de la forma más conveniente. Ahorrar cupo nunca es razón para elegir un camino peor; el modelo solo se posterga si usarlo ahora retrasa medir. Primero se mide hasta dónde llega el cálculo y después se suman usos del modelo; el primer candidato es describir qué cambió entre dos documentos, a pedido y de a un par.
- **Por qué:** la calidad es lo prioritario; medir primero muestra dónde aporta la IA.
- **Alternativas descartadas:** eliminar la IA del proceso; elegir por ahorro de cupo.
- **Dónde se aplica:** `parecidos-propuesta.md`, secciones 3, 7 y 13.
- **Quién decidió:** dueño del producto.
- **Fuente:** `parecidos-propuesta.md`, sección 3; conversación del 13-14 oct.

### D-100 · Procesar en pasadas sobre todo el archivo

- **Fecha:** 13 oct 2026
- **Tema:** Base documental
- **Estado:** en revisión (el diseño de la iteración se sigue investigando)
- **El problema:** hoy cada documento se procesa por separado, una vez. Pero lo más valioso a veces solo se ve mirando todo el archivo: que un acuerdo, sus recibos y su ejemplar firmado nombran a la misma persona; que falta una factura en una serie; que el nombre del propio estudio aparece en todo y no relaciona nada.
- **Qué se decidió:** el procesamiento pasa a ser iterativo sobre todo el archivo, en pasadas: por documento, por par o familia, y sobre todo el archivo; lo de una pasada enriquece y corrige a las otras. Cada pasada necesita objetivos específicos y hay que asegurar que el proceso termine: pocas vueltas, corte cuando una vuelta ya casi no cambia nada, y cada vuelta solo agrega propuestas, nunca une ni borra por su cuenta.
- **Por qué:** capturar lo que solo se ve en el conjunto, sin un proceso que no termine.
- **Alternativas descartadas:** solo procesamiento lineal por documento.
- **Dónde se aplica:** `parecidos-propuesta.md`, sección 8. Las pasadas 1 y 2 son la etapa de parecidos; la 3 va después, pero ya condiciona qué se guarda por documento.
- **Quién decidió:** dueño del producto (dirección); el diseño detallado está en investigación.
- **Fuente:** `parecidos-propuesta.md`, sección 8; conversación del 13-14 oct.

### D-101 · El conjunto de prueba lo arma el asistente y lo corrige el dueño

- **Fecha:** 13 oct 2026
- **Tema:** Parecidos
- **Estado:** vigente
- **El problema:** para ajustar los umbrales sin adivinar hace falta una "hoja de respuestas": entre 50 y 100 documentos con la relación correcta escrita a mano para cada par.
- **Qué se decidió:** la arma el asistente a partir de las mediciones y la revisa y corrige el dueño, que es quien conoce los documentos.
- **Por qué:** combinar el trabajo pesado del asistente con el conocimiento del dueño.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `parecidos-propuesta.md`, secciones 11 y 13 (decisión abierta 1, ahora resuelta).
- **Quién decidió:** dueño del producto (aceptó la propuesta del agente).
- **Fuente:** conversación del 13-14 oct; `parecidos-propuesta.md`, sección 13.

### D-102 · Esfuerzo estimado solo en términos relativos, nunca en tiempo

- **Fecha:** 13–14 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente
- **El problema:** las estimaciones en horas o días que da el asistente se basan en cómo trabaja una persona y siempre fallan para un agente de IA.
- **Qué se decidió:** nunca estimar en horas ni en ninguna unidad de tiempo; solo esfuerzo relativo entre tareas ("esta cuesta más que aquella").
- **Por qué:** las estimaciones de tiempo vienen del trabajo humano y no aplican.
- **Alternativas descartadas:** estimaciones en tiempo.
- **Dónde se aplica:** toda planificación y propuesta.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 13-14 oct.

### D-103 · Corregir los formularios tomados por el mismo documento

- **Fecha:** 13 oct 2026
- **Tema:** Base documental
- **Estado:** vigente (arreglado en el PR #38)
- **El problema:** dos fichas de cliente de personas distintas figuran en la base como copias una de la otra. Son el mismo formulario PDF completado para dos personas: las respuestas están en los campos del formulario y no salen al extraer el texto, así que el texto de las dos es idéntico y la regla "mismo texto, mismo documento" (D-020) las une. Eso es mezclar dos clientes. Confirmado con el motor de la app.
- **Qué se decidió:** corregirlo antes de construir los parecidos. Se agrega al texto extraído lo completado en los campos del formulario, lo que además hace buscable lo completado y lo hace llegar al wiki.
- **Por qué:** es un defecto de lo ya construido con riesgo legal directo.
- **Alternativas descartadas:** no unir por texto cuando el PDF tiene campos: evitaba el error pero dejaba sin leer lo completado.
- **Dónde se aplica:** P-074; `src-tauri/src/docbase/text.rs`; `parecidos-propuesta.md`, secciones 2 y 14.
- **Quién decidió:** el dueño del producto ("es un error y hay que corregirlo"); el agente eligió el arreglo.
- **Fuente:** P-074; `parecidos-propuesta.md`, sección 2, punto 6; conversación del 13-14 oct.

### D-104 · Un formulario PDF entrega su plantilla y sus datos

- **Fecha:** 14 oct 2026
- **Tema:** Base documental
- **Estado:** vigente (primer paso hecho en el PR #38)
- **El problema:** reconocer en un conjunto de documentos qué parte es plantilla y qué parte son datos es una de las tareas más difíciles de la etapa de parecidos. Pero un PDF con formulario ya viene separado así: la página impresa es la plantilla y los campos completados son los datos.
- **Qué se decidió:** aprovecharlo desde ya. Además de sumar lo completado al texto (D-103), la base guarda los campos de cada formulario aparte, uno por uno (nombre del campo y valor), como el derivado "campos" que prevé el diseño. La regla que use esto para proponer "misma plantilla, otros datos" llega con la etapa de parecidos.
- **Por qué:** da casi gratis, y con certeza, lo que en otros documentos hay que inferir comparando textos.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `src-tauri/src/docbase/text.rs` (derivado `fields`); PR #38; propuesta de parecidos.
- **Quién decidió:** el dueño del producto lo sugirió; el agente lo implementó.
- **Fuente:** conversación del 14 oct; PR #38.

### D-105 · Guardar el texto una vez por espacio no expone nada nuevo

- **Fecha:** 14 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** Cuando el texto leído de los escaneos se guardaba por proyecto, la razón dada era no mezclar textos de distintos clientes (D-025). Desde que la base guarda el texto una vez para todo el espacio, quedó la duda de cómo se atiende esa preocupación.
- **Qué se decidió:** Guardar una sola vez el texto de un documento que aparece en varias ubicaciones no cambia quién puede verlo: el mismo texto ya estaba en esas ubicaciones, al alcance de quien podía abrirlas. La preocupación de mezclar clientes no es de almacenamiento sino de uso: qué información recibe junta el modelo al responder o al escribir el wiki. Eso lo gobierna el alcance de cada proyecto y el conjunto de lo visible para cada persona (diseño, sección 6).
- **Por qué:** Razonamiento del dueño del producto, confirmado por el agente.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `docs/design/base-documental.md` (secciones 6 y 9).
- **Quién decidió:** El dueño del producto.
- **Fuente:** Conversación del 14 oct.

### D-106 · Las relaciones entre documentos también se infieren, no solo se miden

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** vigente
- **El problema:** La lista de relaciones de la propuesta solo tenía lo que se puede medir comparando palabras.
- **Qué se decidió:** Se suman relaciones que se infieren: dice lo mismo con otras palabras, traducción de, respuesta a, constancia de firma de, mismo asunto, misma persona con otro asunto. Algunas necesitan el modelo.
- **Por qué:** Lo que un abogado necesita relacionar no se limita a textos parecidos.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `docs/design/parecidos-propuesta.md`, sección 4.
- **Quién decidió:** El dueño del producto.
- **Fuente:** Conversación del 14 oct.

### D-107 · "Ejemplar firmado" solo si la firma se puede verificar; vista previa como relación propia

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** vigente
- **El problema:** En la carpeta de prueba, la carta documento escaneada que parecía el ejemplar enviado de un borrador era en realidad una vista previa generada por el sistema del correo, con la marca de agua "VISTA PREVIA - SIN VALOR LEGAL".
- **Qué se decidió:** Un documento solo se trata como ejemplar firmado si la firma se puede verificar (en una imagen, típicamente con el modelo). Se distingue en qué etapa está un documento: borrador hecho con un procesador de texto, vista previa generada por el sistema que lo va a despachar, o ya procesado (enviado, recibido, firmado). "Vista previa" pasa a ser una relación.
- **Por qué:** Tratar una vista previa sin valor legal como un ejemplar firmado es un error con consecuencias legales.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `docs/design/parecidos-propuesta.md`, secciones 3, 4 y 5 bis.
- **Quién decidió:** El dueño del producto.
- **Fuente:** Conversación del 14 oct; hoja de respuestas.

### D-108 · Un catálogo de tipos de documento que crece de a poco

- **Fecha:** 14 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** Muchas relaciones no se pueden decidir sin saber qué tipo de documento es cada uno y en qué etapa está.
- **Qué se decidió:** La base tendrá un catálogo de tipos de documento: para cada tipo, qué es, sus etapas y cómo se reconocen, sus marcas verificables sin modelo, los datos que interesan, los documentos que suelen acompañarlo y cómo se mide. Arranca con los tipos de la carpeta de prueba y se amplía con el uso; los tipos nuevos los confirma una persona.
- **Por qué:** Cada estudio, vertical y país tiene sus tipos; describirlos mejora el reconocimiento con el tiempo.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-075; `docs/design/parecidos-propuesta.md`, sección 5 bis.
- **Quién decidió:** El dueño del producto lo propuso.
- **Fuente:** Conversación del 14 oct.

### D-109 · Las firmas electrónicas se reconocen con reglas

- **Fecha:** 14 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** Varios documentos de la carpeta de prueba se firmaron con servicios de firma electrónica, y saber si un documento está firmado y por quién es clave para relacionarlo con su versión sin firmar.
- **Qué se decidió:** Las firmas digitales incrustadas en un PDF se reconocen con reglas, porque siguen un estándar. Con una salvedad verificada: los servicios de firma sellan cada versión que emiten, así que un PDF trae una firma incrustada aunque ninguna parte lo haya firmado. Las reglas se arman por servicio, separando el sello del servicio de la firma de cada parte.
- **Por qué:** Es verificable, no requiere modelo y no inventa.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** P-076.
- **Quién decidió:** El dueño del producto lo propuso; el agente verificó la salvedad.
- **Fuente:** Conversación del 14 oct.

### D-110 · Los nombres de archivo son hipótesis; manda el contenido

- **Fecha:** 14 oct 2026
- **Tema:** Base documental
- **Estado:** vigente
- **El problema:** En la carpeta de prueba, el nombre de las facturas indicaba el mes de consumo, no la fecha de emisión ni la de vencimiento; tomarlo como dato hacía ver un hueco en la serie que quizás no existe.
- **Qué se decidió:** Los nombres de archivo y de carpeta sirven para plantear hipótesis. Cuando el contenido dice otra cosa, manda el contenido.
- **Por qué:** Los nombres los pone una persona y no son confiables.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `docs/design/reglas-de-procesamiento.md` (R-04).
- **Quién decidió:** El dueño del producto.
- **Fuente:** Conversación del 14 oct.

### D-111 · El tipo de documento necesita el modelo y va temprano; cálculo y modelo se alternan

- **Fecha:** 14 oct 2026
- **Tema:** IA
- **Estado:** vigente
- **El problema:** La propuesta recomendaba medir primero solo con cálculo y sumar usos del modelo después.
- **Qué se decidió:** Determinar el tipo de documento requiere el modelo, y saberlo temprano permite medir según el tipo. Las pasadas de cálculo y de modelo se alternan, cada una con un propósito bien definido.
- **Por qué:** Medir sin conocer el tipo es menos efectivo que medir por tipo.
- **Alternativas descartadas:** Medir primero solo con cálculo: descartada por el dueño.
- **Dónde se aplica:** `docs/design/parecidos-propuesta.md`, secciones 7 y 8; `docs/design/reglas-de-procesamiento.md` (R-19).
- **Quién decidió:** El dueño del producto.
- **Fuente:** Conversación del 14 oct.

### D-112 · Compartir una persona no es compartir un asunto

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** vigente
- **El problema:** En la carpeta de prueba, un préstamo y un contrato de consultoría nombran a la misma persona, pero son asuntos sin relación.
- **Qué se decidió:** Que dos documentos nombren a la misma persona o empresa no los hace del mismo asunto. Para "mismo asunto" hace falta algo más específico; si no, la relación es "misma persona, otro asunto".
- **Por qué:** Evita vincular asuntos distintos por una persona compartida.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `docs/design/reglas-de-procesamiento.md` (R-05).
- **Quién decidió:** El dueño del producto.
- **Fuente:** Conversación del 14 oct; hoja de respuestas.

### D-113 · No se hacen inferencias sobre inferencias

- **Fecha:** 14 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente
- **El problema:** En un proceso por pasadas, una conclusión puede usarse como base de la siguiente, y los errores se acumulan.
- **Qué se decidió:** Toda conclusión se ancla en las fuentes. Encadenar inferencias es excepcional, y lo que resulte se verifica contra las fuentes.
- **Por qué:** Experiencia del dueño: encadenar inferencias acumula el error de forma creciente.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `docs/design/reglas-de-procesamiento.md` (R-01 a R-03).
- **Quién decidió:** El dueño del producto.
- **Fuente:** Conversación del 14 oct.

### D-114 · Las reglas del procesamiento se documentan aparte y se revisan

- **Fecha:** 14 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente
- **El problema:** Las reglas que hacen que el análisis por pasadas funcione estaban dentro de la propuesta de parecidos.
- **Qué se decidió:** Pasan a un documento propio, revisable por una persona en cualquier momento y obligatorio para los agentes. Son un punto de partida y se revisan con las pruebas.
- **Por qué:** Valen para todo análisis de la base, y hay que poder evaluarlas y cambiarlas.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `docs/design/reglas-de-procesamiento.md`; `CLAUDE.md`.
- **Quién decidió:** El dueño del producto.
- **Fuente:** Conversación del 14 oct.

### D-115 · El proceso tiene que decir qué falta

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** vigente
- **El problema:** Relacionar lo que hay no alcanza: al estudio le importa también lo que no está.
- **Qué se decidió:** El proceso informa lo que falta: un hueco en una serie, el ejemplar firmado de un contrato que no aparece, la carta enviada de una vista previa.
- **Por qué:** Es una de las ayudas más valiosas para un abogado y solo se ve mirando el archivo entero.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `docs/design/reglas-de-procesamiento.md` (R-23); propuesta de parecidos, pasada 9.
- **Quién decidió:** El dueño del producto.
- **Fuente:** Conversación del 14 oct.

### D-116 · Qué cuenta como texto de rutina

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** la pasada 0 de la propuesta dice que lo que aparece en "una fracción alta" de los documentos (membrete, datos del propio estudio, pie de firma) no cuenta para medir parecidos, pero no dice cuánto es "alta".
- **Qué se decidió:** una secuencia de cinco palabras es texto de rutina cuando aparece en al menos la mitad de los documentos con texto y en al menos diez de ellos. Al llegar un documento se mide su vecindario con el texto de rutina de ese momento; los pares ya medidos no se vuelven a medir solo porque la rutina cambió.
- **Por qué:** con un umbral bajo, las trece facturas de un proveedor harían de su propia plantilla "texto de rutina" y dejarían de parecerse. El mínimo de diez evita que en un archivo chico todo sea rutina. No volver a medir todo cuando cambia la rutina respeta R-15 (solo lo que cambió).
- **Alternativas descartadas:** un 20 % (hacía rutina a la plantilla de las facturas); volver a medir todos los pares cuando cambia la rutina (contra R-15).
- **Dónde se aplica:** `src-tauri/src/docbase/similar/store.rs` (`ROUTINE_SHARE`, `ROUTINE_MIN_DOCS`). En la carpeta de prueba (52 documentos con texto) ninguna secuencia quedó como rutina.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S1 (`desatendido/spec-S1-motor.md`), 14 oct; PR #42.

### D-117 · Cómo se mide cuánto comparten dos documentos

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** las reglas de la sección 6 de la propuesta dicen "comparten 80 %" o "97 %", pero medido sobre secuencias de cinco palabras una sola palabra cambiada rompe cinco secuencias: dos facturas con solo cifras distintas, o un borrador con pocas correcciones, parecen mucho menos parecidos de lo que son.
- **Qué se decidió:** las secuencias de cinco palabras (firma, casilleros, parecido y contención) sirven para encontrar candidatos y quedan como medidas de referencia. Las reglas usan el texto alineado palabra por palabra: "comparten" es qué parte de las palabras propias (sin el texto de rutina) del documento más corto aparece, en orden, en el otro. Los candidatos se buscan con un umbral bajo a propósito (64 bandas de 2 números: un par que comparte el 20 % de sus secuencias aparece como candidato 9 de cada 10 veces) para no perder un anexo dentro de un contrato; se miden a lo sumo 100 vecinos por documento, y un par con menos de 25 % de contención en secuencias no se alinea.
- **Por qué:** es la lectura más fiel de "qué parte del texto tienen en común", y no confunde un dato cambiado con un texto distinto.
- **Alternativas descartadas:** aplicar los umbrales de la propuesta sobre las secuencias de cinco palabras (las facturas quedaban debajo del 80 %).
- **Dónde se aplica:** `src-tauri/src/docbase/similar/propose.rs`, `minhash.rs`, `store.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S1, 14 oct.

### D-118 · Cuándo es "misma plantilla" y cuándo "versión"

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** la propuesta separa "misma plantilla, otros datos" (documentos distintos) de "versión" (el mismo documento) según si lo que cambia es "sobre todo datos" o "sobre todo redacción". Medido sobre la carpeta de prueba, eso solo no alcanzaba: un recibo de la última cuota agrega una cláusula larga, y la redacción agregada tapaba que cambiaban el importe y las fechas; al revés, al completar el bloque de firma de un acuerdo se agregan nombres y fechas que no reemplazan nada.
- **Qué se decidió:** se propone "misma plantilla" cuando, compartiendo mucho, (a) un identificador o un importe se reemplaza por otro ("dos contratos idénticos salvo el monto son dos contratos", sección 6), o (b) cambia un campo de formulario, o (c) hay una sustitución repetida, o (d) en lo reemplazado (tramos donde los dos documentos tienen algo) los datos son más de la mitad. Los bloques que tiene uno solo (una cláusula agregada, un bloque de firma completado) no cuentan como datos reemplazados. "Ejemplar firmado" exige además que el firmado contenga al otro casi entero: le puede faltar a lo sumo el 2 % de sus palabras.
- **Por qué:** ante la duda entre "versión" (uniría las fuentes del wiki) y "misma plantilla" (deja los dos), conviene equivocarse hacia la segunda (R-07). Con estas reglas la carpeta de prueba no tiene ninguna propuesta equivocada.
- **Alternativas descartadas:** la proporción de datos sobre todo lo que cambia (proponía "versión" para la cuota con la cláusula agregada); exigir el 95 % para el ejemplar firmado (un borrador con pocas correcciones pasaba por ejemplar firmado de la versión final).
- **Dónde se aplica:** `src-tauri/src/docbase/similar/propose.rs`, `changes.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S1, 14 oct; evaluación en `desatendido/evaluacion-S1.txt` (fuera del repositorio).

### D-119 · Cómo se reconoce la firma de una parte

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** los PDF que salen de un servicio de firma electrónica traen una firma incrustada aunque nadie haya firmado, porque el servicio sella cada copia (P-076). Hay que distinguir ese sello de la firma de una parte, que es lo único que habilita "ejemplar firmado" (D-107).
- **Qué se decidió:** de cada firma incrustada se leen los nombres de sus certificados; si alguno nombra un servicio del catálogo (los nombres de certificado de cada servicio son datos del catálogo), es el sello del servicio. Es firma de una parte la firma incrustada que no es de un servicio, o la que registra el historial del servicio con frases del catálogo ("ha firmado electrónicamente el documento", "firmó con el nombre de", "DocuSigned by"…). La página de historial o de constancia que agrega el servicio no cuenta como contenido al comparar, y tampoco la línea con el identificador del sobre que el servicio repite en cada página.
- **Por qué:** se verifica sin modelo y sin nombrar a nadie (R-24); un servicio nuevo es una entrada más del catálogo.
- **Alternativas descartadas:** contar firmas (todas las copias de un servicio tienen una); validar criptográficamente la cadena de certificados (no hace falta para distinguir sello de parte, y sumaría dependencias).
- **Dónde se aplica:** `src-tauri/src/docbase/similar/marks.rs`; `src-tauri/resources/catalog/document-types.json`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S1, 14 oct; lectura de las páginas de historial de la carpeta de prueba.

### D-120 · El catálogo de tipos es un archivo de datos

- **Fecha:** 14 oct 2026
- **Tema:** Base documental
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** el catálogo de tipos (D-108) y la lista de relaciones (D-097) tienen que poder ampliarse sin cambiar el programa.
- **Qué se decidió:** un archivo JSON legible, `src-tauri/resources/catalog/document-types.json`, con las relaciones (identificador neutro, si hacen de los dos un solo documento, si tienen dirección, si se miden) y, por tipo, sus etapas, sus variantes (quién emite el formulario o presta el servicio) y sus señales: frases, formas de identificadores (`AA999999999AR`), si es formulario, firmas incrustadas, páginas y líneas que no cuentan al comparar. Va dentro de la app; si en la carpeta de datos de la app hay un `docbase/document-types.json` con la misma forma, se usa ese al iniciar. Lo calculado con otro catálogo se vuelve a calcular solo.
- **Por qué:** agregar un tipo, una variante o una relación es editar un archivo (R-25, R-26); el archivo de reemplazo permite probar sin compilar.
- **Alternativas descartadas:** TOML (el resto de la base usa JSON); guardarlo en la base de datos (todavía no hay pantalla para editarlo).
- **Dónde se aplica:** `src-tauri/src/docbase/similar/types.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S1, 14 oct.

### D-121 · El documento principal de una familia

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** R-14 dice que una familia se arma contra un documento principal, "el más completo, o el ejemplar firmado", y que una familia que crece o se dispersa demasiado se marca.
- **Qué se decidió:** el principal es el que tiene la firma verificable de una parte; entre iguales, el de más palabras; entre iguales, el más antiguo en la base. Entran a la familia solo los documentos con una relación de familia (prácticamente el mismo, ejemplar firmado, vista previa, versión, misma plantilla) con el principal; lo que quedó afuera forma su propia familia de la misma manera. Se marca "large" una familia de más de 30 documentos, y "dispersed" la que no pudo tomar todo lo que estaba encadenado a ella. Lo descartado por una persona no cuenta; lo unido, con la relación que eligió.
- **Por qué:** es la lectura directa de R-14 y evita el encadenamiento.
- **Alternativas descartadas:** el documento con más vínculos como principal (en una cadena A~B~C el del medio se lleva a los dos extremos, que no se parecen).
- **Dónde se aplica:** `src-tauri/src/docbase/similar/store.rs` (`families`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S1, 14 oct.

### D-122 · Alineación propia, sin dependencias nuevas

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** comparar dos textos palabra por palabra necesita un algoritmo de diferencias, y reconocer identificadores por su forma suele hacerse con expresiones regulares.
- **Qué se decidió:** se escribió una alineación chica en la app: anclas en las palabras que aparecen una sola vez en cada texto (la secuencia más larga en el mismo orden) y, entre anclas, la tabla clásica de la subsecuencia común más larga; un tramo demasiado grande para la tabla se toma como cambiado entero. Las formas de los identificadores se describen con una notación propia y simple, sin un paquete de expresiones regulares.
- **Por qué:** lo pedía la especificación; evita dependencias, y los textos legales tienen muchas palabras únicas, así que es rápido y fiel.
- **Alternativas descartadas:** los paquetes `similar` o `diff`, y `regex`.
- **Dónde se aplica:** `src-tauri/src/docbase/similar/align.rs`, `types.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S1, 14 oct.

### D-123 · Solo se comparan documentos con todo su texto

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** un PDF con algunas páginas escaneadas tiene parte de su texto extraído y el resto esperando reconocimiento.
- **Qué se decidió:** se comparan solo los documentos que tienen todo su texto (estado `processed`). Cuando llega el texto reconocido, el documento se firma y se compara solo. El texto que vino de un reconocimiento tiene 5 puntos más de tolerancia en cada umbral (nunca en "prácticamente el mismo") y certeza baja.
- **Por qué:** comparar medio documento daría relaciones equivocadas; y la propuesta pide umbrales más tolerantes para lo reconocido.
- **Alternativas descartadas:** comparar el texto parcial.
- **Dónde se aplica:** `src-tauri/src/docbase/similar/store.rs`, `propose.rs`. En la carpeta de prueba quedaron afuera 14 documentos que esperan reconocimiento, entre ellos los de 4 pares de la hoja de respuestas.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S1, 14 oct.

### D-124 · Las pruebas no usan nombres de la carpeta de prueba

- **Fecha:** 14 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** la comprobación automática de R-24 encontró que las pruebas ya existentes de la base documental usaban nombres de la carpeta de prueba (dos empresas y un apellido) como datos de ejemplo.
- **Qué se decidió:** se reemplazaron por nombres inventados, sin cambiar lo que prueban.
- **Por qué:** R-24 vale también para las pruebas, y la comprobación tiene que pasar.
- **Alternativas descartadas:** excluir las pruebas viejas de la comprobación.
- **Dónde se aplica:** `src-tauri/src/docbase/*.rs`, `src/components/docbase/labels.tsx`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S1, 14 oct.

### D-125 · Los nombres de las relaciones salen del vocabulario

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** el motor identifica cada relación con un nombre técnico neutro (`signed-copy`, `version`…) y la propuesta pide que los nombres que lee la persona sean configurables por vertical y por país. Además, en las relaciones con dirección hay que decir cuál de los dos es qué: "el ejemplar firmado" es uno solo de los dos documentos.
- **Qué se decidió:** cada vocabulario (por vertical y por idioma) tiene la lista `relations`: para cada relación, su nombre ("Ejemplar firmado") y, si tiene dirección, la frase que va entre los dos documentos ("es el ejemplar firmado de"). La pantalla arma "«Contrato firmado.pdf» es el ejemplar firmado de este documento". Una relación que el catálogo agregue y el vocabulario todavía no nombre se muestra por su identificador, sin romper nada. Una prueba exige que cada relación del catálogo tenga nombre en cada idioma, y frase si tiene dirección. Los nombres en español son los de la propuesta (sección 4); los de inglés son una traducción del agente.
- **Por qué:** el catálogo crece sin tocar el programa (decisión del dueño del 13 de octubre) y los nombres no van escritos en las pantallas. La frase de dirección evita que la persona tenga que adivinar cuál es el firmado o la vista previa.
- **Alternativas descartadas:** poner los nombres en el catálogo del motor (mezcla datos técnicos con palabras de cada profesión); una sola etiqueta sin dirección ("Ejemplar firmado" a secas).
- **Dónde se aplica:** `src/vocabulary/index.ts`, `src/vocabulary/legal.ts`, `src/components/docbase/similar.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S2 (pantallas de parecidos), 14 oct.

### D-126 · El vocabulario admite términos por país

- **Fecha:** 14 oct 2026
- **Tema:** Base documental
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** la propuesta pide que los nombres de las relaciones puedan cambiar entre países, y el vocabulario solo distinguía vertical e idioma.
- **Qué se decidió:** cada vertical puede tener `countries`, por etiqueta de idioma y país ("es-MX"), con solo los términos que cambian; se aplican encima del vocabulario de ese idioma. Una sección del país se mezcla clave por clave con la del idioma. Un país nunca se aplica sobre otro idioma: si la vertical no tiene ese idioma, se usa su idioma por defecto sin los términos del país. `setVertical` acepta el país; hoy nadie lo elige y ningún país tiene términos propios (P-079).
- **Por qué:** deja preparada la variante sin cambiar lo que ya funcionaba: sin país, todo sigue igual.
- **Alternativas descartadas:** un vocabulario completo por país (repite todo); esperar a que exista el ajuste para elegir el país.
- **Dónde se aplica:** `src/vocabulary/index.ts`; pruebas en `src/vocabulary/vocabulary.test.ts` con una vertical inventada.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S2 (pantallas de parecidos), 14 oct.

### D-127 · Qué espera una decisión y cómo lo muestra Inicio

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** Inicio tiene que avisar de los parecidos que esperan una decisión, "una línea por familia" según la propuesta. Pero puede haber decenas de familias, y hay pares que no forman familia (por ejemplo, "contenido en").
- **Qué se decidió:** espera una decisión todo par propuesto con una relación sugerida; los "parecidos, sin relación clara" no se cuentan, igual que en el motor de S1 (`docbase_similar_pending`). Inicio muestra una línea por familia con algo pendiente, las más grandes primero y hasta cinco ("4 documentos parecidos a «X» esperan una decisión"), y una línea más con los pares restantes. Cada línea de familia lleva a Documentos filtrado por esa familia; la del resto, a Documentos con el filtro "con parecidos pendientes", que usa la misma definición. Las líneas van con el mismo color que las propuestas del wiki que esperan revisión.
- **Por qué:** que la cuenta de Inicio y lo que muestra el filtro digan lo mismo, y que Inicio no se llene de líneas.
- **Alternativas descartadas:** una línea por cada familia sin límite; una sola línea con el total; contar también los "parecidos" sin relación.
- **Dónde se aplica:** `src/components/docbase/similar.ts` (`buildSimilarAttention`), `home-view.tsx`, `documents-view.tsx`, `src-tauri/src/docbase/query.rs` (filtros `similarPending` y `similarGroup`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S2 (pantallas de parecidos), 14 oct.

### D-128 · Las relaciones que se pueden elegir

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** el botón "elegir otra relación" necesita una lista. El catálogo trae quince relaciones, entre ellas "mismo documento" (que la base aplica sola) y "parecidos" (que significa que no se encontró relación).
- **Qué se decidió:** la lista es la del catálogo, sin "mismo documento" ni "parecidos", en dos grupos: las que se miden comparando textos y las que se infieren. Las inferidas se ofrecen aunque el motor todavía no las proponga, porque una persona sí puede saberlas (por ejemplo, que una carta responde a otra). Un parecido sin relación sugerida no tiene botón "unir con esta relación": solo "elegir una relación" y "descartar".
- **Por qué:** unir con "parecidos" no dice nada, y "mismo documento" ya lo resuelve la base sin preguntar.
- **Alternativas descartadas:** ofrecer solo las medidas; ofrecer todas.
- **Dónde se aplica:** `src/components/docbase/similar.ts` (`relationChoices`), `similar-documents.tsx`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S2 (pantallas de parecidos), 14 oct.

### D-129 · La fuente del wiki que sugiere la app

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066); reemplazada en parte por D-143 (el texto del recuadro: el wiki ya aplica la elección; el criterio de sugerencia sigue valiendo)
- **El problema:** al unir dos documentos con una relación de "mismo documento" (prácticamente el mismo, ejemplar firmado, versión) la persona elige cuál queda como fuente del wiki, y la app sugiere uno. El motor sugiere en algunos casos y en otros no (por ejemplo, en "versión" no sugiere).
- **Qué se decidió:** se sugiere la del motor cuando la hay; si no, el que tiene la firma de una parte, después el que tiene más palabras, y si empatan el contenido más viejo. Al unir aparece un recuadro con los dos documentos para elegir, con el sugerido marcado. Como el wiki todavía no usa esa elección (P-078), el recuadro lo dice: "La elección queda guardada; por ahora el wiki todavía no la aplica".
- **Por qué:** sigue el mismo criterio que el principal de una familia (D-121) y no promete algo que todavía no pasa.
- **Alternativas descartadas:** unir sin preguntar usando la sugerencia; no sugerir nada.
- **Dónde se aplica:** `src/components/docbase/similar.ts` (`suggestedWikiSource`), `similar-documents.tsx`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S2 (pantallas de parecidos), 14 oct.

### D-130 · Ver las diferencias lado a lado queda para después

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** la propuesta pide ver los dos textos lado a lado con lo que cambia resaltado. El motor alinea los textos pero no guarda la alineación, así que hace falta un comando nuevo y una vista propia.
- **Qué se decidió:** en esta etapa cada parecido muestra un botón "Ver diferencias" deshabilitado, con una nota que dice que todavía no está disponible. Queda como P-077.
- **Por qué:** la etapa se concentró en decidir (unir, elegir, descartar, deshacer); el botón visible deja claro que la función viene.
- **Alternativas descartadas:** construirla ahora; no mostrar nada.
- **Dónde se aplica:** `src/components/docbase/similar-documents.tsx`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S2 (pantallas de parecidos), 14 oct.

### D-131 · Cómo se ve cada parecido en el detalle de un documento

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** la sección "Documentos parecidos" tenía que decir qué documento es, qué relación se propone, en qué consiste el parecido, cuánto comparten "en lenguaje simple" y qué se puede hacer.
- **Qué se decidió:** una tarjeta por parecido, con el estilo de las ubicaciones: el otro documento con enlace a su detalle y su carpeta; la relación propuesta (o la elegida, si ya se unieron) y la frase de dirección; la descripción del motor; las medidas en palabras: "Comparten el 92 % del texto" (palabras en común sobre las de los dos), "El 97 % del más corto está, palabra por palabra, en el otro" solo cuando agrega algo (5 puntos o más sobre la anterior), las palabras de cada uno y un aviso si parte del texto se leyó de una imagen. Los porcentajes se cortan, no se redondean, para que nunca diga 100 % si no es igual. Arriba a la derecha, el estado (espera una decisión, unidos o descartado, con su fecha). Unir con una relación de documentos distintos es un solo clic; con una de "mismo documento" pide antes la fuente. Las decisiones se pueden deshacer; los descartados quedan al final, atenuados. "Familia" se usa como palabra de la interfaz, no del vocabulario de la profesión.
- **Por qué:** mismo marco visual que el resto del detalle (la marca llega con P-057), y todo lo que la persona necesita para decidir sin abrir otra pantalla.
- **Alternativas descartadas:** una tabla; mostrar las medidas técnicas (Jaccard, contención) tal cual.
- **Dónde se aplica:** `src/components/docbase/similar-documents.tsx`, `similar.ts`, `document-detail.tsx`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S2 (pantallas de parecidos), 14 oct.

### D-132 · Un servicio de firma o un formulario no es el tipo de un documento

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos (catálogo de tipos)
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** el catálogo de S1 tenía como tipos "PDF emitido por un servicio de firma electrónica" y "Formulario PDF completable". Pero la hoja de respuestas, con razón, dice que esos documentos son un acuerdo, un recibo o una ficha de cliente: el servicio de firma o el formulario dicen cómo se emitió el documento, no qué es.
- **Qué se decidió:** esas dos entradas quedan marcadas en el catálogo como `carrier` ("portador"). Sus marcas nunca dan el tipo (el tipo lo pregunta el modelo) y sus datos y reglas valen para el documento que las lleve (por ejemplo, el identificador del proceso de firma). El modelo no las ve en la lista de tipos.
- **Por qué:** sin esto, todo acuerdo firmado por un servicio quedaba con un "tipo" que no le sirve a nadie, y los acuerdos con y sin servicio de firma quedaban separados.
- **Alternativas descartadas:** borrar esas entradas (se pierden sus señales, que S1 usa); dejarlas como tipos.
- **Dónde se aplica:** `src-tauri/resources/catalog/document-types.json` (`carrier`), `similar/types.rs`, `similar/classify.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S3 (tipos con el modelo), 14 oct.

### D-133 · Los tipos que se agregaron al catálogo

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos (catálogo de tipos)
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** el modelo elige de una lista cerrada (R-22), y la lista de S1 solo tenía cartas documento, acuses y constancias de firma.
- **Qué se decidió:** se agregaron los tipos que la propuesta (sección 5 bis) nombra para empezar: factura, recibo, acuerdo (contratos, convenios, acuerdos y memorandos), boleta de pago y ficha de cliente, cada uno con una descripción general, sus etapas (acuerdo y recibo: borrador, sin firmar, firmado por una parte, firmado por todas) y sus datos. Las descripciones no nombran ningún caso (R-24).
- **Por qué:** son los tipos que la propuesta ya preveía; sin ellos el modelo tenía que inventar nombres para la mitad de los documentos.
- **Alternativas descartadas:** dejar que el modelo proponga esos tipos como nuevos y que una persona los confirme (sería pedir a la persona lo que la propuesta ya definió).
- **Dónde se aplica:** `src-tauri/resources/catalog/document-types.json`; sus nombres visibles en `src/vocabulary/legal.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S3, 14 oct.

### D-134 · Qué pregunta se le hace al modelo por cada familia

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos (uso del modelo)
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** R-21 pide una pregunta por familia, sobre su documento principal. Pero en una familia los miembros pueden estar en otra etapa que el principal (el ejemplar sin firmar y el firmado, la vista previa y la carta enviada).
- **Qué se decidió:** la pregunta lleva el texto del principal (un extracto: las primeras 1.200 palabras y las últimas 300, porque la página de historial de firmas suele estar al final), el nombre del archivo como pista (R-04), lo que ya se verificó con marcas, la lista cerrada de tipos con sus etapas y, para cada miembro (hasta 30), su nombre, la frase del motor que dice en qué se parece al principal y sus marcas. El modelo responde el tipo (vale para todos), la etapa del principal y, en "otros", la etapa de cada miembro. Si las marcas propias de un documento dicen la etapa, mandan las marcas sobre el modelo. Un miembro sin etapa dicha toma la del principal.
- **Por qué:** sigue siendo una sola pregunta por familia, y la etapa de cada miembro se decide con lo que el modelo puede ver.
- **Alternativas descartadas:** una pregunta más por cada miembro de otra etapa; dejar sin etapa a los miembros.
- **Dónde se aplica:** `src-tauri/src/docbase/similar/classify.rs` (`type_prompt`, `rows_from_answer`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S3, 14 oct.

### D-135 · Cuándo las marcas alcanzan y no se pregunta

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos (uso del modelo)
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** la pasada 4 no tiene que preguntar si las marcas verificables ya dicen el tipo y la etapa.
- **Qué se decidió:** alcanzan cuando las marcas del principal son de un solo tipo que no es un portador y dan una sola etapa (o el tipo no tiene etapas), y cada miembro de la familia o tiene marcas propias con su etapa o está en la misma etapa por su relación (prácticamente el mismo, misma plantilla, versión). Marcas de dos tipos o de dos etapas se contradicen: pregunta el modelo.
- **Por qué:** una marca verificada vale más que el modelo, pero una marca que se contradice no decide nada. Ejemplo general: el formulario de un correo trae impreso el recuadro del acuse aunque esté vacío, así que una carta tiene marcas de carta y de acuse.
- **Alternativas descartadas:** preguntar siempre; decidir por la primera marca.
- **Dónde se aplica:** `similar/classify.rs` (`type_by_marks`, `rows_by_marks`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S3, 14 oct.

### D-136 · La misma pregunta no se repite: cómo se reconoce

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos (uso del modelo)
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** R-20 pide que la misma pregunta no se haga dos veces, y que la respuesta se guarde con lo preguntado y la versión del modelo.
- **Qué se decidió:** cada respuesta se guarda en `model_answer` con la huella (SHA-256) de la versión de la pregunta y su texto entero, el texto de la pregunta, el motor y el modelo que respondió, y la fecha. Como el texto de la pregunta incluye el extracto del documento y la lista de tipos, cambiar el texto, la familia o el catálogo es otra pregunta; recalcular todo, borrar los tipos o volver a abrir la app no lo es. Las respuestas no se borran cuando se va un documento: si el mismo texto vuelve, no cuesta nada. Solo se guarda una respuesta que es JSON.
- **Por qué:** la huella del texto completo de la pregunta es exactamente "la misma pregunta"; no hace falta decidir a mano qué partes cuentan.
- **Alternativas descartadas:** guardar por documento (se perdería al cambiar el documento y se repetiría por cada copia).
- **Dónde se aplica:** `similar/classify.rs` (`question_key`, `ask`), esquema 7.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S3, 14 oct.

### D-137 · Respuesta del modelo en JSON estricto, con el camino de Codex que ya existía

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos (uso del modelo)
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** hacía falta preguntarle al modelo desde Rust, con el proveedor intercambiable, y que la respuesta tuviera siempre la misma forma.
- **Qué se decidió:** un contrato nuevo, `DocumentAsker` (en `source_volume/asking.rs`, al lado del de reconocimiento de texto), con una implementación de Codex CLI que reutiliza la misma ejecución de `codex exec` (carpeta vacía, solo lectura, sin aprobaciones) y le pasa la forma de la respuesta con `--output-schema`. La pantalla elige el motor con el modelo configurado; hoy solo responde Codex CLI. La respuesta tiene tipo, etapa, si el tipo es nuevo, su nombre y descripción, confianza (0 a 1), una razón breve y la etapa de los otros miembros. Un tipo nuevo se guarda como propuesto con el nombre que dio el modelo; uno de la lista dicho como nuevo es el de la lista; una etapa que el tipo no tiene no se guarda.
- **Por qué:** lo pide la especificación y evita respuestas que no se pueden leer.
- **Alternativas descartadas:** pedir JSON solo en el texto de la pregunta; llamar a otros proveedores desde Rust (queda para cuando haga falta).
- **Dónde se aplica:** `src-tauri/src/source_volume/asking.rs`, `recognition_codex.rs`, `src/lib/document-asking.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S3, 14 oct.

### D-138 · Cuándo se le pregunta al modelo

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos (uso del modelo)
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** las preguntas consumen el cupo del proveedor y no pueden correr a espaldas del usuario.
- **Qué se decidió:** dos caminos. (1) En Orígenes, un panel "Tipos de documento" con cuántos documentos tienen tipo, cuántas preguntas faltan y el botón "Reconocer tipos", que antes avisa cuántas preguntas va a hacer y que consumen cupo; una vuelta hace como mucho 60 (R-16) y el resto queda para otra. (2) Mientras el wiki del archivo está activo, un hilo aparte hace una pregunta cada 30 segundos con el modelo configurado; si falla (por ejemplo, por límite de uso) espera 10 minutos. Pausar el wiki lo detiene. El aviso de "Activar" del wiki lo dice. Lo que no cuesta nada (marcas, respuestas ya guardadas, reglas de datos, relaciones por tipo) corre solo con el resto de las pasadas.
- **Por qué:** es el mismo criterio que el wiki del archivo: un acto explícito, o el wiki encendido.
- **Alternativas descartadas:** preguntar siempre en segundo plano; preguntar todo de una vez sin límite.
- **Dónde se aplica:** `src/components/docbase/document-types-view.tsx`, `general-project-panel.tsx`, `src-tauri/src/docbase/worker.rs` (`spawn_asking`), `commands.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S3, 14 oct.

### D-139 · Cómo se buscan los datos de cada tipo

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos (pasada 5)
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** los datos que interesan de cada tipo tienen que estar en el catálogo como datos, con un patrón general, y el modelo solo debe usarse donde las reglas no encuentran lo que buscan.
- **Qué se decidió:** cada dato del catálogo dice qué clase de valor es (identificador, importe, fecha, número o el resto de la línea) y cómo se encuentra: pocas palabras después de una frase, por la forma de un identificador o por el valor de una marca ya verificada. Una lista después de una frase se lee entera ("facturas A, B y C"); un total es el mayor importe junto a la palabra. Solo los datos marcados como `required` que no aparecen se le preguntan al modelo, en una pregunta por documento con la misma caché.
- **Por qué:** agregar un dato o un tipo es una entrada del catálogo (R-25, R-26), y el modelo se usa poco y donde agrega.
- **Alternativas descartadas:** expresiones regulares en el catálogo (difíciles de leer y corregir para una persona); preguntarle todos los datos al modelo.
- **Dónde se aplica:** `similar/data.rs`, `document-types.json` (`data`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S3, 14 oct.

### D-140 · Cómo corrigen las relaciones las reglas del tipo

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos (pasada 6)
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** con el tipo, la etapa y los datos, algunas relaciones medidas cambian; pero lo que la regla general ya resolvía no se toca.
- **Qué se decidió:** tres clases de regla, escritas en el catálogo. "Datos distintos": dos documentos del tipo con un dato distinto (número de factura, número de envío, proceso de firma) son documentos distintos; la relación medida baja a la de la regla y nunca sube (orden: mismo documento, prácticamente el mismo, ejemplar firmado, versión, vista previa, contenido en, misma plantilla, parecidos). Para facturas y constancias baja a "misma plantilla"; para cartas documento, a "parecidos", porque el catálogo dice que el formulario del correo no cuenta como parecido. "Par de etapas": una vista previa y el borrador, la carta enviada o la recibida con las mismas facturas pasan a "vista previa", solo si estaban medidas como prácticamente el mismo, versión, contenido en (la vista previa contiene el texto de la carta y le suma el formulario) o misma plantilla. "Dato compartido": un acuse de recibo y la carta con su número de envío, o una constancia de firma y el documento de su mismo proceso de firma, se proponen aunque sus textos no se parezcan; solo reemplazan "parecidos" o "misma plantilla". Lo medido se guarda y vuelve si la regla deja de valer; las propuestas que dependen del tipo dicho por el modelo quedan con certeza baja; una decisión de una persona nunca se toca.
- **Por qué:** son las reglas que da el catálogo, y siempre empujan hacia el lado seguro (R-07) o agregan una relación que el texto no podía ver.
- **Alternativas descartadas:** que la pasada 6 decida sobre todos los pares.
- **Dónde se aplica:** `similar/by_type.rs`, `document-types.json` (`rules`), columnas nuevas de `similar_pair`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S3, 14 oct.

### D-141 · El texto de las imágenes en la prueba real

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos (medición)
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** en la prueba de S1, 14 documentos de la carpeta de prueba no tenían texto (son imagen) y las cartas documento casi todas eran de esos. Sin su texto no se puede medir su tipo.
- **Qué se decidió:** la herramienta de prueba copia (nunca mueve ni modifica) el texto ya reconocido que había en las carpetas `raw/recognized` del proyecto del wiki y del wiki descartado, y reconoce con Codex lo que faltaba (2 páginas). Ese texto queda en una carpeta temporal, fuera del repo y de la carpeta de prueba.
- **Por qué:** mide lo que la app va a ver cuando el wiki ya leyó esas imágenes, sin volver a gastar cupo en lo ya leído.
- **Alternativas descartadas:** medir sin las imágenes; reconocerlas todas de nuevo.
- **Dónde se aplica:** `similar/check_types.rs` (solo pruebas).
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S3, 14 oct.

### D-142 · Las etapas se describen en el catálogo, y eso va en la pregunta

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos (catálogo de tipos, uso del modelo)
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** en la primera prueba real el modelo acertó el tipo de casi todos los documentos, pero solo 7 de 18 etapas. La pregunta llevaba los nombres de las etapas sin decir cómo se reconoce cada una: el modelo tomaba el recuadro impreso del aviso de recibo como prueba de entrega, y los miembros de una familia que no se nombraban heredaban la etapa del principal (el ejemplar sin firmar quedaba "firmado").
- **Qué se decidió:** cada tipo del catálogo puede describir sus etapas (`stageDescriptions`), y la pregunta las incluye. La carta documento y el acuse aclaran que el recuadro del aviso de recibo vacío no prueba nada. La pregunta pide la etapa de cada miembro de la familia, y un miembro sin etapa dicha solo hereda la del principal si su relación es de la misma etapa (prácticamente el mismo, misma plantilla, versión). La versión de la pregunta pasó a `tipo-2`.
- **Por qué:** es conocimiento general del catálogo (R-26), no una regla para un caso; y un miembro sin etapa es mejor que uno con una etapa equivocada.
- **Alternativas descartadas:** reglas por marcas para decidir "sin firmar" (un servicio de firma no siempre deja una marca de firma de parte, D-140).
- **Dónde se aplica:** `document-types.json`, `similar/classify.rs`, `similar/types.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** prueba real de S3 contra la hoja de respuestas, 14 oct.

### D-143 · El documento no elegido deja de alimentar el wiki

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** al unir dos parecidos con una relación de "mismo documento" (prácticamente el mismo, ejemplar firmado, versión) la persona elige cuál queda como fuente del wiki (D-129), pero el wiki seguía incorporando los dos.
- **Qué se decidió:** el documento no elegido sale de las fuentes del wiki del proyecto general, en todas sus ubicaciones (sus copias son el mismo documento). Se aplica en el puente con el wiki con el mismo camino que las exclusiones: no aparece como pendiente, no se encola al activar el wiki ni desde Orígenes, y el vigilante del proyecto no lo incorpora aunque cambie, porque toda incorporación pasa por la misma pregunta a la base. Lo que ya se había incorporado de él no se borra ni se mueve (igual que al excluir, D-079): las páginas que salieron de él siguen en el wiki. Deshacer la unión, o elegir al otro, lo vuelve a ofrecer. Unir con una relación de documentos distintos (misma plantilla, contenido en, etc.) deja los dos. El texto del recuadro de D-129 cambió: ahora dice "Solo el elegido alimenta el wiki; lo que el otro ya aportó se queda".
- **Por qué:** es lo que pide la propuesta (sección 3) y reutiliza un camino ya probado, así que ninguna entrada al wiki queda sin cubrir.
- **Alternativas descartadas:** borrar o unir las páginas del wiki que salieron del no elegido (la propuesta dice que podrían unirse: queda para más adelante); filtrar solo en la pantalla del proyecto general.
- **Dónde se aplica:** `src-tauri/src/docbase/mod.rs` (`passed_over_sql`), `bridge.rs` (`wiki_sources`, `excluded_sources`), `src/components/docbase/similar-documents.tsx`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S4 (el wiki usa la fuente elegida, P-078), 14 oct.

### D-144 · La cola del wiki sigue cada decisión sobre parecidos

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** una unión puede llegar con el documento no elegido ya esperando en la cola, y con la fuente elegida sin incorporar. Al deshacer, el que había quedado afuera vuelve a ser fuente.
- **Qué se decidió:** después de cada decisión (unir, descartar o deshacer), si el proyecto abierto es el general: se sacan de la cola las tareas que esperan (no la que está en curso) de fuentes que la base ya no ofrece al wiki; y, si el wiki está activo, se encolan las fuentes de los dos documentos que nunca se incorporaron, que es la elegida al unir y, al deshacer, también la otra. Con el wiki en pausa no se encola nada; queda como pendiente para cuando se active. Si esa parte falla, la decisión igual queda guardada y la pantalla lo avisa.
- **Por qué:** con el wiki activo el wiki se alimenta solo (D-071): lo que pasa a ser fuente entra como entraría un archivo nuevo, y lo que deja de serlo no debería gastar modelo. No se corta la tarea en curso para no perder trabajo a medias.
- **Alternativas descartadas:** dejar que la fuente elegida espere a que el usuario la encole a mano; cancelar también la tarea en curso.
- **Dónde se aplica:** `src/lib/docbase-wiki-bridge.ts` (`followSimilarDecision`, `sourcesOfDocuments`), `src/components/docbase/similar-documents.tsx`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S4 (el wiki usa la fuente elegida, P-078), 14 oct.

### D-145 · Dónde se avisa que un documento no alimenta el wiki, y qué pasa con las cadenas

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066); reemplazada en parte por D-148 (un círculo de elecciones ya no se permite; el aviso y la regla de "algún par" siguen valiendo)
- **El problema:** la persona tiene que poder ver, en el detalle del documento no elegido, por qué ya no va al wiki. Además, un documento puede estar unido con más de uno.
- **Qué se decidió:** en la sección "En el wiki" del detalle, arriba de las páginas que salieron de él, un recuadro: "Este documento no alimenta el wiki: se unió con «X», que es la fuente elegida", con una línea que aclara que lo ya incorporado se queda y que deshacer lo vuelve a ofrecer. Si se unió con varios y en todos se eligió al otro, los nombra a todos. Un documento queda afuera si en algún par unido como el mismo documento se eligió al otro; en una cadena (A con B eligiendo B, B con C eligiendo C) quedan afuera A y B. Si una persona arma un círculo de elecciones podrían quedar afuera todos; no se previene en esta etapa.
- **Por qué:** la sección del wiki es donde se mira qué aportó un documento al wiki; la regla de "algún par" es simple y coincide con lo que la persona eligió en cada unión.
- **Alternativas descartadas:** mostrar el aviso en la ficha lateral; resolver las cadenas eligiendo una sola fuente por familia.
- **Dónde se aplica:** `src/components/docbase/similar.ts` (`passedOverNotice`), `document-detail.tsx`, `src-tauri/src/docbase/mod.rs` (`passed_over_sql`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** construcción de S4 (el wiki usa la fuente elegida, P-078), 14 oct.

### D-146 · En un formulario PDF, de cada casilla o botón marcado se escribe qué opción es

- **Fecha:** 14 oct 2026
- **Tema:** Base documental
- **Estado:** en revisión (agente sin consulta; a confirmar según P-066)
- **El problema:** Desde el PR #38, lo completado en un formulario PDF se agrega a su texto. Para una casilla o un botón de opción marcado se escribía solo "campo: X", y eso no dice cuál opción se eligió: un "SÍ / NO" puede ser un campo con dos botones o dos casillas sueltas con nombres sin sentido ("Check Box12"). Además, la biblioteca que lee los PDF solo da por marcada una casilla cuyo estado se llama "Yes"; en formularios en castellano el estado suele llamarse "Sí", y esas casillas no se escribían. En el wiki, el modelo leyó invertidas dos respuestas de una ficha de cliente y generó una consulta sobre una discrepancia que no existía.
- **Qué se decidió:** Una casilla o botón cuenta como marcado si su estado es cualquiera distinto de "Off". De cada uno marcado se escribe una línea `- <campo>: <opción>`. La opción es, en este orden: el nombre que el formulario le da al botón elegido, si es una palabra real (no "Choice2", "On" ni un número); si no, el texto impreso al lado del botón (del lado donde están las etiquetas de toda esa fila); si no, su posición ("opción 2 de 3"). Si el campo tiene un nombre genérico ("Check Box12", "Group1"), se usa como nombre la pregunta impresa en la misma fila, antes de los botones. Una casilla suelta con nombre propio dice "marcado". Las no marcadas no se escriben: la pregunta más la etiqueta de la elegida ya dicen cuál fue la respuesta. Lo mismo vale para el derivado `fields` de la base.
- **Por qué:** Es la forma más corta que un lector (persona o modelo) no puede leer al revés, y sale solo de señales generales de la estructura del formulario, sin nombrar ningún caso (R-24).
- **Alternativas descartadas:** Escribir todas las opciones de la fila con su marca ("SI [x] NO [ ]"): más ruido y, cuando la pregunta ocupa varias líneas, el orden de las palabras se mezcla. Leer el valor de exportación con funciones de pdfium que la biblioteca no expone: exigía manejar el PDF a bajo nivel.
- **Dónde se aplica:** `src-tauri/src/commands/pdf_form.rs`; F-016.
- **Quién decidió:** Agente sin consulta.
- **Fuente:** Sesión desatendida del 14 oct (especificación F1).

### D-147 · El preprocesado recibe la ruta completa de la fuente, la misma que lee el ingest

- **Fecha:** 14 oct 2026
- **Tema:** Base documental
- **Estado:** en revisión (agente sin consulta; a confirmar según P-066)
- **El problema:** Al generar el wiki del archivo, cada documento de una carpeta montada dejaba en el registro un aviso de que no se pudo preprocesar: la base pasa las fuentes como ruta relativa al proyecto (`raw/sources/...`) y el preprocesado la buscaba así, sin el proyecto delante, y no la encontraba. El ingest se recuperaba porque al leer el archivo arma la ruta completa, pero el texto se extraía dos veces y el registro se llenaba de avisos falsos.
- **Qué se decidió:** Antes de preprocesar, toda ruta relativa se completa con la carpeta del proyecto, igual que hace el ingest al leer; el componente de fuentes (`source_volume`) la resuelve después a donde vive de verdad la carpeta montada. La cola sigue guardando la ruta tal como llegó.
- **Por qué:** Usa el mismo mecanismo que el ingest, en el único lugar por donde pasan todas las fuentes antes de la cola, sin tocar el código de Rust.
- **Alternativas descartadas:** Que la base pase rutas absolutas: arreglaba un solo llamador y cambiaba lo que guarda la cola. Resolver rutas relativas en Rust: el comando no conoce el proyecto.
- **Dónde se aplica:** `src/lib/source-lifecycle.ts` (`enqueueSourceIngest`); F-002.
- **Quién decidió:** Agente sin consulta.
- **Fuente:** Sesión desatendida del 14 oct (especificación F1).

### D-148 · Un círculo de elecciones no deja a todos fuera del wiki

- **Fecha:** 14 oct 2026
- **Tema:** Parecidos
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** un documento deja de alimentar el wiki si en algún par unido como el mismo documento se eligió al otro (D-145). Si las elecciones forman un círculo (A cede ante B, B ante C, C ante A), cada uno tiene otro elegido en su lugar y ninguno de los documentos unidos llega al wiki, sin que la persona lo haya querido. Con un solo par no puede pasar: elegir al otro en el mismo par solo los intercambia.
- **Qué se decidió:** al unir como el mismo documento (o cambiar la fuente elegida de un par ya unido), la base mira el conjunto de documentos unidos así con esos dos, con la elección nueva incluida. Si en ese conjunto no queda ninguno elegido en todos sus pares, la decisión se rechaza y no se guarda nada; la pantalla muestra: "Con esa elección ninguno de los N documentos unidos como el mismo quedaría como fuente del wiki: cada uno tiene otro elegido en lugar de él. Elegí el otro documento como fuente o deshacé una de esas uniones." Un conjunto sin círculos siempre tiene al menos uno, así que las cadenas y las estrellas no cambian. Las uniones entre documentos distintos (misma plantilla, etc.) no cuentan. El control se hace al decidir: si después se borra el documento que era la única fuente de un conjunto con un círculo, los demás pueden quedar afuera hasta que la persona cambie una elección.
- **Por qué:** rechazar es la regla más simple de explicar: lo que la persona eligió en cada par se respeta tal cual, y nunca se elige una fuente por ella. Resolverlo solo (por ejemplo, tomar la última elección como ganadora) cambiaría en silencio el efecto de una decisión anterior.
- **Alternativas descartadas:** dejar entrar a todos los del círculo; elegir sola una fuente por conjunto (D-145 ya descartó una fuente por familia); avisar sin rechazar.
- **Dónde se aplica:** `src-tauri/src/docbase/similar/store.rs` (`decide_similar`, `joined_without_source`); prueba `choices_that_close_a_circle_are_refused` en `similar/tests.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** arreglo pendiente de S4 al apilarla sobre S3 (P-078), 14 oct.

### D-149 · La comprobación de los PR y los instaladores, solo en Windows por ahora

- **Fecha:** 15 oct 2026
- **Tema:** Distribución
- **Estado:** vigente
- **El problema:** cada PR se compilaba en macOS, Linux y Windows, y los instaladores se generaban para cuatro plataformas. macOS era, lejos, lo que más minutos de GitHub Actions consumía, y el 14 de octubre GitHub desactivó las Actions del repositorio (el motivo no se conoce; se consultó a soporte). Al dueño hoy solo le interesa Windows.
- **Qué se decidió:** la comprobación de cada PR y la generación de instaladores corren solo en Windows. La regla que protege `main` exige solo la comprobación de Windows. macOS y Linux vuelven cuando la aplicación esté más estable (P-082); lo necesario para volver quedó comentado en los workflows.
- **Por qué:** concentra el esfuerzo y el consumo donde está el producto hoy.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `.github/workflows/ci.yml`, `.github/workflows/build.yml`, regla `proteger-main`; P-082.
- **Quién decidió:** el dueño del producto.
- **Fuente:** conversación del 15 oct; PR #49.
- **Nota del 16 oct:** las Actions del repositorio volvieron a funcionar; el 15 de octubre ya había corridas terminadas bien. La decisión no cambia.

### D-150 · La IA se usa donde agrega valor o el proceso la necesita

- **Fecha:** 15 oct 2026
- **Tema:** IA
- **Estado:** vigente (reemplaza en parte a D-099; precisa D-042)
- **El problema:** en distintas sesiones, los agentes que programan el proyecto leyeron "determinista primero" (D-042) y "reducir el uso de IA" (D-099) como una regla para evitar la IA, y llegaron a conclusiones como "esto no debería usar el modelo" aun cuando el proceso lo necesitaba o el modelo daba un resultado mejor.
- **Qué se decidió:** la IA se usa donde agrega valor y donde el proceso la necesita. Se prefiere una herramienta determinista solo si logra la misma eficacia o una mejor, con la misma eficiencia o una mejor. Eso no limita de ninguna manera el uso de la IA cuando el proceso la requiere o cuando logra un resultado superior al de las herramientas deterministas. El costo o el cupo no son argumentos para elegir un camino peor; antes de una prueba que consuma cupo se avisa al dueño, y avisar no es evitar.
- **Por qué:** la calidad del resultado es lo que importa; el sesgo contra la IA producía procesos que no cumplían su función.
- **Alternativas descartadas:** mantener "reducir el uso de IA" como regla general.
- **Dónde se aplica:** `CLAUDE.md` (sección "Uso de la inteligencia artificial"); principio 5 de `base-documental.md`; R-18 de `reglas-de-procesamiento.md`; sección 3 de `parecidos-propuesta.md`; toda propuesta y pasada de la base documental.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 15 oct.

### D-151 · No se sincroniza con upstream

- **Fecha:** 15 oct 2026
- **Tema:** Upstream
- **Estado:** vigente (reemplaza a D-034 y D-058)
- **El problema:** D-034 ya decía que las necesidades propias pesan más que la compatibilidad con upstream, pero la compatibilidad seguía apareciendo como criterio al diseñar y programar (tocar poco los archivos originales, analizar cada novedad de upstream).
- **Qué se decidió:** no se sincroniza con upstream (`nashsu/llm_wiki`). El proyecto diverge de forma definitiva; lo que haga upstream ya no interesa. El código se cambia según nuestros requerimientos, en cualquier archivo, y cuánto nos alejemos de upstream no importa ni preocupa. No se proponen contribuciones a upstream.
- **Por qué:** una regla simple, sin matices, que no se pueda leer como una restricción.
- **Alternativas descartadas:** seguir analizando las novedades de upstream una por una (D-034).
- **Dónde se aplica:** `CLAUDE.md` (sección "Upstream"); principio 8 y sección 9 de `base-documental.md`; P-008; `FORK_CHANGES.md` (D-152).
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 15 oct.

### D-152 · `FORK_CHANGES.md` se congela

- **Fecha:** 16 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente (reemplaza en parte a D-012: los demás registros siguen)
- **El problema:** `FORK_CHANGES.md` anotaba, por propósito, qué archivos del original tocaba el fork y si convenía proponer cada cambio al original. Servía para que traer novedades de upstream fuera barato. Desde D-151 no se sincroniza con upstream, así que mantenerlo es trabajo sin uso, y lo que cambió ya queda en este registro, en `pendientes.json` y en git.
- **Qué se decidió:** el archivo queda congelado, con una nota al principio que lo explica, como historia de lo que el fork cambió hasta el 16 de octubre. No se actualiza más y `CLAUDE.md` deja de pedirlo.
- **Por qué:** sin sincronización, el registro perdió su función.
- **Alternativas descartadas:** borrarlo (se pierde la historia sin ganar nada); seguir manteniéndolo como lista de funciones propias (ya la dan este registro y el documento de diseño).
- **Dónde se aplica:** `FORK_CHANGES.md`; `CLAUDE.md`.
- **Quién decidió:** dueño del producto, sobre la recomendación del agente.
- **Fuente:** conversación del 15 y 16 oct.

### D-153 · Estado "a verificar" en los pendientes

- **Fecha:** 16 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente
- **El problema:** once pendientes figuraban "en curso" aunque su código estaba terminado e integrado; solo faltaba que el dueño lo viera en la app. Mezclados con el trabajo que de verdad está en marcha, daban la impresión de que había mucho más a medio hacer, y no se veía qué le tocaba mirar al dueño.
- **Qué se decidió:** un estado propio, `a-verificar`, para lo construido e integrado que espera que el dueño lo vea en la app. Si lo aprueba pasa a `hecho`; si pide cambios, vuelve a `pendiente` o `en-curso`.
- **Por qué:** separar lo que espera al dueño de lo que espera al agente.
- **Alternativas descartadas:** dejarlos "en curso".
- **Dónde se aplica:** `pendientes.json` (valores y reglas; P-011, P-014, P-021, P-023, P-031 a P-034, P-037, P-038, P-047).
- **Quién decidió:** dueño del producto, sobre la propuesta del agente.
- **Fuente:** conversación del 16 oct.

### D-154 · El producto mínimo no está cerrado

- **Fecha:** 16 oct 2026
- **Tema:** Producto
- **Estado:** vigente
- **El problema:** todo lo que lista el pendiente del producto mínimo (P-049) está construido y el wiki del archivo ya se generó, y el agente propuso darlo por cerrado.
- **Qué se decidió:** no se cierra. Hay una primera versión construida, pero la experiencia de uso deja mucho que desear y el wiki que generó es inservible. Que algo esté construido no lo hace terminado: un paso se cierra cuando cumple su función para quien lo usa. El dueño va a traer una propuesta de diseño para revisarla, ajustarla e implementarla.
- **Por qué:** el producto mínimo tiene que servir a un estudio, no solo existir.
- **Alternativas descartadas:** cerrarlo porque las tareas que listaba estaban hechas.
- **Dónde se aplica:** P-049 y los pendientes que dependen de él.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 16 oct.

### D-155 · Cómo cuenta el contador de Atención

- **Fecha:** 16 oct 2026
- **Tema:** Interfaz
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** la propuesta de la vista de atención dice que el contador suma problemas y decisiones, pero no dice qué es "uno". Contar documentos infla el número (una familia de trece facturas serían trece), y hay avisos que no se pueden resolver (74 accesos directos de un disco en la nube) que dejarían el contador siempre encendido.
- **Qué se decidió:** un problema cuenta una vez por línea (cada línea es algo que no funciona). Una decisión cuenta por cada decisión que hay que tomar: una familia de parecidos es una, un par suelto es una, cada tipo propuesto es una, cada propuesta del wiki es una, cada enlace roto es uno. Las líneas de baja prioridad (documentos demasiado grandes, accesos directos de la nube, enlaces rotos, documentos cancelados en la cola) se muestran marcadas "puede esperar" y no cuentan. Inicio dice lo mismo que el contador ("2 problemas · 9 decisiones").
- **Por qué:** sigue el principio 6 de la propuesta (lo que se resuelve en bloque es una decisión) y el ejemplo de su sección 4; un contador que nunca baja deja de servir.
- **Alternativas descartadas:** contar líneas también en las decisiones (esconde cuánto trabajo hay); contar documentos (infla); contar todo (el contador no se apaga nunca).
- **Dónde se aplica:** `src/lib/attention/order.ts` (`attentionCounts`); sección 13 de `base-documental.md`.
- **Quién decidió:** agente sin consulta (tanda desatendida del 16 oct; P-066).
- **Fuente:** construcción de la vista de atención (P-083), rama `feat/attention-view`.

### D-156 · Las fuentes de avisos son piezas de la pantalla con una forma común

- **Fecha:** 16 oct 2026
- **Tema:** Interfaz
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** la vista de atención tiene que juntar avisos de partes que viven en lugares distintos: unos en el motor (la base documental), otros solo en la pantalla (la cola del wiki, su revisión, los cambios de archivos). Y la capa de entidades que viene tiene que sumar los suyos sin rehacer la vista.
- **Qué se decidió:** cada fuente es una pieza de la pantalla registrada en una lista (`src/components/attention/sources/index.ts`): lee lo que su parte guarda y entrega sus avisos con una forma común (`src/lib/attention/types.ts`: clase, tema, prioridad, cantidad, título, por qué importa, qué hacer, cuándo deja de ser cierto, acciones, casos). Cada vez que algo cambia entrega la lista entera: lo que deja de entregar desaparece. Las partes del motor exponen una consulta y su fuente arma los textos en la pantalla, donde está el vocabulario. Una fuente que falla se muestra como un problema propio sin tirar a las demás.
- **Por qué:** los textos llevan términos del vertical, que viven en el vocabulario de la pantalla (D-072); y la mitad de las fuentes no está en el motor.
- **Alternativas descartadas:** que el motor entregue los avisos ya redactados (los términos del dominio quedarían escritos en el motor); una sola consulta que lo calcule todo (cada parte nueva obligaría a tocarla).
- **Dónde se aplica:** `src/components/attention/`; `src/stores/attention-store.ts`.
- **Quién decidió:** agente sin consulta (tanda desatendida del 16 oct; P-066).
- **Fuente:** construcción de la vista de atención (P-083), rama `feat/attention-view`.

### D-157 · El panel de actividad del wiki queda reducido a una línea

- **Fecha:** 16 oct 2026
- **Tema:** Interfaz
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** la sección "En curso" de Atención reemplaza al panel de actividad del costado del wiki. Había que decidir si el panel se quitaba del todo o quedaba como enlace.
- **Qué se decidió:** queda una sola línea al pie del costado del wiki, solo cuando hay algo: dice el primer problema de la cola o de los cambios de archivos o, si no hay, lo que está en curso ("Incorporando al wiki: 12 de 40 documentos"), y al hacer clic lleva a Atención, donde se controla. Los controles del panel viejo (pausar, reanudar, reintentar, cancelar, subir en la cola, reintentar o ignorar un cambio) pasaron a Atención; la selección múltiple de la cola y la lista de páginas escritas por cada incorporación no pasaron (P-089).
- **Por qué:** quien trabaja en el wiki miraba ese lugar para saber si la cola avanzaba; "En curso" no suma al contador, así que sin la línea no se enteraría. Duplicar los controles en dos lugares contradice "un solo lugar".
- **Alternativas descartadas:** quitarlo del todo (se pierde de vista el avance desde el wiki); dejarlo completo (dos lugares para lo mismo, uno en inglés).
- **Dónde se aplica:** `src/components/layout/activity-link.tsx`; se borró `activity-panel.tsx`.
- **Quién decidió:** agente sin consulta (tanda desatendida del 16 oct; P-066).
- **Fuente:** construcción de la vista de atención (P-083), rama `feat/attention-view`.

### D-158 · Clase y prioridad de los avisos que la propuesta no fijaba

- **Fecha:** 16 oct 2026
- **Tema:** Interfaz
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** la tabla de la sección 5 de la propuesta clasifica los avisos de hoy, pero no todos los que aparecieron al construirla, ni su prioridad dentro de la clase.
- **Qué se decidió:** prioridad alta para lo que frena todo lo demás (un origen que no se puede leer, archivos que no se pudieron leer o sin texto, falta configurar el modelo); normal para el resto; baja para lo que queda afuera sin que se pueda hacer mucho o lo que puede esperar. Los documentos cancelados en la cola van a "En curso" con prioridad baja, con "Volver a encolar" y "Quitar de la cola" (acción nueva: saca de la cola, para siempre, un documento fallido o cancelado). Lo que la base todavía está leyendo y lo que el wiki hace con el modelo fuera de la cola (una revisión con el modelo, el repaso de propuestas resueltas) van a "En curso". Los errores de la incorporación se dicen en palabras (límite de uso, sin conexión, clave rechazada, tardó demasiado, archivo no encontrado); los que no se reconocen se muestran como vinieron.
- **Por qué:** ordenar por lo que bloquea primero; no dejar en la cola cosas que nadie puede sacar.
- **Alternativas descartadas:** una sola prioridad por clase (los accesos directos de la nube quedarían a la par de un origen desconectado).
- **Dónde se aplica:** `src/components/attention/sources/` (docbase, wiki-queue, file-sync); `removeStoppedTasks` en `src/lib/ingest-queue.ts`.
- **Quién decidió:** agente sin consulta (tanda desatendida del 16 oct; P-066).
- **Fuente:** construcción de la vista de atención (P-083), rama `feat/attention-view`.

### D-159 · Pares fuera de las familias y confirmación de una familia

- **Fecha:** 16 oct 2026
- **Tema:** Base documental
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** la vista de parecidos se ordena por familia, pero hay pares que no son entre el principal de una familia y uno de sus miembros: dos miembros de la misma familia entre sí, o un documento contenido en otro (esa relación no arma familias). Y "confirmar una familia de la misma plantilla de un clic" podía entenderse como unir todos los pares de la familia o solo los del principal.
- **Qué se decidió:** la vista muestra las familias (primero las que tienen dos que podrían ser el mismo documento, después las que tienen otra cosa por decidir, al final las ya decididas, plegadas) y aparte los "pares fuera de las familias". Confirmar la familia une como "misma plantilla" solo los pares del principal con sus miembros que siguen propuestos como "misma plantilla", con la evidencia que se vio (R-08); los pares propuestos con otra relación quedan para decidirlos de a uno. Las decisiones se guardan juntas y las familias se rehacen una sola vez. Cada unión se puede deshacer desde su par.
- **Por qué:** las familias se arman contra el principal (R-14); unir dos miembros entre sí por estar en la misma familia sería inferir sobre lo inferido (R-02).
- **Alternativas descartadas:** unir todos los pares de la familia; esconder los pares sueltos (no se podrían decidir desde la vista).
- **Dónde se aplica:** `src/components/docbase/similar-view.tsx`; `decide_similar_many` y `similar_open_pairs` en `src-tauri/src/docbase/similar/store.rs`.
- **Quién decidió:** agente sin consulta (tanda desatendida del 16 oct; P-066).
- **Fuente:** construcción de la vista de atención (P-083), rama `feat/attention-view`.

### D-160 · Las diferencias lado a lado se calculan en la pantalla

- **Fecha:** 16 oct 2026
- **Tema:** Base documental
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** P-077 pedía mostrar los dos textos de un par lado a lado, con lo que cambia resaltado y los datos aparte de la redacción. El motor alinea los textos al medir, pero no guarda la alineación.
- **Qué se decidió:** la pantalla compara, a pedido, los dos textos que la base ya tiene, palabra por palabra (algoritmo de Myers: su costo crece con cuánto difieren, no con cuán largos son). Un cambio es un dato cuando todas sus palabras tienen un dígito (importes, fechas, números, identificadores); si no, es redacción. Los tramos iguales largos se pliegan. Pasadas las 2.500 diferencias no se muestra y se dice por qué. No se guarda nada.
- **Por qué:** es determinista y da el mismo resultado que el motor para mostrar; no necesita otro comando ni espacio en la base.
- **Alternativas descartadas:** un comando del motor que devuelva su alineación (más trabajo para el mismo resultado en pantalla); clasificar los datos como el motor (identificador, importe, fecha, nombre) en la pantalla (duplicaría su lógica; queda como mejora si hace falta).
- **Dónde se aplica:** `src/components/docbase/text-diff.ts` y `diff-panel.tsx`; P-077.
- **Quién decidió:** agente sin consulta (tanda desatendida del 16 oct; P-066).
- **Fuente:** construcción de la vista de atención (P-083), rama `feat/attention-view`.

### D-161 · Dónde viven los tipos confirmados y cuándo los ve el modelo

- **Fecha:** 16 oct 2026
- **Tema:** Base documental
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** al confirmar un tipo propuesto, tiene que pasar a la lista cerrada (R-22). El catálogo de tipos es un archivo que viene con la app, y la lista entra en cada pregunta al modelo: si cambiara la pregunta de todos los documentos, se volverían a preguntar todos (la misma pregunta no se hace dos veces, R-20, pero una pregunta distinta sí).
- **Qué se decidió:** las decisiones sobre tipos se guardan en la base (tabla `type_decision`, versión 8 del esquema), con el nombre que les dio la persona. Al confirmar, los documentos de ese tipo dejan de ser propuestas y la pantalla los nombra con ese nombre. La lista cerrada de la pregunta suma los tipos confirmados, pero solo para las preguntas que se hacen de nuevo (documentos nuevos o que cambiaron): lo ya respondido no se vuelve a preguntar. Sin decisiones, la pregunta es palabra por palabra la de antes. Cambiar el nombre es confirmar con otro nombre; el identificador neutro no cambia. Todo se puede deshacer (R-09).
- **Por qué:** cumplir R-22 sin volver a preguntar el archivo entero cada vez que alguien confirma un tipo.
- **Alternativas descartadas:** escribir el tipo en el catálogo del disco (es de la app y se reemplaza con cada versión); volver a preguntar todo con la lista nueva (costo sin ganancia: lo ya clasificado no cambia).
- **Dónde se aplica:** `src-tauri/src/docbase/similar/classify.rs` (`decide_type`, `type_list`); `src-tauri/src/docbase/schema.rs` (V8); `document-types-review.tsx`; P-080.
- **Quién decidió:** agente sin consulta (tanda desatendida del 16 oct; P-066).
- **Fuente:** construcción de la vista de atención (P-083), rama `feat/attention-view`.

### D-162 · Qué pasa al rechazar un tipo propuesto

- **Fecha:** 16 oct 2026
- **Tema:** Base documental
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** un "no" de una persona tiene que bloquear (R-08), pero un documento sin tipo vuelve a preguntarse, y el modelo, con la misma pregunta, daría la misma respuesta.
- **Qué se decidió:** al rechazar, los documentos de ese tipo lo pierden (y sus datos), y su pregunta se vuelve a hacer con una línea más: los tipos que una persona rechazó y que no tiene que proponer. Se hace en la próxima vuelta de preguntas (sola, si el wiki del archivo está activo; o desde Orígenes). Si el modelo insiste con un tipo rechazado, la respuesta no se usa y el documento queda sin tipo.
- **Por qué:** la decisión de la persona es evidencia nueva, y sin decírsela al modelo se repetiría la respuesta.
- **Alternativas descartadas:** dejar los documentos con el tipo marcado como rechazado (seguiría apareciendo como tipo); asignarles "sin tipo" para siempre (no se clasificarían nunca).
- **Dónde se aplica:** `src-tauri/src/docbase/similar/classify.rs` (`rows_from_answer`, `rejected_text`).
- **Quién decidió:** agente sin consulta (tanda desatendida del 16 oct; P-066).
- **Fuente:** construcción de la vista de atención (P-083), rama `feat/attention-view`.

### D-163 · La revisión de enlaces que corre sola

- **Fecha:** 16 oct 2026
- **Tema:** Wiki
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** el dueño decidió que la revisión estructural de enlaces corra sola al vaciarse la cola del wiki, para que su número coincida con el de la vista. La revisión de enlaces también guarda lo que encuentra el modelo cuando una persona lo pide, y había que decidir qué pasa con eso.
- **Qué se decidió:** cada vez que la cola del wiki termina (después del repaso de propuestas resueltas), se revisan los enlaces sin modelo, con la configuración de la revisión de enlaces del proyecto, y sus resultados reemplazan a los estructurales anteriores; lo que encontró el modelo queda como estaba. La vista de atención cuenta los enlaces rotos de esa revisión, no los del índice de la base, y lleva a la revisión de enlaces.
- **Por qué:** así la vista y la revisión dicen siempre lo mismo, sin gastar modelo.
- **Alternativas descartadas:** borrar todo y volver a revisar (se perdería lo que el modelo encontró a pedido); seguir contando el índice de la base (no coincide con la revisión).
- **Dónde se aplica:** `src/lib/auto-structural-lint.ts`; `onQueueDrained` en `src/lib/ingest-queue.ts`.
- **Quién decidió:** agente sin consulta (tanda desatendida del 16 oct; P-066).
- **Fuente:** construcción de la vista de atención (P-083), rama `feat/attention-view`.

### D-164 · Los avisos del wiki del archivo con otro proyecto abierto

- **Fecha:** 16 oct 2026
- **Tema:** Interfaz
- **Estado:** en revisión (agente sin consulta, P-066)
- **El problema:** el dueño decidió que la vista muestre los avisos del archivo y los del proyecto abierto. La revisión y los enlaces rotos del wiki del archivo se resuelven con el archivo abierto, y la app trabaja con un proyecto abierto a la vez.
- **Qué se decidió:** las líneas del proyecto abierto dicen de qué proyecto son cuando no es el archivo. Mientras tanto, la vista lee del disco las propuestas y los enlaces rotos del wiki del archivo (cada 30 segundos) y los muestra marcados "Wiki del archivo", sin botones, diciendo que se resuelven abriendo el archivo. Lo de la base documental es siempre del archivo.
- **Por qué:** cumplir la decisión sin abrir dos proyectos a la vez.
- **Alternativas descartadas:** un botón que cambie de proyecto desde la línea (cambiar de proyecto detiene la cola y recarga todo; no es algo para un clic de paso).
- **Dónde se aplica:** `src/components/attention/sources/wiki.ts`.
- **Quién decidió:** agente sin consulta (tanda desatendida del 16 oct; P-066).
- **Fuente:** construcción de la vista de atención (P-083), rama `feat/attention-view`.
### D-170 · Una capa de conocimiento entre la base documental y el wiki

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente
- **El problema:** el wiki generado con la carpeta de prueba no sirve para un estudio: de 39 páginas de entidad, 28 tienen una sola fuente; la contraparte principal aparece en 31 páginas de documentos y su página usa 5. La causa está en cómo trabaja la incorporación del wiki: procesa un documento por vez, el modelo ve solo los títulos de las páginas existentes, decide por documento si una entidad "merece página", la identidad es el nombre del archivo, no hay alias, la procedencia es solo por página y borrar una fuente no retira lo que aportó.
- **Qué se decidió:** una capa nueva de conocimiento, con registro propio dentro de la base del espacio (la misma base SQLite de la base documental, como módulo aparte): entidades canónicas con identificador estable, sus alias, sus menciones en cada documento con su rol, y las afirmaciones que hacen los documentos sobre ellas, cada una con su evidencia. El wiki pasa a ser una proyección de ese registro.
- **Por qué:** los problemas medidos son de identidad y de procedencia, y no se arreglan mejorando las instrucciones de una incorporación que mira un documento por vez. Hace falta un registro que vea todo el archivo y guarde de dónde sale cada dato.
- **Alternativas descartadas:** la alternativa A, de cambios mínimos sobre la incorporación actual (pasarle al modelo el contenido de las páginas candidatas, agregar alias): mantiene la identidad por nombre de archivo y la procedencia por página.
- **Dónde se aplica:** `docs/design/conocimiento-propuesta.md`; `docs/design/base-documental.md` (secciones 3, 4, 9 y 11); `src-tauri/src/docbase/knowledge/`.
- **Quién decidió:** dueño del producto, sobre el análisis del agente ("tomamos todas tus sugerencias").
- **Fuente:** propuesta del dueño del 15 oct y conversación del 16 oct.

### D-171 · Toda afirmación lleva una cita verificada por cálculo

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente
- **El problema:** el modelo puede atribuirle a un documento algo que no dice. Si las páginas de entidad se escriben desde lo que extrajo el modelo, y eso no se comprueba, se infiere sobre lo inferido (R-02).
- **Qué se decidió:** toda mención y toda afirmación que extrae el modelo trae su cita textual, y un cálculo comprueba que la cita está en el texto del documento, con la normalización de la base. Si no está, se descarta. Una afirmación tiene sujeto, predicado, objeto (una entidad o un valor), documento, cita y su ubicación (la página cuando se puede), el tiempo del hecho si el documento lo dice, y un estado: respaldada, en conflicto, retirada o pendiente. Varias evidencias para la misma afirmación no la duplican. Las contradicciones se guardan y se muestran; nunca gana el último documento.
- **Por qué:** así todo queda anclado a la fuente (R-01), y escribir páginas desde afirmaciones verificadas no viola R-02.
- **Alternativas descartadas:** confiar en lo que devuelve el modelo; resolver las contradicciones quedándose con lo más reciente.
- **Dónde se aplica:** `docs/design/conocimiento-propuesta.md` (secciones 2 y 5); `src-tauri/src/docbase/knowledge/` (verificación de citas).
- **Quién decidió:** dueño del producto, sobre el análisis del agente.
- **Fuente:** conversación del 16 oct.

### D-172 · Persona, organización y asunto, con roles como datos

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente
- **El problema:** hay que elegir qué clases de cosas registra la capa sin diseñar de más, y cómo se nombra el papel de cada una en un documento sin escribir el dominio en el programa.
- **Qué se decidió:** los tipos de entidad iniciales son persona, organización y asunto (expediente, caso, contrato marco). Cada mención lleva su rol en el documento (parte, apoderado, firmante, testigo, domicilio constituido, emisor, destinatario, etc.). Los tipos, los roles y los predicados de las afirmaciones son datos del vocabulario de la capa, no van escritos en el código. Conceptos, productos y eventos quedan para después; los conceptos siguen con el wiki como hoy.
- **Por qué:** son las entidades que importan en un estudio jurídico, y como datos se pueden ampliar o cambiar por vertical (R-26).
- **Alternativas descartadas:** una ontología amplia desde el principio.
- **Dónde se aplica:** `src-tauri/resources/catalog/knowledge.json`; `docs/design/conocimiento-propuesta.md` (sección 6).
- **Quién decidió:** dueño del producto, sobre el análisis del agente.
- **Fuente:** conversación del 16 oct.

### D-173 · La identidad de las entidades se resuelve en cascada, con abstención

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente
- **El problema:** hay que decidir cuándo dos menciones son la misma entidad. Unir dos personas distintas mezcla sus documentos en todas las páginas; dejar separada a una misma persona la fragmenta.
- **Qué se decidió:** en cascada, con tres salidas posibles (la misma, distinta, dudosa): 1) identificadores (DNI, CUIT, matrícula, número de expediente): el mismo identificador une solo, con posibilidad de deshacer; uno distinto no une nunca; 2) nombre normalizado, atributos y contexto (iniciales, orden de apellidos, abreviaturas societarias), sabiendo que "S.A.", "LLC" y "Ltd" de un mismo nombre son organizaciones distintas salvo evidencia; 3) el modelo, con evidencia acotada, respuesta estructurada y la opción de abstenerse, para los candidatos difíciles, por grupo; 4) lo dudoso va a una persona. Ante la duda no se une (R-07). Si los nombres coinciden y los identificadores difieren por poco (un escaneo mal leído), el caso es dudoso.
- **Por qué:** una unión falsa es mucho peor que una separación; el identificador es la evidencia más fuerte que dan los documentos.
- **Alternativas descartadas:** unir por parecido del nombre; pedirle siempre la decisión a una persona, aun con el mismo identificador.
- **Dónde se aplica:** etapa 2 de la capa (`docs/design/conocimiento-propuesta.md`, sección 5).
- **Quién decidió:** dueño del producto (la unión automática por identificador, expresamente), sobre el análisis del agente.
- **Fuente:** conversación del 16 oct.

### D-174 · Las decisiones de identidad de una persona son firmes y reversibles

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente
- **El problema:** una persona va a unir, separar y corregir entidades. Si un recálculo de la capa deshace lo que decidió, el trabajo se pierde y la persona deja de confiar.
- **Qué se decidió:** las decisiones de una persona (unir, separar, "este dato está mal") se guardan como las de los documentos parecidos: con la evidencia que se le mostró y su huella, firmes, reversibles y nunca pisadas por un recálculo (R-08, R-09). Las correcciones se hacen como decisiones, no editando el texto de una página.
- **Por qué:** lo que decide una persona es la evidencia más fuerte que hay.
- **Alternativas descartadas:** corregir editando la página del wiki.
- **Dónde se aplica:** tabla `knowledge_decision` de la base; etapa 2 de la capa.
- **Quién decidió:** dueño del producto, sobre el análisis del agente.
- **Fuente:** conversación del 16 oct.

### D-175 · La página de una entidad se proyecta desde el registro

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente
- **El problema:** hoy el modelo escribe las páginas de entidad documento por documento, y la página de una persona parece completa aunque le falte casi todo. Además las instrucciones al modelo son de investigación académica y le permiten agregar conocimiento externo.
- **Qué se decidió:** la página de una entidad tiene dos partes: la lista completa de sus documentos, agrupada por asunto, sin modelo y siempre al día; y una síntesis escrita por el modelo solo desde afirmaciones verificadas, con referencias a sus documentos. Se rehace solo cuando cambian sus afirmaciones o menciones. Las notas de una persona van en una sección protegida que la app nunca reescribe. La incorporación actual del wiki deja de crear páginas de entidad: sigue escribiendo la página de cada documento y los conceptos, con instrucciones adaptadas a un estudio jurídico (sin conocimiento externo), y enlaza a las páginas que proyecta la capa. No se busca compatibilidad con Obsidian.
- **Por qué:** la lista de documentos sale del registro y no se le escapa ninguno; la síntesis no puede decir nada que no tenga cita.
- **Alternativas descartadas:** seguir dejando que la incorporación escriba las páginas de entidad; mantener la compatibilidad con Obsidian.
- **Dónde se aplica:** etapa 3 de la capa; `src/lib/ingest.ts`.
- **Quién decidió:** dueño del producto, sobre el análisis del agente.
- **Fuente:** conversación del 16 oct.

### D-176 · Sin derivaciones por reglas, incremental, con el costo de modelo aprobado

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente
- **El problema:** la propuesta original incluía derivar relaciones por reglas, lo que exige un grafo de dependencias entre afirmaciones; y hay que saber cuánto modelo se acepta gastar.
- **Qué se decidió:** sin derivaciones por reglas en esta etapa (quedan como prioridad siguiente): alcanza con el trabajo incremental de la base. Documento nuevo: se extrae, se resuelven sus menciones y se rehacen solo las páginas tocadas. Documento borrado o reemplazado: se retiran sus menciones y afirmaciones, se conservan las que tienen otra evidencia y se rehacen las páginas afectadas. Reprocesar sin cambios no cambia nada (R-13). El costo de modelo aprobado: una lectura por documento y por espacio, preguntas sueltas para lo dudoso y un texto por entidad tocada.
- **Por qué:** sin derivaciones no hace falta el grafo de dependencias, y la capa se puede construir sobre lo que la base ya hace.
- **Alternativas descartadas:** construir las derivaciones desde el principio.
- **Dónde se aplica:** `docs/design/conocimiento-propuesta.md` (secciones 3 y 5).
- **Quién decidió:** dueño del producto, sobre el análisis del agente.
- **Fuente:** conversación del 16 oct.
