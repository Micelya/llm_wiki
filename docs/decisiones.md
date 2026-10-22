# Registro de decisiones

**Proyecto:** Micelya Desktop (repositorio `Micelya/llm_wiki`) · **Al 22 de octubre de 2026**

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
| D-009 | 5 al 8 oct | IA | Proveedor de IA: Codex en desarrollo; para clientes, suscripción propia o servicio de Amazon detrás de Micelya | vigente; reemplazada en parte por D-468 |
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
| D-052 | 11 oct | Producto | Producto mínimo: etapa 1 más el puente con el wiki | reemplazada por D-463 |
| D-053 | 11 oct | Proceso de trabajo | Producto mínimo primero; lo posterior queda "en espera" | vigente |
| D-054 | 11 oct | Interfaz | Cuatro pantallas de la base, con el estilo actual de la app | vigente |
| D-055 | 11 oct | Interfaz | Marco visual y marca: para toda la app, en un turno dedicado | vigente |
| D-056 | 11 oct | Interfaz | Pantallas nuevas en español; traducción completa aparte | vigente |
| D-057 | 11 oct | Etiquetas | "Cliente" y "caso" son etiquetas, no proyectos | vigente |
| D-058 | 11 oct | Upstream | No proponer contribuciones a upstream hasta nuevo aviso | reemplazada por D-151 |
| D-059 | 11 oct | IA | La base no le manda nada al modelo por su cuenta | vigente |
| D-060 | 11 oct | Wiki | Asignar el proyecto general apaga su incorporación automática | reemplazada por D-071 |
| D-061 | 11 oct | Wiki | El proyecto general se elige entre los existentes | reemplazada por D-069 |
| D-062 | 11 oct | Interfaz | La app abre en Inicio | vigente |
| D-063 | 11 oct | Wiki | Quitar una carpeta de la base deja las páginas ya generadas | en revisión |
| D-064 | 11 oct | Base documental | Vigilar solo por barrido periódico | en revisión |
| D-065 | 11 oct | Wiki | "Qué cambió" lo sigue detectando el vigilante del wiki | en revisión |
| D-066 | 11 oct | Base documental | Archivos que el inventario ignora | vigente |
| D-067 | 11 oct | Interfaz | Mensajes de error del motor en inglés, por ahora | en revisión |
| D-068 | 11 oct | Etiquetas | Dejar vacía una etiqueta corregida vuelve al valor de la carpeta | en revisión |
| D-069 | 12 oct | Producto | La app crea sola el proyecto general y lo llama "Archivo" | reemplazada por D-395 |
| D-070 | 12 oct | Producto | Nombres técnicos neutros; la carpeta de trabajo se pregunta una vez | vigente |
| D-071 | 12 oct | Wiki | El wiki del Archivo se activa a mano y después se alimenta solo | vigente |
| D-072 | 12 oct | Producto | Términos del dominio en un vocabulario por profesión e idioma | vigente |
| D-073 | 12 oct | Base documental | "Excluir" en vez de "borrar" | vigente |
| D-074 | 12 oct | Interfaz | Orígenes como árbol, con acciones por fila | vigente |
| D-075 | 12 oct | Etiquetas | Grupos y etiquetas, reglas por carpeta, lo manual manda | vigente |
| D-076 | 12 oct | Wiki | Las páginas son la verdad; la base lleva un índice reconstruible | en revisión |
| D-077 | 12 oct | Interfaz | Si había otro proyecto abierto, la app lo reabre | reemplazada por D-395 |
| D-078 | 12 oct | Base documental | "Volver a barrer" en una carpeta barre todo su origen | vigente |
| D-079 | 12 oct | Wiki | Excluir no quita del wiki lo ya incorporado | reemplazada por D-396 |
| D-080 | 12 oct | Wiki | Enlaces emparejados sin distinguir mayúsculas, espacios ni guiones | reemplazada por D-396 |
| D-081 | 12 oct | Etiquetas | El color de cada etiqueta lo pone la app | en revisión |
| D-082 | 12 oct | Wiki | El índice de enlaces todavía no lo usan grafo, búsqueda ni revisión | en revisión |
| D-083 | 12 oct | Producto | Un solo vertical, fijo, por ahora | en revisión |
| D-084 | 12 oct | Distribución | Probar el instalador en otra máquina antes de entregarlo | vigente |
| D-085 | 13 oct | Idioma | El Archivo nace con un idioma de wiki fijo | vigente |
| D-086 | 13 oct | Idioma | Un idioma fijo por estudio, con el ajuste que la app ya tiene | vigente |
| D-087 | 13 oct | Idioma | El wiki en varios idiomas se posterga mucho | en revisión |
| D-088 | 13 oct | Idioma | Detección de idioma sin IA pero confiable; idioma como dato de la base | vigente |
| D-089 | 13 oct | Wiki | "Pausar" el wiki también frena la cola | reemplazada por D-396 |
| D-090 | 13 oct | Etiquetas | Caja de etiquetas: Enter no crea por accidente; sin nombres fijos de clientes | vigente |
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
| D-116 | 14 oct | Parecidos | Texto de rutina: lo que aparece en la mitad de los documentos o más, y en al menos diez | vigente |
| D-117 | 14 oct | Parecidos | "Comparten" se mide palabra por palabra sobre el texto alineado; las secuencias de cinco palabras solo buscan candidatos | vigente |
| D-118 | 14 oct | Parecidos | Misma plantilla si se reemplaza un identificador o un importe, o si lo reemplazado es mayormente datos | vigente |
| D-119 | 14 oct | Parecidos | Firma de una parte: la que no es del servicio, o la que registra el historial del servicio | vigente |
| D-120 | 14 oct | Base documental | El catálogo de tipos y relaciones es un archivo de datos dentro de la app, reemplazable sin compilar | vigente |
| D-121 | 14 oct | Parecidos | El principal de una familia es el firmado por una parte; si no, el de más texto | vigente |
| D-122 | 14 oct | Parecidos | Alineación propia, sin sumar dependencias | vigente |
| D-123 | 14 oct | Parecidos | Solo se comparan documentos con todo su texto; los que esperan reconocimiento quedan afuera | vigente |
| D-124 | 14 oct | Proceso de trabajo | Las pruebas existentes dejan de usar nombres de la carpeta de prueba | vigente |
| D-125 | 14 oct | Parecidos | Los nombres de las relaciones salen del vocabulario, con la frase que dice su dirección; una relación sin nombre se muestra por su identificador | vigente |
| D-126 | 14 oct | Base documental | El vocabulario admite términos por país, que reemplazan solo lo que cambian y solo sobre su propio idioma | vigente |
| D-127 | 14 oct | Parecidos | Qué cuenta como "espera una decisión" y cómo lo muestra Inicio: una línea por familia (hasta cinco) y una para el resto | vigente |
| D-128 | 14 oct | Parecidos | Las relaciones que se pueden elegir: las del catálogo, medidas e inferidas, sin la automática ni "parecidos" | vigente |
| D-129 | 14 oct | Parecidos | Qué fuente del wiki sugiere la app al unir dos documentos como el mismo, y cómo se elige | vigente; reemplazada en parte por D-143 |
| D-130 | 14 oct | Parecidos | Ver las diferencias lado a lado queda para después, con el botón visible y deshabilitado | vigente |
| D-131 | 14 oct | Parecidos | Cómo se ve cada parecido en el detalle de un documento: tarjeta, medidas en palabras y unir en un clic cuando no hay nada que elegir | vigente |
| D-132 | 14 oct | Parecidos (catálogo de tipos) | Un servicio de firma o un formulario no es el tipo de un documento | vigente |
| D-133 | 14 oct | Parecidos (catálogo de tipos) | Los tipos que se agregaron al catálogo | vigente |
| D-134 | 14 oct | Parecidos (uso del modelo) | Qué pregunta se le hace al modelo por cada familia | vigente |
| D-135 | 14 oct | Parecidos (uso del modelo) | Cuándo las marcas alcanzan y no se pregunta | vigente |
| D-136 | 14 oct | Parecidos (uso del modelo) | La misma pregunta no se repite: cómo se reconoce | vigente |
| D-137 | 14 oct | Parecidos (uso del modelo) | Respuesta del modelo en JSON estricto, con el camino de Codex que ya existía | vigente |
| D-138 | 14 oct | Parecidos (uso del modelo) | Cuándo se le pregunta al modelo | vigente |
| D-139 | 14 oct | Parecidos (pasada 5) | Cómo se buscan los datos de cada tipo | vigente |
| D-140 | 14 oct | Parecidos (pasada 6) | Cómo corrigen las relaciones las reglas del tipo | vigente |
| D-141 | 14 oct | Parecidos (medición) | El texto de las imágenes en la prueba real | vigente |
| D-142 | 14 oct | Parecidos (catálogo de tipos, uso del modelo) | Las etapas se describen en el catálogo, y eso va en la pregunta | vigente |
| D-143 | 14 oct | Parecidos | Al unir dos documentos como el mismo, el no elegido deja de ser fuente del wiki en todas sus ubicaciones; lo ya incorporado queda | vigente |
| D-144 | 14 oct | Parecidos | Después de unir o deshacer, la cola sigue la decisión: sale lo que espera y ya no alimenta el wiki, y entra la fuente que nunca se incorporó | vigente |
| D-145 | 14 oct | Parecidos | El aviso "no alimenta el wiki" va en la sección "En el wiki" del detalle, y vale cualquier par unido en que se eligió al otro | vigente; reemplazada en parte por D-148 |
| D-146 | 14 oct | Base documental | En un formulario PDF, de cada casilla o botón marcado se escribe qué opción es | vigente |
| D-147 | 14 oct | Base documental | El preprocesado recibe la ruta completa de la fuente, la misma que lee el ingest | vigente |
| D-148 | 14 oct | Parecidos | Una elección de fuente que dejaría sin ninguna fuente a documentos unidos como el mismo se rechaza, con un mensaje que dice cómo salir | vigente |
| D-149 | 15 oct | Distribución | La comprobación de los PR y los instaladores, solo en Windows por ahora | vigente |
| D-150 | 15 oct | IA | La IA se usa donde agrega valor o el proceso la necesita; lo determinista, solo si iguala o mejora el resultado | vigente |
| D-151 | 15 oct | Upstream | No se sincroniza con upstream: el proyecto diverge y la compatibilidad deja de ser un criterio | vigente |
| D-152 | 16 oct | Proceso de trabajo | `FORK_CHANGES.md` se congela: sin sincronización, ya no hace falta | vigente |
| D-153 | 16 oct | Proceso de trabajo | Estado "a verificar" para lo construido que solo espera que el dueño lo vea en la app | vigente |
| D-154 | 16 oct | Producto | El producto mínimo no está cerrado: el wiki generado es inservible y la experiencia de uso no alcanza | vigente |
| D-155 | 16 oct | Interfaz | El contador de Atención cuenta problemas por línea y decisiones por decisión; lo de baja prioridad no cuenta | vigente |
| D-156 | 16 oct | Interfaz | Las fuentes de la vista de atención son piezas de la pantalla, con una forma común; el motor da los datos | vigente |
| D-157 | 16 oct | Interfaz | El panel de actividad del wiki queda reducido a una línea que lleva a Atención | vigente |
| D-158 | 16 oct | Interfaz | En qué clase y con qué prioridad va cada aviso que no fijaba la propuesta | vigente |
| D-159 | 16 oct | Base documental | La vista de parecidos separa los pares fuera de las familias; confirmar una familia une solo al principal con sus miembros | vigente |
| D-160 | 16 oct | Base documental | Las diferencias lado a lado se calculan en la pantalla, palabra por palabra, sin guardarse | vigente |
| D-161 | 16 oct | Base documental | Los tipos confirmados se guardan en la base y se suman a la lista cerrada solo en las preguntas nuevas | vigente |
| D-162 | 16 oct | Base documental | Rechazar un tipo propuesto deja sus documentos sin tipo y se vuelven a preguntar diciendo que no lo proponga | vigente |
| D-163 | 16 oct | Wiki | La revisión de enlaces que corre sola reemplaza los resultados estructurales y conserva los del modelo | reemplazada por D-396 |
| D-164 | 16 oct | Interfaz | Con otro proyecto abierto, los avisos del wiki del archivo se muestran sin botones | vigente |
| D-170 | 16 oct | Conocimiento | Una capa de conocimiento propia, dentro de la base del espacio, entre la base documental y el wiki (alternativa B) | vigente |
| D-171 | 16 oct | Conocimiento | Toda mención y afirmación del modelo trae su cita textual, y un cálculo comprueba que está en el texto; si no está, se descarta | vigente |
| D-172 | 16 oct | Conocimiento | Tipos iniciales persona, organización y asunto, con el rol de cada mención; tipos, roles y predicados son datos | vigente |
| D-173 | 16 oct | Conocimiento | Identidad en cascada con abstención; el mismo identificador une solo y se puede deshacer; uno distinto nunca une | vigente |
| D-174 | 16 oct | Conocimiento | Las decisiones de una persona sobre identidades son firmes, reversibles y no las pisa un recálculo | vigente |
| D-175 | 16 oct | Conocimiento | La página de una entidad es una proyección del registro; la incorporación del wiki deja de crear páginas de entidad | vigente |
| D-176 | 16 oct | Conocimiento | Sin derivaciones por reglas por ahora; todo incremental; costo de modelo aprobado | vigente |
| D-177 | 16 oct | IA | Las respuestas del modelo se guardan por pregunta y por motor y modelo; con un motor elegido solo cuentan las suyas | vigente |
| D-178 | 16 oct | Conocimiento | Qué cuenta como cita verificada: una tira continua de palabras del texto que contiene el nombre o el valor | vigente |
| D-179 | 16 oct | Conocimiento | La lectura de un documento espera a que se pregunte su tipo; los de menos de 10 palabras no se leen | vigente |
| D-180 | 16 oct | Conocimiento | Cambiar de modelo no vuelve a leer lo ya extraído; se relee si cambia el texto, la pregunta o el vocabulario | vigente; reemplazada en parte por D-193 |
| D-181 | 16 oct | Conocimiento | En segundo plano se pregunta sin pausa mientras hay trabajo: primero tipos, después la lectura de la capa | vigente |
| D-182 | 16 oct | Conocimiento | Los documentos largos se leen en partes de páginas enteras de hasta 4.000 palabras, y se guardan cuando todas tienen respuesta | vigente |
| D-183 | 16 oct | Conocimiento | Vocabulario inicial de la capa, y lo que se guarda en la etapa 1: evidencias entre menciones del documento | vigente |
| D-184 | 16 oct | Conocimiento | La evaluación de la extracción corre sobre una copia de la base de la app y compara por tipo y nombre normalizado | vigente |
| D-185 | 16 oct | Conocimiento | La unidad que se resuelve es el grupo de menciones de un documento, con una clave que no depende del orden de lectura | vigente |
| D-186 | 16 oct | Conocimiento | Cómo se comparan los nombres: sin artículos, títulos ni forma societaria; los candidatos salen de índices, no de comparar todos contra todos | vigente |
| D-187 | 16 oct | Conocimiento | El mismo identificador fuerte une solo, salvo nombres sin relación; uno distinto separa; un carácter de diferencia es dudoso | vigente |
| D-188 | 16 oct | Conocimiento | El modelo decide los candidatos difíciles contra una entidad principal; su veredicto se guarda por par; un nombre sin forma que comparten varias sociedades queda dudoso | vigente; reemplazada en parte por D-212 |
| D-189 | 16 oct | Conocimiento | Las decisiones de una persona (unir, separar, dato incorrecto, deshacer) se guardan sobre claves estables y se aplican al instante | vigente |
| D-190 | 16 oct | Conocimiento | El identificador de una entidad lo conserva la que se queda con más menciones; la que se vacía queda unida a otra o retirada | vigente |
| D-191 | 16 oct | Conocimiento | Afirmaciones: lo mismo dicho varias veces es una sola; un dato de un valor a la vez con valores distintos es un conflicto o una historia | vigente; reemplazada en parte por D-209 |
| D-192 | 16 oct | Conocimiento | La resolución se recalcula entera sin modelo, escribe solo lo que cambió y corre en el hilo de preguntas después de la lectura | vigente |
| D-193 | 16 oct | Conocimiento | Cambiar las palabras de la resolución no vuelve a leer el archivo: la lectura depende solo de lo que usa la pregunta | vigente |
| D-194 | 16 oct | Conocimiento | Cómo se evalúa la identidad contra la hoja: pares de menciones, uniones y separaciones falsas por separado | vigente |
| D-195 | 16 oct | Conocimiento | Proyección: nombre de archivo estable de cada página de entidad | vigente; reemplazada en parte por D-239 |
| D-196 | 16 oct | Conocimiento | Proyección: qué entidad tiene página (regla del vocabulario, sobre todo el archivo) | vigente; reemplazada en parte por D-213 |
| D-197 | 16 oct | Conocimiento | Proyección: la síntesis del modelo sale solo de afirmaciones numeradas y se comprueba | vigente |
| D-198 | 16 oct | Conocimiento | Proyección: el bloque de entidades al final de la página de cada documento | vigente |
| D-199 | 16 oct | Conocimiento | Coordinación: proyectar después de resolver, entera y escribiendo solo lo que cambió | vigente |
| D-200 | 16 oct | Conocimiento | Incorporación: sin páginas de entidad en el archivo e instrucciones de estudio jurídico | vigente |
| D-201 | 16 oct | Conocimiento | Índice y resumen: todas las páginas por tipo, en el idioma del wiki | vigente |
| D-202 | 16 oct | Conocimiento | Mantenimiento: la fusión de duplicados deja afuera las páginas proyectadas | reemplazada por D-220 |
| D-203 | 16 oct | Conocimiento | Proyección: la base escribe las páginas y conserva las notas de una persona | vigente |
| D-204 | 16 oct | Conocimiento | Cómo se evalúa la proyección: páginas, documentos listados y oraciones comprobadas | vigente |
| D-205 | 16 oct | Conocimiento | Un documento que se va se lleva lo suyo; las decisiones y los identificadores pasan al que toma su lugar | vigente |
| D-206 | 16 oct | Conocimiento | La página de una entidad que deja el registro se borra, salvo notas o enlaces de páginas que la app no escribe | vigente; reemplazada en parte por D-239 |
| D-207 | 16 oct | Conocimiento | Lo excluido no cuenta (su lectura se conserva); lo unido con otro elegido cuenta y la página nombra al elegido | vigente |
| D-208 | 16 oct | Conocimiento | Cómo se miden los cambios, los asuntos y los vínculos sobre la copia de la base | vigente |
| D-209 | 16 oct | Conocimiento | La forma de cada valor, el mismo valor escrito de dos maneras, cargo y monto reclamado como listas, domicilio constituido aparte | vigente |
| D-210 | 16 oct | Conocimiento | Un cargo es un cargo en una organización, tomada de la cita o de la relación del documento | vigente |
| D-211 | 16 oct | Conocimiento | Las palabras genéricas ("mi", "el cliente") no son nombres ni alias | vigente |
| D-212 | 16 oct | Conocimiento | Un nombre sin forma que comparten varias sociedades se pregunta una vez contra todas y une solo si el modelo nombra una | vigente |
| D-213 | 16 oct | Conocimiento | Un asunto es un grupo de documentos unidos por evidencia propia del asunto, con página propia | vigente |
| D-214 | 16 oct | Conocimiento | Los vínculos entre entidades se muestran de los dos lados, con su cita | vigente |
| D-215 | 16 oct | Conocimiento | Las versiones del esquema de la capa pasan a ser la 9, la 10 y la 11, después de la 8 de la vista de atención | vigente |
| D-216 | 16 oct | Conocimiento | Las identidades dudosas que comparten una entidad son una sola decisión | vigente |
| D-217 | 16 oct | Interfaz | Qué lleva la capa de conocimiento a la vista de atención, en qué clase y con qué prioridad | vigente |
| D-218 | 16 oct | Conocimiento | Sacar un documento de un asunto lo separa de cada uno de sus documentos; así se responde también una propuesta del modelo | vigente |
| D-219 | 16 oct | Interfaz | Pantallas de personas y organizaciones, de identidades dudosas y de asuntos; toda corrección es una decisión sobre el registro | vigente |
| D-220 | 16 oct | Conocimiento | La fusión de duplicados de Mantenimiento une o separa en el registro las páginas que se arman desde él | vigente |
| D-221 | 16 oct | Wiki | Enlaces dentro del texto de la página de cada documento, solo donde no pueden enlazar mal | reemplazada por D-396 |
| D-222 | 16 oct | Proceso de trabajo | La prueba de R-24 revisa todo el programa, sus pruebas y los textos de diseño | vigente |
| D-223 | 16 oct | Interfaz | Los nombres de tipos, roles, predicados e identificadores de la capa, en el vocabulario de cada vertical | vigente |
| D-224 | 16 oct | Conocimiento | Una decisión sobre una identidad guarda lo que la pantalla mostró de cada lado | vigente |
| D-244 | 17 oct | Conocimiento | El dinero es una relación entre un deudor y un acreedor, con control de dirección por la cita | vigente |
| D-245 | 17 oct | Conocimiento | Los términos definidos y los rótulos de formulario son alias locales, no nombres | vigente |
| D-246 | 17 oct | Conocimiento | Una pregunta de lectura nueva: se vuelve a leer el archivo entero | vigente |
| D-247 | 17 oct | Conocimiento | El domicilio tiene clase, tiempo y titular | vigente |
| D-248 | 17 oct | Conocimiento | Formularios: cada valor con su campo y su persona, y el rótulo aparte | vigente |
| D-249 | 17 oct | Conocimiento | Los identificadores por su rótulo; los números que no identifican a nadie | vigente |
| D-250 | 17 oct | Conocimiento | Lo que tiene consecuencias jurídicas nombra a su objeto en la cita | vigente |
| D-251 | 17 oct | Conocimiento | El rol "domicilio constituido" y los roles de una constancia de firma | vigente |
| D-252 | 17 oct | Conocimiento | Un conflicto es el mismo dato, de la misma clase, del mismo titular y del mismo tiempo | vigente |
| D-253 | 17 oct | Proceso de trabajo | La muestra de atribución: 40 afirmaciones al azar, juzgadas a ciegas | vigente |
| D-259 | 17 oct | Conocimiento | Una entidad se nombra por su forma más completa que sea nombre propio, no por la más repetida | vigente |
| D-260 | 17 oct | Conocimiento | Grupos de sociedades: mismo nombre, otra forma y otra señal que las vincule; lo no atribuible va al grupo | vigente; ampliada en D-473 |
| D-261 | 17 oct | Conocimiento | Dentro de un documento, la sociedad sin forma es la única forma completa que escribe el documento | vigente |
| D-262 | 17 oct | Conocimiento | Áreas y casillas generales no se unen por nombre: son contacto de la organización que indica el documento o de su grupo | vigente |
| D-263 | 17 oct | Conocimiento | Dos asuntos del mismo nombre entre otras partes no se le preguntan al modelo | vigente |
| D-264 | 17 oct | Conocimiento | Todos los nombres de un grupo de menciones cuentan para unir y comparar, no solo el principal | vigente |
| D-265 | 17 oct | Conocimiento | La pregunta de identidad trata la errata de una razón social entre las mismas partes (identidad-4) | vigente |
| D-266 | 17 oct | Conocimiento | Página de persona u organización: con un rol de sustancia o con algo que los documentos dicen de ella | vigente |
| D-267 | 17 oct | Conocimiento | El asunto se nombra por la relación entre sus partes, nunca por un comprobante, un número o un archivo | vigente |
| D-268 | 17 oct | Conocimiento | El número de un asunto en el texto de otro documento lo suma al asunto; el porqué, resumido y plegado | vigente |
| D-269 | 17 oct | Conocimiento | Una representación vale con la sociedad en la línea anterior y la cita que dice cómo actúa (P-122) | vigente |
| D-274 | 17 oct | Conocimiento | El nivel de razonamiento va a Codex (solo bajo, medio y alto) y es parte del nombre del motor | vigente |
| D-275 | 17 oct | Conocimiento | Un CUIT con prefijo de persona dado a una sociedad es de la persona (lectura-6) | vigente |
| D-276 | 17 oct | Conocimiento | Cómo se comparan modelos: cuatro variantes con su caché, la hoja, atribución a ciegas y tiempo | vigente |
| D-277 | 17 oct | Conocimiento | El mismo modelo cambia entre corridas: una corrida no alcanza para juzgar un cambio fino | vigente |
| D-278 | 17 oct | Conocimiento | gpt-6.1-sol queda como alternativa; alto se descarta por tiempo | vigente |
| D-279 | 17 oct | Conocimiento | Por defecto, gpt-6-luna con razonamiento alto | vigente |
| D-280 | 17 oct | Desktop | Una carpeta nueva tiene un solo id de proyecto y su cola nunca queda huérfana | vigente |
| D-281 | 17 oct | Conocimiento | Un nombre comercial con otra sociedad entre paréntesis no es esa sociedad | vigente |
| D-282 | 17 oct | Conocimiento | Una abreviatura sin su último punto cuenta por la forma con punto | vigente |
| D-283 | 17 oct | Conocimiento | Fechas en el idioma del wiki también con ordinales en inglés y con la forma de las escrituras | vigente |
| D-284 | 17 oct | Conocimiento | Los datos de contacto solos no le dan página a una entidad | vigente |
| D-285 | 17 oct | Conocimiento | El nombre propio de un asunto sin artículo inicial ni el nombre de una parte detrás de un guion | vigente |
| D-286 | 17 oct | Conocimiento | Más palabras genéricas y de áreas en el vocabulario | vigente |
| D-287 | 17 oct | Conocimiento | Un código postal o un DNI no es un teléfono | vigente |
| D-288 | 17 oct | Conocimiento | Ronda 1 de la revisión del wiki regenerado: valores que no lo son, constancias de firma, nombres y asuntos | vigente |
| D-289 | 17 oct | Conocimiento | Presentación de identificadores, alias y nombres de personas | vigente |
| D-290 | 17 oct | Producto | Chat, búsqueda y grafo se conservan, sin redundancias ni usos del modelo innecesarios; revisar los procesos heredados | vigente |
| D-291 | 17 oct | IA | Micelya Core ofrecerá los servicios de IA (también vectores); la app mantiene proveedores configurables y aísla su configuración | vigente |
| D-292 | 18 oct | Proceso de trabajo | La comprobación de los PR corre en todo PR, apunte a la rama que apunte; no apilar PR salvo que sea inevitable | vigente |
| D-293 | 18 oct | Wiki | Ajustes al rediseño: certeza y quién confirmó cada afirmación; el grafo ordena y no recorta; una extracción principal; página común solo para versiones; asunto sin mínimo de documentos | vigente; reemplazada en parte por D-373 |
| D-294 | 18 oct | Wiki | Decisiones abiertas resueltas: página del documento generada por ahora; asunto según P-061, no se construye aún; internet aparte y a pedido; vectores sin decidir | vigente; reemplazada en parte por D-373 |
| D-295 | 18 oct | Conocimiento | Conceptos por cálculo: por ahora solo medición de pasajes repetidos, fuera del repositorio | vigente |
| D-296 | 18 oct | Proceso de trabajo | Prioridad: atribución, importes y uniones falsas primero; en errores críticos hay que demostrar mejora con dos corridas comparables | vigente |
| D-300 | 18 oct | Proceso de trabajo | Casos de aceptación de los errores críticos, comprobables sobre cualquier base | vigente |
| D-301 | 18 oct | Wiki | Línea de base de la búsqueda con preguntas de un abogado | reemplazada por D-396 |
| D-310 | 18 oct | Capa de conocimiento | Cada importe lleva su estado, leído por el modelo y listado en el vocabulario | vigente |
| D-311 | 18 oct | Capa de conocimiento | Un importe que el documento da como ejemplo no se guarda como relación entre las partes | vigente |
| D-312 | 18 oct | Capa de conocimiento | Regla de fecha: un pago con fecha posterior al documento no está pagado | vigente |
| D-313 | 18 oct | Capa de conocimiento | Una afirmación de importe con estados distintos según el documento los muestra todos | vigente |
| D-314 | 18 oct | Base documental | Una casilla marcada se escribe con «(marcada)» y su pregunta por la primera oración | vigente |
| D-315 | 18 oct | Base documental | Las marcas de un formulario aplanado se leen por su lugar en la tabla, y si no se puede se dice | vigente |
| D-316 | 18 oct | Base documental | Versión de la extracción de texto: lo extraído con una versión vieja se revisa una vez | vigente |
| D-317 | 18 oct | Capa de conocimiento | Una definición con forma jurídica es un nombre, no un término; los términos se reconocen aunque tengan marcas de énfasis | vigente |
| D-318 | 18 oct | Proceso de trabajo | Medición de N2a: releer solo los documentos afectados reutilizando las respuestas de los demás | vigente |
| D-320 | 18 oct | Capa de conocimiento | Cada afirmación lleva su certeza: con cita comprobada, confirmada, dudosa o corregida | vigente |
| D-321 | 18 oct | Capa de conocimiento | Las decisiones de una persona se colocan sobre lo que dicen los documentos en cada resolución; lo que ya no encuentran queda para revisar | vigente |
| D-322 | 18 oct | Capa de conocimiento | La síntesis y las páginas escriben lo dudoso como dudoso; se dice «con cita comprobada» | vigente |
| D-323 | 18 oct | Proceso de trabajo | Medición de N2b: sin preguntar salvo las síntesis cuya pregunta cambió | vigente |
| D-330 | 18 oct | Capa de conocimiento | El resumen del documento sale de la lectura de la capa: tema y puntos, cada punto con su cita comprobada | vigente |
| D-331 | 18 oct | Wiki | La página de cada documento la escribe la base desde el resumen, sin modelo, hasta que N4 la arme como página del documento | reemplazada por D-396 |
| D-332 | 18 oct | Wiki | En el wiki del archivo la incorporación no llama al modelo: solo crea la página del documento para que la base la complete | reemplazada por D-396 |
| D-333 | 18 oct | Wiki | Se quitan la revisión del wiki con el modelo y el enriquecimiento de enlaces; la limpieza de avisos con el modelo no corre en el wiki del archivo | reemplazada por D-396 |
| D-334 | 18 oct | Wiki | La investigación en internet guarda su resultado fuera del wiki del archivo | reemplazada por D-397 |
| D-335 | 18 oct | Proceso de trabajo | Medición de N3: lectura completa desde la base de partida, sin trasplantes | vigente |
| D-340 | 18 oct | Wiki | Una página por documento o por familia de versiones, escrita por la base | reemplazada por D-396 |
| D-341 | 18 oct | Wiki | Cada punto, dato e importe de una página de versiones dice de qué versión sale | reemplazada por D-396 |
| D-342 | 18 oct | Wiki | La página de un documento solo tiene lo que sabe la base y las notas; las páginas viejas se absorben | reemplazada por D-396 |
| D-343 | 18 oct | Capa de conocimiento | El resumen de la lectura trae importes, fechas clave y números, y en una constancia de firma qué se firmó | vigente |
| D-344 | 18 oct | Base documental | La carta en Word y en PDF propuesta como «parecido» está bien: no se cambia la regla (P-210) | vigente |
| D-350 | 18 oct | Búsqueda | La búsqueda del wiki del archivo sale de la base: el texto recorre todo lo visible y el grafo ordena y amplía, nunca recorta | vigente |
| D-351 | 18 oct | Wiki | El grafo y los enlaces del wiki del archivo son las relaciones de la base | reemplazada por D-396 |
| D-352 | 18 oct | Búsqueda | El chat recibe una página por documento y la de la entidad más específica que nombra la pregunta | vigente |
| D-360 | 18 oct | Capa de conocimiento | Un término que el documento define nombra una sola cosa en él | vigente |
| D-361 | 18 oct | Búsqueda | Con un número en la pregunta, el resumen solo ordena los documentos que lo tienen | vigente |
| D-362 | 18 oct | Medición | La medición de referencia del 18 de octubre son dos lecturas completas desde cero | vigente |
| D-363 | 18 oct | Tipos de documento | Un documento que solo comparte la plantilla se tipa solo si el tipo del principal puede separar la familia | vigente |
| D-364 | 18 oct | Operación | Regeneración con los datos reales el 18 de octubre | vigente |
| D-365 | 18 oct | Identidad de entidades | Dos personas cuyo nombre de pila difiere en una letra se unen solo si el modelo está seguro | vigente |
| D-366 | 18 oct | Operación | La identidad corregida se reaplica en la app con los datos reales | vigente |
| D-370 | 18 oct | Conocimiento | Para medir conceptos, una «unidad» junta las versiones y copias de un documento | vigente |
| D-371 | 18 oct | Conocimiento | Qué es un pasaje repetido | vigente |
| D-372 | 18 oct | Conocimiento | Tres clases para un grupo de pasajes | vigente |
| D-373 | 20 oct | Asuntos | Un solo concepto de asunto | vigente |
| D-374 | 20 oct | Asuntos | Las referencias son datos del documento | vigente |
| D-375 | 20 oct | Asuntos | El asunto tiene estado desde ahora; todos abiertos | vigente |
| D-376 | 20 oct | Pantallas | Una sola vista por cosa | vigente |
| D-377 | 20 oct | Asuntos | Las referencias son las menciones de tipo asunto, sin tabla nueva; dejan de tener entidad al resolver otra vez | vigente |
| D-378 | 20 oct | Asuntos | Una referencia junta documentos por su forma normalizada, nunca entre otras partes | vigente |
| D-379 | 20 oct | Asuntos | Una afirmación sobre una referencia queda como valor escrito; la referencia sigue como contexto de las preguntas | vigente |
| D-381 | 20 oct | Asuntos | La agrupación cuenta familias: versiones y lo que es parte de un documento (constancia de firma, traducción, acuse) | vigente |
| D-382 | 20 oct | Asuntos | Confirmar o descartar se guarda en el asunto y sobrevive al recálculo | vigente |
| D-383 | 20 oct | Asuntos | El modelo propone documentos sueltos solo para un asunto confirmado o creado por una persona | vigente |
| D-384 | 20 oct | Asuntos | «Caso» en las pantallas de abogados; el wiki sigue con las palabras del catálogo | vigente |
| D-386 | 20 oct | Asuntos | Crear y editar un asunto a mano: lo que hace una persona se guarda por asunto y gana sobre la agrupación | vigente |
| D-389 | 20 oct | Pantallas | Página única de persona u organización: el artículo es su página del wiki tal como la escribió la proyección; los enlaces del wiki llevan a ella | vigente; reemplazada en parte por D-396 |
| D-395 | 21 oct | Producto | Se dejan de admitir los proyectos de wiki originales; la app abre en Inicio | vigente |
| D-396 | 21 oct | Wiki | Los archivos del wiki desaparecen: lo no repetido pasa a la base y la carpeta se borra tras migrar y respaldar | vigente |
| D-397 | 21 oct | Investigación | La investigación en internet queda, adaptada: se lanza desde una ficha y se guarda en la base | vigente |
| D-398 | 21 oct | Grafo | El grafo y su análisis se calculan sobre la base y suman los asuntos con su estado | vigente |
| D-399 | 21 oct | Wiki | Lint sale; queda pendiente una revisión con el modelo que agregue valor | vigente |
| D-400 | 21 oct | Producto | El servidor de API local y el recortador web (Clip) salen; se rehacen sobre la base más adelante | vigente |
| D-402 | 21 oct | Wiki | Las páginas se guardan en la base con su ruta lógica; la proyección no usa archivos ni el proyecto general | reemplazada por D-396 |
| D-403 | 21 oct | Wiki | Las notas y consultas guardadas del wiki entran una sola vez en la base; una nota sin dueño queda aparte | reemplazada por D-396 |
| D-404 | 21 oct | Interfaz | Lo que las pantallas leen de las páginas sale de comandos de la base | vigente |
| D-405 | 21 oct | Base documental | La lectura automática se rige por su propio ajuste, no por «Activar wiki» | vigente |
| D-407 | 21 oct | Interfaz | Las pantallas muestran el artículo y las notas desde la base; todo enlace abre una pantalla, nunca un archivo | vigente |
| D-408 | 21 oct | Interfaz | Una pantalla toma una sección del artículo por las palabras de la proyección; la búsqueda de la base no necesita proyecto | vigente |
| D-410 | 21 oct | Grafo | El grafo muestra los objetos de la base, con o sin página, y cada nodo abre su pantalla | vigente |
| D-411 | 21 oct | Grafo | Relevancia, comunidades y análisis salen de las relaciones de la base, con textos del vocabulario | vigente |
| D-412 | 21 oct | Interfaz | Los textos del grafo se escriben en español, fuera de los paquetes de traducción | vigente |
| D-413 | 21 oct | Chat | El chat lee y guarda todo en la base: buscar, leer un objeto, guardar una consulta, pasajes y relaciones | vigente |
| D-414 | 21 oct | Investigación | La investigación en internet se guarda en la base, de una persona, organización o asunto o suelta, con sus fuentes citadas | vigente |
| D-415 | 21 oct | Chat | El historial y las preferencias del chat viven en la carpeta de datos de la app, con una copia única desde el proyecto general | vigente |
| D-417 | 21 oct | Orígenes | «Lectura automática» reemplaza a «Activar wiki»: la base lee sola con el modelo configurado mientras está activa | vigente |
| D-418 | 21 oct | Chat | El agente del chat y los modelos de línea de comandos trabajan en la carpeta de datos de la app; el chat se guarda solo después de leído | vigente |
| D-419 | 21 oct | Ajustes | Todos los ajustes son globales; salen los que solo servían a la incorporación del wiki | vigente |
| D-420 | 21 oct | Reconocimiento | El motor que reconoce escaneos se conserva sin uso hasta que la base lo use por su cuenta | vigente |
| D-421 | 21 oct | Proceso de trabajo | Desktop y Core se comunican por escrito, con un buzón de cada lado (`docs/dropbox/`) | vigente |
| D-422 | 21 oct | IA | Cada servicio de IA se puede usar con distintos motores, configurables; Core es uno más | vigente |
| D-423 | 21 oct | Interfaz | Documento, persona u organización y caso con la misma pantalla de dos pestañas; las notas al pie de las dos | vigente |
| D-424 | 21 oct | Atención | Atención en tres pestañas; «En curso» muestra todas las etapas del procesamiento, hechas y por hacer, cada una desplegable con sus documentos | vigente |
| D-425 | 21 oct | Atención | Las nueve etapas de «En curso» salen del código y se cuentan de lo guardado; la pestaña cuenta las etapas sin terminar e Inicio, los documentos en proceso | vigente |
| D-426 | 21 oct | Interfaz | La línea del avance va fija al pie de la ventana, debajo del menú lateral, y dice «Todo al día» cuando no hay trabajo | vigente |
| D-427 | 21 oct | Atención | Lo que hace el motor y el último error del modelo se guardan en memoria; el error se dice en palabras en la etapa que preguntó | vigente |
| D-428 | 21 oct | Reconocimiento | El reconocimiento de escaneos en la base va antes del registro de motores, sobre el contrato `TextRecognizer` | en revisión |
| D-429 | 21 oct | Reconocimiento | El motor que reconoce escaneos es el modelo elegido en Ajustes cuando es Codex; la base lo guarda aparte del de las preguntas | vigente |
| D-430 | 21 oct | Reconocimiento | Los escaneos se reconocen antes que cualquier otra pregunta, de a un documento; lo pedido va primero; una falla del proveedor espera y una del documento lo marca | vigente; reemplazada en parte por D-452 |
| D-431 | 21 oct | Reconocimiento | El documento dice qué motor reconoció su texto y cuándo, y las páginas con partes «[ilegible]»; las marcas de cada página no se muestran | vigente |
| D-432 | 22 oct | Conocimiento | Un cambio de instrucciones no relee nada por su cuenta; se relee a pedido, con aviso de costo, y lo ya leído se controla de nuevo con las respuestas guardadas | vigente; ampliada en D-465 |
| D-433 | 22 oct | Conocimiento | La cita de una relación entre personas nombra a las dos; parentescos y herencia en el catálogo, guardados una sola vez | vigente |
| D-434 | 22 oct | Conocimiento | Una relación queda dudosa si sus citas nombran a otros y no a quien se le atribuye, o si el mismo documento dice un estado civil que no corresponde | vigente |
| D-435 | 22 oct | Conocimiento | «Representa a» necesita sus palabras donde lo dicen; un juzgado no representa ni es parte; heredar «en representación» no es representar | vigente |
| D-436 | 22 oct | Conocimiento | Un identificador fuerte escrito con más o menos caracteres no separa dos fichas | vigente |
| D-437 | 22 oct | Conocimiento | Un ítem de una lista se encuentra donde la lista lo tiene, con su nombre en singular o plural | vigente |
| D-438 | 22 oct | Parecidos | Cada familia de la misma plantilla tiene su plantilla guardada en la base (versión 21), también las propuestas | vigente |
| D-439 | 22 oct | Parecidos | La plantilla se arma alineando cada documento con el principal, sin modelo; sus campos se nombran por su clase y se ve desde el documento, la familia y Documentos | vigente; ampliada en D-466 |
| D-440 | 22 oct | Parecidos | Dos documentos de una plantilla en los que no cambia ningún dato son posiblemente el mismo documento | vigente |
| D-441 | 22 oct | Páginas | Las citas del artículo son notas numeradas que se tocan; «Referencias» y «Otros datos» salen del artículo y los documentos se listan una vez | vigente |
| D-442 | 22 oct | Páginas | La página de un documento se titula por su tipo, número, partes y fecha; sus fechas, en castellano; el nombre del archivo queda en los datos | vigente |
| D-443 | 22 oct | IA | Un registro de servicios de IA en la base dice qué motor usa cada servicio; la base lo lee en cada pedido y la configuración del chat y de la búsqueda son copias que el registro escribe | vigente |
| D-444 | 22 oct | IA | Las claves de API van al llavero del sistema; se mueven una vez al abrir, cada una recién después de confirmar que el llavero la guardó, y sin llavero quedan en el archivo | vigente |
| D-445 | 22 oct | IA | El catálogo de motores dice qué sabe hacer cada uno, dónde procesa y qué guarda su proveedor; el motor alternativo se guarda pero la base todavía no lo usa sola | vigente |
| D-446 | 21 oct | Portales | «Portales» en el menú con un navegador embebido que maneja la app; la ventana se habilita solo para el desafío o el inicio de sesión; novedades a pedido; cada expediente es un caso confirmado | vigente |
| D-447 | 22 oct | Portales | La ventana del portal es una vista hija sin ningún permiso de la app; la app la lee con guiones propios evaluados con retorno, y la bloquea con una capa que solo frena lo que hace una persona | vigente |
| D-448 | 22 oct | Portales | Cada documento se guarda con el nombre de la convención y sus datos al lado en `.micelya.json`; no se baja lo que la app ya bajó ni lo que la base ya tiene con ese nombre | vigente |
| D-449 | 22 oct | Portales | El recorrido pasa por todas las páginas y baja solo lo nuevo; se detiene y retoma ante el desafío; el expediente entra a la base como origen «Expedientes» y caso confirmado | vigente |
| D-450 | 21 oct | Carpeta de trabajo | La carpeta de trabajo de Micelya está en Ajustes, es un origen de la base y tiene «Recortes web» y «Expedientes»; los recortes web se guardan ahí como archivos markdown | vigente |
| D-451 | 22 oct | Carpeta de trabajo | La carpeta de trabajo es un solo origen: lo que la app guarda adentro nunca es un origen aparte, el origen «Expedientes» que hubiera se suma sin perder nada y, si la carpeta cambia, el origen se muda; el recortador escucha solo en la propia máquina | vigente |
| D-452 | 22 oct | Avance | «Casos» y «Páginas» esperan la primera vuelta del motor en vez de figurar hechas; los pedidos de reconocimiento a mano se guardan en la base | vigente |
| D-453 | 22 oct | Chat | El chat adjunta documentos de la base con un buscador o con «@»; el agente lee su texto de la base con su nombre y su lugar, y los cita | vigente |
| D-454 | 22 oct | Artículos | Las notas numeradas y los enlaces de los artículos muestran una tarjeta flotante al pasar el mouse o llegar con el teclado; tocar una nota la muestra y la deja, como antes; en pantalla táctil el primer toque de un enlace muestra su tarjeta y el segundo lo abre | vigente |
| D-455 | 22 oct | Artículos | Las vistas previas flotantes se apagan en Ajustes → Interfaz; vienen encendidas y el cambio se aplica al instante, sin el botón Guardar | vigente |
| D-456 | 22 oct | Conocimiento | Una persona nombrada por el papel que tiene en el documento («el causante», «mis mandantes») cuenta como nombrada; «causante» toma el asunto del documento | vigente |
| D-457 | 22 oct | Conocimiento | El hijo de una filiación es quien el texto nombra justo antes de «hijo de» | vigente |
| D-458 | 22 oct | Conocimiento | Fichas partidas: la medición cuenta la mejor coincidencia; una sigla con puntos es sigla; lo que el modelo dijo de un grupo releído sigue valiendo | vigente |
| D-459 | 22 oct | Conocimiento | La pregunta de lectura (`conocimiento-7`) pide cada filiación por separado, de cada persona, con una cita que nombra a los dos y no saltea palabras | vigente |
| D-460 | 22 oct | Conocimiento | Una filiación dada al revés se da vuelta; un padre sin hijo toma el de la filiación; una cita con puntos suspensivos se descarta con ese motivo | vigente |
| D-461 | 22 oct | Portales | La ventana del portal se crea una sola vez y se reutiliza: se muestra al entrar a «Portales» y se oculta, sin cerrarla, al salir o al recargar la app; queda siempre como lo último que pidió la página | vigente |
| D-462 | 22 oct | Forma de trabajo | Limpieza de pendientes: se cierra lo que ya no corresponde y cada pendiente queda en una caja (Claude, decide el dueño, ver en la app, en espera), con prioridad en lo de Claude y una propuesta en cada decisión; la sesión arranca con un resumen | vigente |
| D-463 | 22 oct | Producto | El producto mínimo se cierra con la revisión de la experiencia de uso con la marca, la conexión con Micelya Core, un documento generado desde una plantilla, acciones pedidas por el chat y un instalador de prueba para el estudio de referencia | vigente |
| D-464 | 22 oct | Forma de trabajo | Las decisiones que el agente tomó sin consultar quedan aprobadas en bloque; las del wiki retirado, reemplazadas por su retiro | vigente |
| D-465 | 22 oct | Conocimiento | Lo leído con otra versión de la pregunta o con otro modelo no se relee solo: Atención lo cuenta con un botón para releerlo, y siempre hay una opción para releer todo | vigente |
| D-466 | 22 oct | Plantillas | Una sección «Plantillas» para ver todas las plantillas con sus campos y, como parte del producto mínimo, generar un documento a partir de una | vigente |
| D-467 | 22 oct | Distribución | El instalador de prueba se genera cuando el dueño lo pida, para la prueba con el estudio de referencia; no se publica una versión descargable antes | vigente |
| D-468 | 22 oct | Servicios de IA | El estudio de referencia va a usar Micelya Core como proveedor de IA; qué usan los demás clientes se analiza después | vigente |
| D-469 | 22 oct | Interfaz | El marco visual con la marca de Micelya es parte del producto mínimo: primero una maqueta de tres pantallas para aprobar y después se aplica a toda la app | vigente |
| D-470 | 22 oct | Vocabulario | El vocabulario queda fijo en estudios jurídicos de Argentina y se valida con el estudio de referencia al entregarle el producto mínimo | vigente |
| D-471 | 22 oct | Conocimiento | Dos fichas que una persona separó se vuelven a proponer como la misma solo ante evidencia fuerte y nueva | vigente |
| D-472 | 22 oct | Asuntos | En un caso, la app propone quién es el cliente del estudio y quién la contraparte, y una persona lo confirma una vez por caso | vigente |
| D-473 | 22 oct | Conocimiento | La ficha de un grupo de sociedades se crea siempre que haya una señal firme, aunque no haya nada que atribuirle, y también se puede pedir por el chat | vigente |
| D-474 | 22 oct | Parecidos | Las cláusulas repetidas entre contratos se muestran en chico: dentro de los documentos de una misma plantilla | vigente |
| D-475 | 22 oct | Asuntos | Un caso se cierra o se archiva a mano y se puede reabrir; cerrado, sale de las listas y de Atención pero sigue en la búsqueda | vigente |
| D-476 | 22 oct | Conocimiento | Una revisión con el modelo busca contradicciones entre los documentos de una ficha o un caso, a pedido, y las lleva a Atención | vigente |
| D-477 | 22 oct | Conocimiento | Entran al catálogo los vínculos de expedientes que se usan seguido, medidos antes con el expediente de prueba nuevo | vigente |
| D-478 | 22 oct | Portales | Los términos de uso del portal judicial se revisan con el estudio de referencia al entregar el producto mínimo, y con ellos se prueba también el inicio de sesión del abogado | vigente |
| D-479 | 22 oct | Proceso de trabajo | La protección de main exige una sola comprobación con nombre fijo que siempre informa, y esa comprobación corre las pruebas en Linux | vigente |
| D-480 | 22 oct | Portales | El guion que bloquea la ventana del portal actúa solo en la página principal y, al bloquear, saca el foco | vigente |

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
- **Estado:** vigente, con una consulta abierta (P-029); reemplazada en parte por D-468 (el estudio de referencia usa Micelya Core)
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
- **Estado:** reemplazada por D-463
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** reemplazada por D-395 (antes reemplazó a D-061)
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
- **Estado:** reemplazada por D-395
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
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
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464); reemplazada en parte por D-143 (el texto del recuadro: el wiki ya aplica la elección; el criterio de sugerencia sigue valiendo)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464); reemplazada en parte por D-148 (un círculo de elecciones ya no se permite; el aviso y la regla de "algún par" siguen valiendo)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
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
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
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

### D-177 · El modelo forma parte de la clave de las respuestas guardadas

- **Fecha:** 16 oct 2026
- **Tema:** IA
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** cada respuesta del modelo se guardaba por la huella de la pregunta (su versión y su texto), sin el modelo que la dio. Si el dueño cambiaba de modelo, la app seguía usando las respuestas del anterior y nunca le preguntaba al nuevo. La etapa 1 de la capa de conocimiento pidió agregar el modelo a la clave sin perder lo ya guardado.
- **Qué se decidió:** la tabla de respuestas pasa a tener como clave la pregunta y el motor con su modelo (por ejemplo, "codex-cli:gpt-6-luna"). Las respuestas guardadas hasta ahora se conservan con su pregunta y su motor. Al preguntar, se usa la respuesta de ese mismo motor y modelo, o se pregunta. Cuando las pasadas sin modelo aplican respuestas ya guardadas (tipo, datos), usan las del motor elegido para el archivo; si todavía no se eligió ninguno, la más reciente de cualquier motor, como antes.
- **Por qué:** dos modelos pueden responder distinto la misma pregunta; cada respuesta es evidencia de quien la dio (R-20).
- **Alternativas descartadas:** meter el modelo en la huella de la pregunta (las pasadas sin modelo dejarían de encontrar las respuestas guardadas cuando no hay motor elegido); dejar las preguntas de tipo y datos como estaban y cambiar solo las de la capa nueva (un cambio de modelo no se notaría en el tipo).
- **Dónde se aplica:** `src-tauri/src/docbase/schema.rs` (V8); `similar/classify.rs` (`cached_answer`, `keep_answer`, `ask_and_keep`); pruebas `answers_kept_before_keep_their_question_and_engine` y `once_an_engine_is_chosen_only_its_answers_count`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K1 de la tanda desatendida del 16 oct.

### D-178 · Qué cuenta como una cita verificada

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** D-171 pide que un cálculo compruebe que cada cita del modelo está en el texto del documento, y descartar lo que no. Falta decir qué es "estar en el texto" y qué más se comprueba.
- **Qué se decidió:** la cita y el texto se comparan normalizados como en toda la base (minúsculas, sin acentos, solo letras y números); la cita tiene que aparecer como una tira continua de palabras en cualquier parte del texto, incluidos los campos de un formulario. Además, el nombre de una mención tiene que estar dentro de su cita, el valor de un identificador dentro de la suya y el valor de una afirmación dentro de la suya; si no, se descarta. El tiempo del hecho se conserva solo si está dentro de la cita (si no, se quita el tiempo y la afirmación queda). La página es la que dijo el modelo si la cita está en ella; si no, la primera donde aparece. Un rol fuera de la lista, o que no corresponde al tipo de entidad, pasa a "otro" sin descartar la mención; un tipo o un predicado fuera de la lista, una afirmación cuyo sujeto u objeto no es una mención conservada, o cuyo sujeto no es del tipo que pide el predicado, se descartan. Cada descarte se guarda con su motivo.
- **Por qué:** una cita que contiene el nombre ancla la mención al pasaje, no solo al documento; la normalización evita descartar por un salto de línea o una comilla distinta, y una cifra cambiada sigue siendo otra.
- **Alternativas descartadas:** una coincidencia aproximada (aceptaría citas con una cifra cambiada, justo lo que hay que evitar); comprobar solo que el nombre esté en algún lugar del texto.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/quote.rs` y `extract.rs` (`verify`); pruebas de `quote.rs` y `knowledge/tests.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K1 de la tanda desatendida del 16 oct.

### D-179 · La lectura espera al tipo del documento

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la pregunta de la capa lleva como ayuda el tipo y la etapa del documento y sus datos por tipo. Si se lee antes de que el tipo esté preguntado, la ayuda falta, y leerlo de nuevo cuando llega el tipo costaría otra pregunta.
- **Qué se decidió:** un documento cuyo tipo todavía está por preguntar espera; se cuenta como "esperando el tipo". En el trabajo de fondo las preguntas de tipo van primero, así que la espera es corta. Un cambio posterior del tipo no hace leer de nuevo el documento (la lectura depende del texto, de la pregunta y del vocabulario, D-180). Los documentos con menos de 10 palabras no se leen, como no se les pregunta el tipo.
- **Por qué:** una sola lectura por documento, con la mejor ayuda disponible.
- **Alternativas descartadas:** leer sin esperar; incluir el tipo en lo que decide volver a leer (cada corrección de tipo costaría una lectura).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/extract.rs` (`knowledge_pending`); `similar/classify.rs` (`contents_waiting_for_type`); prueba `a_document_waits_for_its_type_and_a_budget_leaves_the_rest`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K1 de la tanda desatendida del 16 oct.

### D-180 · Qué hace volver a leer un documento

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464); reemplazada en parte por D-193 (qué parte del vocabulario hace volver a leer)
- **El problema:** hay que decidir cuándo una lectura ya hecha deja de valer. Volver a leer todo el archivo cuesta una pregunta por documento.
- **Qué se decidió:** un documento se vuelve a leer cuando cambia su texto (su huella), la versión de la pregunta o la versión del vocabulario de la capa. Cambiar de motor o de modelo no lo hace volver a leer: la lectura queda con el modelo que la hizo anotado. Si una lectura se vuelve a hacer por otro motivo, se le pregunta al modelo nuevo (D-177). Cuando el texto se vuelve a escribir igual (el mismo escaneo leído otra vez), la lectura sigue valiendo. Saber si una lectura está al día no exige leer los textos: alcanza con cuándo se escribió el texto, la pregunta y el vocabulario.
- **Por qué:** releer el archivo entero es una decisión de costo que le toca al dueño, no un efecto lateral de elegir otro modelo.
- **Alternativas descartadas:** releer todo al cambiar de modelo.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/extract.rs` (`basis`, `standings`); prueba `another_model_asks_again_but_the_reading_stays_until_the_text_changes`; P-090.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K1 de la tanda desatendida del 16 oct.

### D-181 · El ritmo de las preguntas en segundo plano

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el trabajo de fondo hacía una pregunta al modelo cada 30 segundos. Con la capa de conocimiento cada documento suma al menos una pregunta, y a ese ritmo un archivo entero tarda demasiado; la etapa 1 pidió poder procesarlo en una tanda.
- **Qué se decidió:** un solo hilo de preguntas, el que ya existía, de a una pregunta por vez: primero las de tipo, etapa y datos; cuando no quedan, las de la capa. Mientras hay trabajo sigue sin pausa; cuando no hay nada que preguntar mira de nuevo a los 30 segundos; cuando el proveedor falla (por ejemplo, por el límite de uso) espera 10 minutos y reintenta. Sigue corriendo solo con el wiki del archivo activo y un motor elegido.
- **Por qué:** una pregunta a la vez no compite con la generación del wiki por el mismo proveedor, y sin pausas un archivo de prueba se procesa en una tanda.
- **Alternativas descartadas:** un hilo aparte para la capa (dos preguntas a la vez al mismo proveedor); preguntas en paralelo.
- **Dónde se aplica:** `src-tauri/src/docbase/worker.rs` (`spawn_asking`, `ask_step`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K1 de la tanda desatendida del 16 oct.

### D-182 · Cómo se lee un documento largo

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** algunos documentos (contratos largos) tienen decenas de miles de caracteres. Pasarlos enteros en una sola pregunta hace que el modelo pierda detalle; cortarlos en cualquier lugar parte una cláusula.
- **Qué se decidió:** un documento se lee en partes de páginas enteras de hasta 4.000 palabras; una página más larga que eso se corta por líneas. La lectura de un documento se guarda recién cuando todas sus partes tienen respuesta. La misma mención (mismo tipo, nombre y rol) en dos partes queda una sola, con los identificadores de las dos. "Es la misma que" solo vale dentro de una parte; unir menciones de partes distintas queda para la resolución de identidad.
- **Por qué:** páginas enteras respetan la ubicación de las citas; guardar todo junto evita una lectura a medias.
- **Alternativas descartadas:** el principio y el final del texto, como en la pregunta de tipo (para la capa se necesita todo el documento).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/extract.rs` (`parts`, `verify`); pruebas `a_long_text_is_read_in_parts_of_whole_pages` y `the_same_mention_in_two_parts_is_one`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K1 de la tanda desatendida del 16 oct.

### D-183 · El vocabulario inicial y lo que guarda la etapa 1

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** D-172 fija los tipos y pide roles y predicados como datos, pero no cuáles. Y en la etapa 1 todavía no hay entidades (las arma la resolución de identidad), así que hay que decidir a qué se refiere una afirmación recién extraída.
- **Qué se decidió:** roles: los de la hoja de respuestas de entidades (parte, apoderado, firmante, remitente, destinatario, emisor, domicilio, testigo, otro) más "referido", para el asunto del que trata un documento. Identificadores: DNI, CUIT o CUIL, matrícula, expediente y otro. Dieciséis predicados de un estudio jurídico: cargo, domicilio, representa a, socio de, empleado de, deudor de, acreedor de, monto reclamado, monto adeudado, parte en, inicio y fin de un vínculo, profesión, nacionalidad, estado civil y contacto. Los asuntos se extraen como entidades, con rol "referido". En la etapa 1 cada afirmación se guarda como **evidencia** entre menciones del mismo documento (sujeto y, si corresponde, objeto); la resolución de identidad las convierte en afirmaciones sobre entidades y agrupa las que dicen lo mismo. Las tablas de entidades, alias, afirmaciones y decisiones ya existen, vacías.
- **Por qué:** los roles de la hoja permiten medir sin traducir; las evidencias por mención no dependen de una identidad que todavía no se resolvió.
- **Alternativas descartadas:** predicados libres, como los de la hoja (no se pueden consolidar); crear una entidad por mención ya en la etapa 1.
- **Dónde se aplica:** `src-tauri/resources/catalog/knowledge.json`; tablas `knowledge_mention`, `knowledge_evidence`, `knowledge_claim` (`schema.rs`, V8); P-093.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K1 de la tanda desatendida del 16 oct.

### D-184 · Cómo se evalúa la extracción contra la hoja de entidades

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la extracción se mide contra la hoja de respuestas de entidades (R-17). El texto de los escaneos de la carpeta de prueba ya está reconocido en la base de la app, y volver a reconocerlo costaría cupo. Además, la hoja escribe los nombres como aparecen en cada documento y usa predicados libres.
- **Qué se decidió:** la prueba de evaluación (ignorada en la corrida normal) abre una copia de la base de la app, con su texto, tipos y datos, y la actualiza en una carpeta temporal; también puede leer carpetas, como las pruebas de parecidos. Una mención de la hoja se da por encontrada si el documento tiene una mención del mismo tipo con el mismo nombre normalizado, o con todas las palabras de uno dentro del otro (dos palabras al menos); se cuenta aparte la coincidencia exacta y el rol. Una afirmación de la hoja se compara con los predicados del vocabulario que dicen lo mismo, con una tabla de equivalencias en la prueba; los identificadores que la hoja da como afirmaciones se buscan entre los identificadores de la mención. Las respuestas del modelo se guardan entre corridas por motor y pregunta.
- **Por qué:** medir sin volver a reconocer, y sin castigar diferencias de forma que la resolución de identidad igual va a cubrir.
- **Alternativas descartadas:** exigir el nombre exacto (contaría como errores "Nombre Apellido" frente a "Apellido, Nombre").
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/check.rs`; prueba R-24 en `similar/check.rs`, que ahora revisa también el vocabulario de la capa contra la hoja de entidades.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K1 de la tanda desatendida del 16 oct.

### D-185 · La unidad que se resuelve: el grupo de menciones de un documento

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la extracción guarda una mención por cada forma de escribir a alguien y por cada papel que tiene en el documento, y el modelo dice qué menciones del mismo documento son la misma ("la Empresa" y la razón social). Para decidir quién es quién en todo el archivo hace falta una unidad, y una forma de referirse a ella que no cambie cuando el archivo se lee en otro orden o se vuelve a leer un documento sin cambios: las decisiones de una persona y las respuestas del modelo tienen que seguir apuntando a lo mismo.
- **Qué se decidió:** dentro de un documento, las menciones del mismo tipo con el mismo nombre normalizado, y las que el modelo dio como la misma, forman un **grupo del documento**, salvo que algo las separe (dos formas societarias, dos identificadores fuertes distintos). El grupo toma como nombre principal el más específico (un nombre antes que un número o una casilla; más palabras; con forma societaria). Su clave es `cluster:<documento>:<tipo>:<nombre normalizado>`, y se la encuentra por cualquiera de los nombres de sus menciones, así que sobrevive a un cambio del nombre principal. Todo se ordena por la huella del texto del documento y por el nombre, nunca por el orden en que se leyó. Las tablas nuevas de la etapa (`knowledge_doubt`, dudas para una persona; `knowledge_verdict`, lo que dijo el modelo de cada par) se agregaron a la misma versión 8 del esquema de la rama, porque ninguna base la aplicó todavía (al rebasar pasa a la versión siguiente, P-094).
- **Por qué:** resolver por mención multiplicaría las comparaciones y repetiría lo que el modelo ya dijo dentro del documento; una clave por texto y nombre hace que leer el archivo en otro orden dé las mismas entidades (R-13) y que una decisión no se pierda al releer.
- **Alternativas descartadas:** el número de fila de la mención como referencia (cambia al releer el documento); una versión 9 del esquema solo para estas tablas (complicaría el renumerado al rebasar).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/resolve.rs` (`clusters`, `cluster_key`, `names_index`); `schema.rs` (V8); prueba `the_order_of_reading_does_not_change_the_entities`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K2 de la tanda desatendida del 16 oct.

### D-186 · Cómo se comparan los nombres y cómo se encuentran los candidatos

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el mismo nombre se escribe de muchas formas (mayúsculas, acentos, "Apellido, Nombre", iniciales, "S.A." o "SA") y nombres muy parecidos son de entidades distintas (la misma razón social como sociedad local y como sociedad extranjera; dos personas con el mismo nombre de pila). Comparar todas las menciones contra todas no escala.
- **Qué se decidió:** un nombre se compara por sus palabras normalizadas, sin las palabras que no cuentan (artículos, "de"), sin los títulos de una persona ("Dr.", "Sr.", "escribano"), sin lo que está entre paréntesis (una sigla, la sociedad detrás de una marca) y, en una organización, con la forma societaria aparte. Esas palabras y las formas societarias son datos del vocabulario de la capa (`identity` en `knowledge.json`), no código (R-26). Resultados posibles: **igual** (personas: las mismas palabras en cualquier orden, dos al menos; organizaciones: las mismas palabras y la misma forma, o dos palabras al menos), **otra forma** (el mismo nombre con otra forma societaria: otra organización), **quizá** (un nombre parcial, una inicial, un error de tipeo de una o dos letras, una sigla, una organización sin su forma, la misma casilla de correo con nombres distintos, una casilla sin nombre que contiene el apellido) y **sin relación**. Un asunto nunca es igual por su nombre ("el acuerdo" suele ser otro acuerdo): a lo sumo es "quizá". Una persona nombrada con una sola palabra solo es candidata de la única entidad cuyo nombre tiene esa palabra. Los candidatos se buscan por índices: las palabras, las tres primeras letras de las palabras largas (un error más adelante), las iniciales (una sigla), la casilla de correo y los pedazos de una casilla sin nombre. Una clave compartida por más de 200 grupos no se usa (R-16).
- **Por qué:** con esto lo seguro se une sin preguntar y lo parecido llega al modelo con su motivo, sin comparar todos contra todos.
- **Alternativas descartadas:** una medida de parecido entre cadenas con un umbral (mezcla nombres de pila compartidos con errores de tipeo); unir por nombre parcial sin preguntar (un nombre y un apellido pueden ser de más de una persona).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/names.rs`; `src-tauri/resources/catalog/knowledge.json` (`identity`); pruebas de `names.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K2 de la tanda desatendida del 16 oct.

### D-187 · Lo que hacen los identificadores

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** D-173 dice que el mismo identificador une solo y que uno distinto no une nunca, pero el modelo a veces le atribuye un identificador a la mención equivocada (el CUIT de una sociedad a quien firma por ella), y los escaneos leen mal una cifra.
- **Qué se decidió:** son fuertes los identificadores marcados así en el vocabulario (DNI, CUIT o CUIL, matrícula, expediente; no "otro", donde caen números de factura o de cliente), de cinco caracteres o más, de un tipo de entidad al que corresponden. El mismo identificador fuerte une los grupos sin preguntar, salvo que los nombres no tengan ninguna relación o tengan otra forma societaria: entonces es **dudosa**. Uno distinto del mismo tipo los mantiene separados en todas las pasadas. Si difieren en un solo carácter y los nombres pueden ser el mismo, es **dudosa** (puede ser una cifra mal leída). Un identificador que el modelo dio a menciones de dos tipos (una persona y una organización) o a dos grupos del mismo documento no se usa: está mal atribuido. Si varias entidades tienen el mismo nombre y distintos identificadores, las que no tienen identificador no se unen a ninguna y quedan dudosas.
- **Por qué:** un identificador mal atribuido uniría a dos personas que firmaron por la misma sociedad; una unión falsa es mucho peor que una separación (R-07).
- **Alternativas descartadas:** unir siempre por el mismo identificador (las atribuciones erróneas producirían uniones falsas); usar los números de "otro" (comprobantes compartidos no son identidad, R-05).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/resolve.rs` (`clusters`, `identifiers`, `same_names`); `knowledge.json` (`strong`); prueba `identifiers_names_and_the_model_in_cascade`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K2 de la tanda desatendida del 16 oct.

### D-188 · Cómo decide el modelo los candidatos difíciles

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464); reemplazada en parte por D-212 (el nombre sin forma que comparten varias sociedades ahora se pregunta una vez contra todas)
- **El problema:** después de los identificadores y los nombres iguales quedan los parecidos (nombre parcial, inicial, error de tipeo, sigla, organización sin forma). Decidirlos con reglas une de más o de menos; preguntar por cada par cuesta y deja que el modelo una por cadena (A con B y B con C).
- **Qué se decidió:** se arma una pregunta por **entidad principal** (las más nombradas primero) y sus candidatos, hasta ocho por pregunta (R-21). Cada uno se describe con sus nombres, identificadores, lo que dicen los documentos, hasta cinco apariciones con su cita, y quién más aparece en esos documentos. El modelo responde por cada candidato "misma", "distinta" o "dudosa", con una certeza y un motivo. "misma" une (la certeza queda anotada con el motivo); "dudosa" queda como duda para una persona. Lo que dijo se guarda como **veredicto del par** (`knowledge_verdict`, con el motor) y vale en todas las resoluciones siguientes, aunque la descripción cambie: el mismo par no se pregunta dos veces (R-20). Se pregunta contra los grupos propios de la principal, y un candidato unido por el modelo no trae a sus propios candidatos (R-14). Un nombre de organización sin forma societaria que en el archivo tienen dos o más sociedades con distinta forma queda dudoso sin preguntar: puede ser cualquiera.
- **Por qué:** medido contra la hoja (R-17): en una corrida el modelo unió con motivos débiles un nombre sin forma con una de las sociedades que lo comparten (52 pares de uniones falsas); no preguntar ese caso las eliminó. Se probó también unir solo con certeza alta: los pares bien unidos bajaron de 92 % a 74 % (casi todos los "misma" con certeza media eran correctos), así que se descartó.
- **Alternativas descartadas:** preguntar por todos los pares (costo y cadenas); unir solo con certeza alta (separaciones falsas medidas); no usar el modelo para nombres (dejaría separado a quien firma con nombre completo y con nombre corto).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/resolve.rs` (`candidates`, `describe`, `question`, `resolve`); versión de la pregunta `identidad-2`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K2 de la tanda desatendida del 16 oct.

### D-189 · Las decisiones de una persona sobre identidades

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** D-174 pide decisiones firmes, reversibles y con la evidencia mostrada, pero no dice sobre qué se guardan: una entidad puede cambiar cuando entra un documento nuevo.
- **Qué se decidió:** cuatro acciones: **unir** dos entidades o menciones, **separar** (una mención de su entidad, o dos entidades que no son la misma), **marcar como incorrecto** (una mención que no es una entidad, una evidencia o una afirmación) y **deshacer** cualquiera de ellas (deshacer una unión automática es separar). Se guardan sobre las claves estables de los grupos (D-185), de la evidencia o de la afirmación, con la evidencia que se mostró y su huella; si la pantalla no la manda, se guardan los nombres de los dos lados. Cada resolución las lee de nuevo: un recálculo nunca las pisa (R-08, R-09). Una separación gana sobre todo lo demás, incluso sobre una unión de una persona; una unión de una persona gana sobre un identificador distinto. Al decidir, la resolución corre en el acto sin preguntarle nada al modelo. No se arma una duda sobre un par que una persona ya decidió. Solo se decide sobre dos entidades del mismo tipo.
- **Por qué:** claves estables hacen que una decisión siga valiendo cuando el archivo cambia; aplicarla en el acto muestra el efecto en la pantalla.
- **Alternativas descartadas:** guardar la decisión sobre el identificador de la entidad (cambia al unir o separar).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/decide.rs`; comandos `docbase_knowledge_decide`, `docbase_knowledge_undo`, `docbase_knowledge_decisions`; prueba `what_a_person_decides_is_firm_and_can_be_undone`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K2 de la tanda desatendida del 16 oct.

### D-190 · Qué entidad conserva su identificador

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** cada resolución arma de nuevo las entidades; si el identificador de una entidad cambiara cada vez, se romperían los enlaces a su página y las referencias de K3.
- **Qué se decidió:** cada entidad nueva toma el identificador de la entidad anterior que le aporta más menciones (las más grandes eligen primero). Una entidad nueva sin antecesora recibe un identificador derivado de su primer grupo (texto y nombre), no del orden de lectura. Una entidad anterior que no conserva ninguna mención queda **unida** a la que se llevó sus menciones (con ese destino) o **retirada** si sus documentos desaparecieron; nunca se borra, para que sus decisiones y enlaces sigan sirviendo. Cada entidad que cambió (menciones, nombre, estado o afirmaciones) queda marcada para que su página se proyecte de nuevo. El nombre visible es el alias más usado entre los nombres principales de sus grupos.
- **Por qué:** identificadores estables (D-170) sin depender del orden.
- **Alternativas descartadas:** un identificador al azar en cada resolución.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/resolve.rs` (`write`, `entity_id`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K2 de la tanda desatendida del 16 oct.

### D-191 · Cómo se consolidan las afirmaciones

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464); reemplazada en parte por D-209 (cargo y monto reclamado ya no son de un valor a la vez; la forma de los valores) y por D-252 (sin tiempo no hay conflicto; la clase y el tiempo del documento)
- **El problema:** la etapa 1 guarda evidencias entre menciones de un documento; las páginas necesitan afirmaciones sobre entidades, sin repetir lo mismo y mostrando las contradicciones (D-171).
- **Qué se decidió:** cada evidencia se pasa a términos de entidades: sujeto, predicado y objeto (otra entidad, o un valor comparado por sus palabras normalizadas). Las que dicen lo mismo son **una afirmación** con varias evidencias. Los predicados de un valor a la vez se marcan en el vocabulario (`single`: cargo, domicilio, monto reclamado, nacionalidad, estado civil); si una entidad tiene valores distintos de uno de ellos, son una **historia** cuando cada valor tiene su propio tiempo, y un **conflicto** cuando el tiempo es el mismo o no se dice: el conflicto se guarda y se muestra, nunca gana el último. Una afirmación marcada como incorrecta queda retirada; una evidencia marcada no cuenta. Una relación de una entidad consigo misma (después de unir) no se guarda. Se recalculan en cada resolución y se escribe solo lo que cambió.
- **Por qué:** es lo que pide D-171; marcar en el vocabulario qué predicados tienen un valor a la vez evita llamar conflicto a dos montos adeudados de facturas distintas.
- **Alternativas descartadas:** conflicto para cualquier predicado con dos valores (los montos y los vínculos de una entidad son muchos a la vez).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/claims.rs`; `knowledge.json` (`single`); pruebas `what_a_person_decides_is_firm_and_can_be_undone` y `claims_with_their_own_time_are_a_history`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K2 de la tanda desatendida del 16 oct.

### D-192 · Cuándo y cómo corre la resolución

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la resolución tiene que ser incremental (D-176) y no depender del orden (R-13). Resolver solo los documentos nuevos contra lo existente, de a uno, hace que el resultado dependa del orden en que llegan.
- **Qué se decidió:** cada vez que hay algo nuevo (una lectura escrita, un documento que se fue, una decisión), la resolución se calcula **entera, sin modelo**, desde las menciones, las decisiones y los veredictos guardados, y **escribe solo lo que cambió** (entidades, alias, la entidad de cada mención, dudas, afirmaciones), marcando las páginas a proyectar de nuevo. El modelo solo se pregunta por los pares sin veredicto. Corre en el hilo de preguntas en segundo plano, después de la lectura de la capa, de a una pregunta por paso y sin pausa mientras quede algo; las marcas de trabajo se borran solo cuando no quedó nada por preguntar. Sin nada nuevo, no calcula nada. También consume las marcas de documentos que se fueron (las entidades sin menciones quedan retiradas); la etapa 4 construye sobre esto.
- **Por qué:** calcular todo sin modelo es barato con índices (no compara todos contra todos) y garantiza que el resultado no depende del orden; lo costoso, el modelo, se pregunta una vez por par.
- **Alternativas descartadas:** resolver cada documento nuevo contra las entidades existentes (depende del orden); preguntar en paralelo al proveedor (D-181).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/resolve.rs` (`knowledge_resolve`); `src-tauri/src/docbase/worker.rs` (`ask_step`); P-097.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K2 de la tanda desatendida del 16 oct.

### D-193 · La lectura depende solo de lo que usa la pregunta

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** D-180 dice que cambiar el vocabulario de la capa vuelve a leer todo el archivo. La etapa 2 agregó al vocabulario palabras de la resolución (artículos, títulos, formas societarias) y marcas (identificadores fuertes, predicados de un valor a la vez) que la pregunta de lectura no usa.
- **Qué se decidió:** la huella que decide si una lectura sigue valiendo se calcula sobre lo que la pregunta usa: tipos, roles, identificadores y predicados con sus descripciones. Cambiar lo demás no relee el archivo. Reemplaza en parte a D-180 (la parte del vocabulario).
- **Por qué:** releer el archivo por un cambio que no cambia la pregunta gastaría cupo sin cambiar nada.
- **Alternativas descartadas:** un archivo de vocabulario aparte para la resolución (dos archivos para una misma capa).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/vocabulary.rs` (`reading_digest`); `extract.rs` (`basis`, `standings`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K2 de la tanda desatendida del 16 oct.

### D-194 · Cómo se evalúa la identidad contra la hoja de entidades

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la etapa 2 pide medir por separado las uniones falsas y las separaciones falsas (R-17), y los casos de la hoja que no se deben unir.
- **Qué se decidió:** la evaluación (prueba ignorada, sobre una copia de la base de la app) lee como en la etapa 1 y resuelve con el modelo real, preguntando primero en paralelo lo que haga falta. Cada mención leída se asigna a la entidad de la hoja que nombra en su documento (mismo tipo, mismo nombre o las palabras de uno dentro del otro); la que coincide con dos se deja afuera. Se cuentan los pares de menciones asignadas: de la misma entidad de la hoja y unidos (aciertos) o separados (separaciones falsas); de entidades distintas y unidos (uniones falsas); también solo entre documentos distintos. Se listan las entidades de la hoja partidas, las propias que mezclan dos de la hoja, los pares que la hoja da como distintos y quedaron unidos, el par dudoso y las menciones que no se pueden resolver. Se comprueba que resolver de nuevo sin cambios no cambie nada.
- **Por qué:** los pares miden las dos clases de error con el peso de cada entidad; las listas muestran dónde está el error.
- **Alternativas descartadas:** contar entidades exactas (una entidad grande partida en dos y una chica unida de más pesarían igual).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/check_identity.rs`; `desatendido-2\K2-correr-evaluacion.sh` (fuera del repositorio).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K2 de la tanda desatendida del 16 oct.

### D-195 · El nombre de archivo de una página de entidad no cambia

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464); reemplazada en parte por D-239 (el archivo sigue al nombre de la entidad)
- **El problema:** la página de una entidad tiene que tener un nombre de archivo que no se rompa: los enlaces del wiki apuntan al nombre del archivo, y el nombre visible de una entidad puede cambiar (otra forma de escribirla pasa a ser la más usada, o una persona la corrige).
- **Qué se decidió:** el archivo es `wiki/entities/<nombre legible>-<8 caracteres del identificador>.md`. El nombre legible sale del nombre visible la primera vez que se escribe la página (palabras sin acentos unidas por guiones) y queda guardado en `knowledge_page.path`; si la entidad cambia de nombre, cambia el título dentro de la página, no el archivo. Los enlaces se escriben `[[archivo|Nombre]]`. Si dos entidades darían el mismo archivo, la segunda lleva el identificador entero.
- **Por qué:** el identificador hace único al archivo y no depende del nombre; la parte legible deja reconocer el archivo en una carpeta. Renombrar el archivo con el nombre rompería los enlaces de las páginas que la app no escribe.
- **Alternativas descartadas:** solo el identificador (ilegible en una carpeta); el nombre solo (cambia y choca entre homónimos); renombrar el archivo y redirigir (más trabajo, enlaces rotos mientras tanto).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/pages.rs` (`page_path`, `slug`); esquema V9 (`knowledge_page`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K3 de la tanda desatendida del 16 oct.

### D-196 · Qué entidad tiene página se decide mirando el archivo entero

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464); reemplazada en parte por D-213 (los asuntos tienen página como grupos de documentos, no como entidades)
- **El problema:** el wiki anterior decidía documento por documento si una entidad "merecía página", y así 28 de 39 páginas tenían una sola fuente y faltaban otras. Además K2 dejó 104 asuntos, muchos de ellos comprobantes, y menciones que son correos o teléfonos.
- **Qué se decidió:** la regla es un dato del vocabulario de la capa (`pages.kinds` en `knowledge.json`). Persona u organización: aparece en dos documentos o más, o en alguno tiene un rol que no es "otro". Asunto: aparece en dos documentos o más y al menos uno trata de él (rol "referido"). Un nombre que es una dirección de correo, un teléfono o un número no es el nombre de una persona ni de una organización: si la entidad tiene otra forma de escribirse que sí lo es, esa da el título; si no, no tiene página.
- **Por qué:** mide todo el archivo (R-05: lo general antes que el documento suelto) y deja afuera lo nombrado al pasar. Medido contra la hoja (R-17): con "o un identificador fuerte" para los asuntos había 22 páginas de asunto, la mayoría facturas a las que el modelo dio número de expediente; con la regla actual quedan 11 y la hoja pierde un asunto de un solo documento. En total 91 páginas; 52 de las 84 entidades de la hoja tienen página.
- **Alternativas descartadas:** decidir por documento (lo de antes); una página para toda entidad (más de 250 páginas, muchas de comprobantes).
- **Dónde se aplica:** `src-tauri/resources/catalog/knowledge.json` (`pages`); `pages.rs` (`has_page`, `name_like`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K3 de la tanda desatendida del 16 oct.

### D-197 · La síntesis de una entidad sale solo de sus afirmaciones verificadas

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la página de una entidad tiene que decir lo que dicen sus documentos sin inventar nada y sin inferir sobre lo inferido (R-02).
- **Qué se decidió:** al modelo se le pasan solo las afirmaciones de la entidad (hasta 120), numeradas, cada una con su estado y hasta dos citas, y se le pide en JSON estricto una lista de oraciones con los números que las respaldan, en el idioma del wiki. Un cálculo descarta la oración que no nombra ninguna afirmación, nombra una que no existe o trae un número que no está en sus afirmaciones o citas. Debajo van, sin modelo, los datos en conflicto con todos sus valores y las referencias de cada número con sus citas y el documento. La respuesta se guarda como cualquier otra (`model_answer`, tarea `synthesis`): mientras las afirmaciones no cambian no se vuelve a preguntar. Sin modelo, la página se escribe igual con la síntesis pendiente.
- **Por qué:** cada oración queda atada a citas comprobadas; el control de números ataja el error más dañino (importes y fechas inventados). Corrida real: 179 oraciones guardadas, 5 descartadas (2,7 %), ninguna sin referencia.
- **Alternativas descartadas:** que el modelo lea los documentos enteros (repetiría la extracción y no se puede comprobar); escribir la síntesis sin modelo (una lista de datos, no un texto).
- **Dónde se aplica:** `pages.rs` (`synthesis_question`, `checked`, `render`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K3 de la tanda desatendida del 16 oct.

### D-198 · Las páginas de documentos enlazan a las entidades con un bloque que escribe la app

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la página de cada documento tiene que enlazar a las páginas de sus entidades, pero el wiki puede escribirla antes de que la capa lea y resuelva ese documento, y las entidades pueden unirse o cambiar después.
- **Qué se decidió:** la app escribe, sin modelo, un bloque al final de la página de cada documento, entre dos marcas (`<!-- micelya:entidades:inicio … -->` y `<!-- micelya:entidades:fin -->`), con cada entidad que el documento nombra, su tipo y su rol, enlazada si tiene página; lo nombrado al pasar y sin página no se lista. La proyección lo rehace cuando cambian las entidades del documento o cuando el wiki vuelve a escribir la página (lo nota por el tamaño y la fecha del archivo); si una fusión dejó dos bloques, deja uno. Al modelo de la incorporación se le pide que nombre a las personas, organizaciones y asuntos en texto, sin enlaces.
- **Por qué:** el registro sabe exactamente qué entidades salieron de cada documento; reescribir de forma determinista no depende del orden ni de que la entidad exista al incorporar, y se corrige solo cuando cambia la identidad.
- **Alternativas descartadas:** pasarle al modelo la lista de entidades (no existe todavía cuando se incorpora, y no se actualiza al unir o separar); enlazar dentro del texto (pendiente, P-102).
- **Dónde se aplica:** `pages.rs` (`source_block`, `with_block`); esquema V9 (`knowledge_source_block`); instrucciones en `src/lib/ingest.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K3 de la tanda desatendida del 16 oct.

### D-199 · La proyección corre cuando termina la resolución y no depende del orden

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** las páginas tienen que reflejar todo el archivo al final de una tanda, sin importar si el wiki incorporó un documento antes o después de que la capa lo leyera.
- **Qué se decidió:** el hilo de fondo que pregunta al modelo proyecta cuando no queda nada por leer ni resolver, de a una pregunta de síntesis por vez. Cada proyección se calcula entera desde el registro y las páginas de documentos del wiki y escribe solo los archivos que cambiaron (huella de lo escrito); no corre si no cambió ni el registro ni ninguna página de documento ni el vocabulario ni el idioma. Cuando escribe, avisa al wiki (evento `docbase://knowledge-pages`), que rehace su índice y su árbol. Comandos: `docbase_knowledge_project` y `docbase_set_wiki_language`.
- **Por qué:** calcular todo y escribir lo que cambió da el mismo resultado en cualquier orden (R-13) y, sin derivaciones, sigue siendo barato (R-15); proyectar a medias durante la resolución mostraría entidades que todavía pueden unirse.
- **Alternativas descartadas:** proyectar en la incorporación del wiki (depende del orden y del modelo del wiki); un grafo de dependencias por página (no hace falta sin derivaciones).
- **Dónde se aplica:** `src-tauri/src/docbase/worker.rs`; `commands.rs`; `src/lib/knowledge-layer.ts`; `src/App.tsx`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K3 de la tanda desatendida del 16 oct.

### D-200 · En el wiki del archivo la incorporación no escribe páginas de entidad

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** las páginas de entidad ahora salen del registro; si la incorporación siguiera escribiéndolas habría dos páginas de cada persona y la vieja seguiría incompleta. Además las instrucciones al modelo eran de investigación académica y permitían agregar conocimiento público.
- **Qué se decidió:** en el proyecto general la incorporación escribe la página de cada documento y los conceptos; se le dice que no escriba páginas de personas, organizaciones ni asuntos, y la app descarta cualquier bloque bajo `wiki/entities/` o de tipo `entity`. Las instrucciones de todos los proyectos pasan a ser las de un estudio jurídico: tipo, fecha y etapa del documento, partes y roles, identificadores, fechas, importes, plazos y obligaciones tal como están escritos, cada hecho con su parte, sin conocimiento externo ni opiniones; los ejemplos académicos se van. Los demás proyectos siguen escribiendo páginas de entidad, con las reglas nuevas. La incorporación le avisa a la base el idioma del wiki.
- **Por qué:** una sola fuente para cada página; el producto es para estudios jurídicos.
- **Alternativas descartadas:** mantener las páginas del wiki y sumarles lo del registro (dos verdades).
- **Dónde se aplica:** `src/lib/ingest.ts` (`IngestPromptOptions`, reglas `LAYER_ENTITY_*`); `src/lib/knowledge-layer.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K3 de la tanda desatendida del 16 oct.

### D-201 · El índice lista todas las páginas por tipo y el resumen del archivo sale del registro

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el índice guardaba solo las 200 páginas escritas más recientemente, con las secciones por tipo vacías, y el resumen general era la plantilla en inglés.
- **Qué se decidió:** el índice se rehace entero desde las páginas en disco cada vez que la incorporación o la capa escriben una página: una sección por tipo (documentos; personas, organizaciones y asuntos, con una subsección por clase; conceptos…) con sus títulos, en el idioma del wiki, con las palabras del vocabulario (`wikiIndex`). El resumen del archivo (`wiki/overview.md`) lo escribe la capa desde el registro: cuántos documentos, cuántas páginas por tipo, los más nombrados y los asuntos, con las palabras de su vocabulario.
- **Por qué:** un índice parcial hace creer que el wiki no tiene lo que tiene; contar desde el registro no necesita modelo y no se equivoca.
- **Alternativas descartadas:** que el modelo escriba el índice o el resumen (reescribe archivos grandes y se equivoca).
- **Dónde se aplica:** `src/lib/wiki-index.ts`; `src/vocabulary/`; `pages.rs` (`overview`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K3 de la tanda desatendida del 16 oct.

### D-202 · La fusión de duplicados no toca las páginas que proyecta la capa

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** reemplazada por D-220 (las páginas del registro vuelven a la búsqueda de duplicados y se unen o separan en el registro)
- **El problema:** Ajustes > Mantenimiento junta páginas duplicadas reescribiéndolas con el modelo y no se puede deshacer; sobre una página proyectada la siguiente proyección la volvería a escribir.
- **Qué se decidió:** las páginas que llevan `generated_by: micelya-conocimiento` no entran en la búsqueda de duplicados y no se pueden fusionar; dos entidades se unen con una decisión sobre el registro (comandos de K2, reversibles; las pantallas llegan en K6).
- **Por qué:** una unión en el registro se puede deshacer (R-09) y arrastra menciones, afirmaciones y enlaces; la fusión de archivos no.
- **Alternativas descartadas:** redirigir la fusión a una decisión del registro desde esa pantalla (queda para K6, P-102).
- **Dónde se aplica:** `src/lib/dedup-runner.ts` (`isProjectedPage`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K3 de la tanda desatendida del 16 oct.

### D-203 · La página de una entidad la escribe la base, con una sección de notas que nunca reescribe

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** las páginas tienen que poder rehacerse sin perder lo que una persona escribió, y quien las arma tiene que tener el registro a mano.
- **Qué se decidió:** la base (Rust) escribe las páginas en la carpeta del proyecto general. Cada página termina con "Notas" y una marca; lo que está debajo se copia tal cual al reescribir. Lo demás se reescribe: las correcciones se hacen como decisiones, no editando. Si una entidad deja de tener página y sus notas están vacías, la página se borra; si tienen algo, queda una página mínima que dice por qué (y con quién se unió) con las notas. Esquema V9: `knowledge_page` y `knowledge_source_block`.
- **Por qué:** la base tiene el registro, el modelo de la capa y el trabajo de fondo; nada de lo que escribe una persona se pierde (R-09).
- **Alternativas descartadas:** escribir las páginas desde el wiki (no tiene el registro ni corre sin la ventana abierta); guardar las notas en la base (se perderían al editar el archivo con otro programa).
- **Dónde se aplica:** `pages.rs` (`notes_of`, `with_notes`, páginas `kept`); `schema.rs` (V9).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K3 de la tanda desatendida del 16 oct.

### D-204 · Cómo se evalúa la proyección contra la hoja de entidades

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la etapa 3 pide medir que cada página tenga todos los documentos de su entidad y que cada frase remita a una cita.
- **Qué se decidió:** una prueba ignorada copia la base de la app, lee y resuelve con el caché del modelo, proyecta sobre un wiki temporal (con las páginas de documentos del wiki anterior copiadas, solo lectura) y cuenta: páginas por tipo, entidades de la hoja con página, documentos de cada entidad de la hoja que su página lista (por la entidad propia donde cayeron más de sus menciones), oraciones guardadas y descartadas, oraciones sin referencia, y que proyectar de nuevo no escribe nada. Guarda una muestra de páginas fuera del repositorio.
- **Por qué:** separa lo que mide esta etapa (la proyección) de lo que arrastra de las anteriores (lo que la extracción no vio o la identidad dejó partido).
- **Alternativas descartadas:** comparar páginas con texto esperado (la hoja no lo tiene).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/check_pages.rs`; `desatendido-2\K3-correr-evaluacion.sh` (fuera del repositorio).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K3 de la tanda desatendida del 16 oct.

### D-205 · Lo que se llevan los documentos que se van y lo que pasa al que toma su lugar

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el registro se recalcula entero desde las menciones, así que lo que dijo un documento que desaparece se va con él. Pero hay cosas que no deben perderse: lo que una persona decidió sobre ese documento (las decisiones lo nombran por su número interno de contenido) y el identificador de cada entidad, del que depende el nombre fijo de su página. Si un documento cambia de versión, la versión nueva es otro contenido con otro número: las decisiones quedaban apuntando a la vieja, y una entidad que solo aparecía en ese documento recibía otro identificador y otra página. Además, SQLite puede volver a usar el número de un contenido borrado, y una decisión vieja podía caer en un documento que no tiene nada que ver.
- **Qué se decidió:**
  - Cuando un documento desaparece (todas sus ubicaciones), se retiran sus menciones y evidencias; una afirmación con otra evidencia se conserva; una entidad sin menciones queda **retirada**, no borrada, con las decisiones sobre ella guardadas.
  - La base anota cada contenido que se va (con la huella de su texto), sus archivos y la entidad que tenía cada nombre del documento (esquema V10: `knowledge_gone`, `knowledge_gone_file`, `knowledge_gone_mention`).
  - El **documento que toma su lugar** es, en este orden: el que vuelve con el mismo texto (por la huella), el que vuelve con los mismos bytes (textos cortos, sin huella), o la versión nueva del archivo en la misma ruta (por el registro de cambios). Las decisiones de una persona sobre el documento viejo valen para ese (se traducen al leerlas; la decisión guardada no se toca), y cada nombre del documento nuevo conserva la entidad que tenía: una entidad que solo aparecía ahí conserva su identificador y su página.
  - Lo mismo cuando un documento se vuelve a leer (otro texto reconocido, otra versión de la pregunta): sus nombres conservan su entidad.
  - Un número de contenido no se vuelve a dar nunca.
  - Lo que dijo el modelo sobre los grupos de un contenido que se va se olvida (las respuestas quedan en `model_answer`: si la misma pregunta vuelve, no cuesta nada).
- **Por qué:** "nada se pierde" (R-09) y "lo que decide una persona es firme" (R-08) tienen que valer también cuando el archivo cambia; y el nombre de una página no debe cambiar porque se corrigió una coma del documento.
- **Alternativas descartadas:** cambiar la clave de las decisiones para que use la huella del texto (rompía las decisiones y veredictos ya guardados y no resolvía el caso de una versión nueva); reescribir las decisiones guardadas al cambiar el documento (se perdería a qué documento se refería la persona).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/changes.rs`; `resolve.rs` (`load_rows`, `load_decided`, `write`); `identity.rs` (número de contenido); `schema.rs` (V10). Pruebas en `tests_changes.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K4/K5 de la tanda desatendida del 16 oct.

### D-206 · La página de una entidad que deja el registro se quita sin romper enlaces

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464); reemplazada en parte por D-239 (una página sin notas no se conserva por un enlace)
- **El problema:** cuando una entidad se retira (sus documentos se fueron) o se une con otra, su página ya no tiene que estar; pero otra página puede enlazarla, y un enlace roto en el wiki confunde.
- **Qué se decidió:** las páginas que escribe la app (las de entidades y asuntos, el resumen y el bloque de entidades de cada página de documento) se rehacen sin esa entidad, así que sus enlaces no se rompen. La página se **borra** salvo que tenga notas de una persona o que la enlace una página que la app no escribe (por ejemplo, el texto que escribió el wiki en la página de un documento, fuera del bloque de la app). En esos casos queda una **página mínima** en el mismo lugar que dice que la entidad dejó el registro, o con quién se unió (con el enlace), y las notas. Cuando ese enlace desaparece y no hay notas, la página mínima también se va.
- **Por qué:** quitar lo que ya no corresponde sin perder nada que haya escrito una persona ni dejar enlaces muertos.
- **Alternativas descartadas:** dejar siempre una página mínima (llena el wiki de páginas vacías); borrar siempre (rompe los enlaces del texto del wiki).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/pages.rs` (`linked_from_outside`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K4/K5 de la tanda desatendida del 16 oct.

### D-207 · Un documento excluido no cuenta; uno unido con otro elegido cuenta, y la página nombra al elegido

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** hay dos formas en que un documento deja de alimentar el wiki sin desaparecer: una persona lo **excluyó** (el archivo o su carpeta), o lo **unió con otro** como el mismo documento y eligió el otro como fuente del wiki. Hay que decidir si lo que dice cuenta para el registro.
- **Qué se decidió:**
  - **Excluido** (todas sus ubicaciones excluidas): no cuenta. El diseño dice que la base no procesa lo excluido: sus menciones salen del registro como si se hubiera borrado (las entidades que solo estaban ahí quedan retiradas), pero su lectura se conserva, así que deshacer la exclusión no vuelve a preguntarle nada al modelo y todo vuelve como estaba (mismas entidades, mismas páginas). Un documento excluido que nunca se leyó no se lee.
  - **Unido con otro elegido:** cuenta como evidencia (su texto es real y las citas se verificaron en él), pero las páginas lo listan como el elegido: en la lista de documentos aparece una sola vez, el elegido; en las referencias, la cita se muestra con su propio nombre y una flecha al elegido. Los dos quedan en el mismo asunto.
  - Un cambio en lo excluido o en lo que una persona unió dispara la resolución y la proyección.
- **Por qué:** es la recomendación de la especificación para lo unido; para lo excluido manda el diseño de la base. Medido sobre el archivo real: excluir el documento que más entidades nombra retira exactamente las entidades que solo estaban ahí (26 de 26) y reescribe solo 3 páginas; devolverlo deja todo igual que antes sin preguntar nada (D-208).
- **Alternativas descartadas:** que lo excluido siga contando (una persona lo excluye justamente para que no se procese); que lo unido no cuente (se perderían citas verificadas que el elegido puede no tener).
- **Dónde se aplica:** `changes.rs` (`active_content`, `left_out`, `leave_out`, `notice_left_out`); `extract.rs` (`standings`); `pages.rs` (`passed_over`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K4/K5 de la tanda desatendida del 16 oct.

### D-208 · Cómo se miden los cambios, los asuntos y los vínculos contra el archivo real

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** las etapas 4 y 5 piden medir que borrar un documento retire lo suyo y nada más, que reprocesar sin cambios no cambie nada, y que los asuntos y los vínculos coincidan con la hoja de entidades.
- **Qué se decidió:** la prueba ignorada de la proyección (`check_pages.rs`) agrega tres mediciones sobre la copia de la base:
  - **Cambios:** toma el documento que más entidades nombra, lo excluye y mide cuántas entidades se retiran (tienen que ser exactamente las que solo estaban ahí), cuántas páginas se escriben y borran; después lo devuelve y comprueba que el estado de las entidades y el texto de todas las páginas sean iguales a los de antes, y que no se le haya preguntado nada al modelo.
  - **Asuntos:** pares de documentos que la hoja pone en el mismo asunto y que la capa agrupa (encontrados), y pares que la capa agrupa y la hoja no (agrupaciones falsas, el error peor), además de cuántos documentos de cada asunto de la hoja quedan juntos.
  - **Vínculos:** para cada afirmación de la hoja entre dos de sus entidades con un predicado del vocabulario, si el registro tiene esa afirmación entre las entidades a las que fueron a parar (en cualquier sentido: la hoja dice "representada por" donde el vocabulario dice "representa a").
  - Las pruebas con documentos inventados cubren los cuatro casos de la etapa 4 (borrado, vuelta, versión nueva, excluido, unido, reprocesar sin cambios) y los de la etapa 5.
- **Por qué:** cada pasada se mide por separado (R-17), y la hoja mide sin enseñar (R-25).
- **Alternativas descartadas:** borrar de verdad un documento de la copia (no se puede devolver sin volver a leer los archivos de la carpeta de prueba; la exclusión pasa por el mismo camino de retiro).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/check_pages.rs`; `desatendido-2\K45-correr-evaluacion.sh` (fuera del repositorio).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K4/K5 de la tanda desatendida del 16 oct.

### D-209 · Valores que no son valores, el mismo valor escrito de dos maneras y datos que no son de un valor a la vez

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464). Reemplaza en parte a D-191 (qué datos son de un valor a la vez).
- **El problema:** las páginas mostraban "los documentos difieren" donde no difieren (P-100). Medido sobre el archivo real había 54 afirmaciones en conflicto y casi todas eran falsas: un número de documento tomado como cargo, un plazo en horas como monto reclamado, un DNI como domicilio, el mismo domicilio escrito con y sin el piso, y cargos y montos que no son de un valor a la vez (una persona tiene varios cargos; un reclamo crece con las facturas).
- **Qué se decidió:**
  - Cada predicado dice en el vocabulario la **forma** de su valor: un importe tiene dígitos y una moneda (o decimales o miles, con pocas palabras alrededor); un texto (cargo, profesión, nacionalidad, estado civil) tiene más letras que dígitos y no es el nombre de quien se habla; un domicilio tiene un número y palabras; un contacto es una casilla de correo o un teléfono. Un valor sin su forma queda en la lectura (la cita es real) pero no es una afirmación. Las monedas y las palabras de un domicilio que no distinguen (piso, ciudad, provincia) son datos del vocabulario.
  - **El mismo valor escrito de dos maneras** es una sola afirmación, con toda su evidencia, mostrada con la escritura más completa: dos domicilios con los mismos números y una palabra en común (o dos números o más en común); dos importes con los mismos números.
  - **Cargo y monto reclamado** dejan de ser datos de un valor a la vez: se listan todos, cada uno con su documento. Domicilio, nacionalidad y estado civil siguen siéndolo.
  - Un **domicilio constituido** para un trámite (la cita dice "constituyo domicilio", "domicilio especial", "a estos efectos") es de otra clase que el real: no están en conflicto. Las palabras que lo dicen son datos del vocabulario (`qualifiers`).
- **Por qué:** medido contra el archivo real: de 54 conflictos quedaron 6, todos reales o atribuciones equivocadas del modelo que conviene ver (una dirección de una sociedad atribuida a otra, dos domicilios de una persona en países distintos). Se descartan 40 valores sin forma; los revisados son errores del modelo.
- **Alternativas descartadas:** pedirle al modelo que vuelva a leer el archivo con instrucciones más estrictas (en K1 una variante así empeoró roles y afirmaciones, D-184); resolver los conflictos eligiendo un valor (nunca gana el último, D-171).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/values.rs`, `claims.rs`; `resources/catalog/knowledge.json` (`shape`, `qualifiers`, `values`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K4/K5 de la tanda desatendida del 16 oct.

### D-210 · Un cargo es un cargo en una organización

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** una persona que es presidente de una sociedad y director de otra aparecía con "los documentos difieren" sobre su cargo (P-100): el cargo se guardaba como un valor suelto, sin la organización.
- **Qué se decidió:** un predicado puede decir en el vocabulario dentro de qué tipo de entidad se dice su valor (`within`; el cargo, dentro de una organización). La organización se toma del mismo documento, sin preguntarle al modelo: la que la cita nombra (si es una sola, o la única de ellas con la que la persona tiene una relación en el documento); si la cita no nombra ninguna, aquella con la que la persona tiene una relación en el documento (representa a, empleado de, socio de), si es una sola. Si no se puede saber, el cargo queda sin organización. El mismo cargo en dos organizaciones son dos afirmaciones, y los valores dentro de organizaciones distintas nunca están en conflicto. La página lo muestra como "presidente (en Ejemplo S.A.)". Esquema V10: `knowledge_claim.context_entity` y `qualifier`.
- **Por qué:** es lo que dice el documento, anclado en la cita (R-01), sin encadenar inferencias (R-02). Con la cita a la vista, una persona ve de dónde salió.
- **Alternativas descartadas:** pedirle al modelo que diga la organización de cada cargo (obligaba a volver a leer todo el archivo y a cambiar la pregunta de lectura; queda como mejora posible si la medición muestra muchos cargos sin organización, P-104).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/claims.rs` (`context_of`); `pages.rs` (`object_text`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K4/K5 de la tanda desatendida del 16 oct.

### D-211 · Las palabras genéricas no son nombres

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el modelo a veces da como mención "mi", "el locatario", "Señor cliente" o "Both Parties", unidas con el nombre verdadero dentro del documento. Eso producía alias espurios en las páginas (P-100) y, peor, un grupo cuyo nombre principal era "Señor cliente" no se unía con la sociedad que tenía el mismo CUIT, porque los nombres "no tenían relación".
- **Qué se decidió:** el vocabulario tiene una lista de palabras genéricas (pronombres, roles como cliente, locatario, empleador, "las partes"). Un nombre de persona u organización hecho solo de esas palabras (y de títulos) no es un nombre: no se compara, no es alias en las páginas y no le da nombre a una entidad; el grupo toma el nombre de otra de sus menciones. En los asuntos, "el acuerdo" sigue siendo una mención del asunto (se une solo por identificador o por el modelo), pero no se muestra como su nombre ni como alias.
- **Por qué:** medido contra la hoja: los pares de menciones de la misma entidad bien unidos subieron de 91,7 % a 92,5 % sin ninguna unión falsa nueva.
- **Dónde se aplica:** `names.rs` (`key`); `vocabulary.rs` (`is_generic`); `pages.rs` (alias); `knowledge.json` (`identity.generic`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K4/K5 de la tanda desatendida del 16 oct.

### D-212 · Un nombre sin forma societaria que comparten varias sociedades se pregunta una vez contra todas

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464). Reemplaza en parte a D-188 (el caso del nombre sin forma).
- **El problema:** la contraparte principal quedaba repartida en varias entidades. Una de las causas: los correos y cartas que la nombran sin su forma societaria quedaban como dudas sin preguntar, porque en el archivo hay varias sociedades con ese nombre y distinta forma (D-188). La decisión anterior se tomó porque preguntar contra cada sociedad por separado hizo que el modelo uniera con motivos débiles (52 pares de uniones falsas).
- **Qué se decidió:** se le hace **una sola pregunta** con todas las sociedades que comparten el nombre, diciéndole que son distintas entre sí y que a lo sumo una puede ser la misma; que diga "misma" solo si los documentos muestran cuál (el mismo identificador, domicilio, asunto o personas) y "dudosa" para todas si no. Se une solo si nombra exactamente una; si nombra más de una o ninguna, queda la duda contra cada una, como antes. Lo que responde se guarda como veredicto de cada par (R-20).
- **Por qué:** medido contra la hoja: los pares de menciones de la misma entidad bien unidos subieron de 92,5 % a 95,6 %, con las mismas 5 uniones falsas de antes (ninguna nueva); la contraparte principal pasó de 4 entidades a 2 y su página lista 30 de sus 34 documentos (antes 27).
- **Alternativas descartadas:** seguir sin preguntar (dejaba partida a la entidad más importante del archivo); preguntar contra cada sociedad por separado (las uniones falsas de D-188).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/resolve.rs` (paso 3b, `question` con `one_of`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K4/K5 de la tanda desatendida del 16 oct.

### D-213 · Un asunto es un grupo de documentos unidos por evidencia propia del asunto

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464). Reemplaza en parte a D-196 (los asuntos ya no tienen página como entidad).
- **El problema:** después de la etapa 3 los asuntos eran las menciones de asunto de cada documento ("el acuerdo", "la factura N", "carta documento X"): 104 entidades, casi todas de un solo documento, y 11 páginas (P-103). Un asunto de un estudio (un reclamo con sus cartas, facturas y correos; un contrato con sus pagos) es otra cosa: un conjunto de documentos que no siempre se nombran igual.
- **Qué se decidió:**
  - Un **asunto** es un grupo de documentos (`knowledge_matter`, `knowledge_matter_document`, esquema V10) que se arma en cada resolución con estos vínculos, de lo más seguro a lo menos (R-11): lo que decidió una persona ("son del mismo asunto" / "no lo son"; una separación no se cruza nunca); dos documentos que una persona unió con una relación de un mismo asunto (el mismo documento, versión, respuesta, acuse de recibo, certificado de firma; la lista es del vocabulario); el mismo asunto nombrado por los dos, ya resuelta la identidad; el mismo número de un asunto en los dos (en una mención de asunto o en un dato del tipo de documento que es un identificador, como las facturas que reclama una carta o el número de envío), comparado por sus dígitos; y el mismo importe con decimales o miles, que no sea redondo.
  - **No une:** compartir una persona o una organización (R-05); un número que identifica a una persona u organización (DNI, CUIT); un número o importe que está en más de la mitad de los documentos (R-16); un importe redondo.
  - Solo cuenta lo que confirmó una persona, nunca una propuesta de parecidos (R-03).
  - Cada asunto tiene un identificador estable (el del asunto que conserva más de sus documentos), un nombre (el asunto que más documentos nombran, con sus partes) y, por cada documento, los motivos por los que está en el grupo.
  - **Página:** un asunto con dos documentos o más tiene página: las partes y personas con sus roles, lo que dicen sus documentos de cada una (con sus citas), sus documentos y "por qué estos documentos van juntos". La escribe la app sin el modelo. Las entidades de tipo asunto ya no tienen página propia: su página es la del grupo. Las páginas de personas y organizaciones agrupan sus documentos por estos asuntos, y el bloque de entidades de cada página de documento nombra su asunto.
  - Las decisiones de una persona se guardan como las de identidad, con claves `content:<id>` que siguen al documento que toma el lugar de otro (D-205).
- **Por qué:** medido contra la hoja: de los 506 pares de documentos que la hoja pone en un mismo asunto, la capa agrupa 356 (70,4 %), con **0 agrupaciones falsas**; el asunto principal del archivo junta 26 de sus 31 documentos; otros tres asuntos quedan completos. Antes de excluir los importes redondos hubo una agrupación falsa (el mismo importe redondo en dos documentos sin relación), y se corrigió con esa regla general.
  - **Lo dudoso queda propuesto** (R-03): un documento que ningún dato específico pone en un asunto pero comparte dos partes o más con alguno se le pregunta al modelo, una vez por documento y con hasta tres asuntos, con la opción de no elegir ninguno. Si elige uno con certeza alta o media, queda **propuesto**: la página del asunto lo muestra en "Posiblemente de este asunto" con el motivo, la de cada entidad lo marca "(propuesto)", y nunca une a otros documentos ni cuenta como firme. Lo que dijo se guarda por documento y asunto (`knowledge_matter_verdict`) y no se vuelve a preguntar.
- **Alternativas descartadas:** unir las menciones de asunto por su número como identidad (mezclaba identidad con pertenencia, y una factura no es "la misma entidad" que el reclamo); familias contra un principal (R-14): una respuesta a una carta que responde a una factura es una cadena legítima de un asunto; el resguardo es que solo unen datos específicos y que se mide la agrupación falsa. Que lo propuesto por el modelo sea firme (lo propuesto no alimenta lo propuesto). Sobre el archivo real el modelo recibió 11 preguntas y no propuso ninguna: los once documentos eran, según la hoja, de otros asuntos.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/matters.rs` (`group`, `knowledge_propose_matters`); `resolve.rs` (`write`); `worker.rs` (las propuestas, en el hilo de preguntas después de la resolución); `decide.rs` (`SameMatter`, `OtherMatter`); `pages.rs` (`render_matter`, agrupación por asunto); `knowledge.json` (`matters`, regla de página de asunto `byMatter`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K4/K5 de la tanda desatendida del 16 oct.

### D-214 · Los vínculos entre entidades se muestran de los dos lados, con su cita

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** las relaciones entre entidades (quién representa a quién, quién es socio, empleado, deudor o acreedor de quién) ya eran afirmaciones con evidencia, pero solo aparecían en la síntesis de la entidad que es sujeto. La organización representada no veía a su representante, salvo como "relacionado".
- **Qué se decidió:** cada página de persona u organización tiene una sección "Vínculos" con las afirmaciones que la unen a otra entidad, en los dos sentidos: "Representa a X" en la página del representante y "Representada por Y" en la de la representada, cada una con su cita y su documento. El nombre del vínculo visto desde el otro lado es del vocabulario (`inverseLabel`). Solo lo que dicen los documentos, sin transitividad ni derivaciones (R-02). No se agregaron predicados nuevos ("apoderado de" es "representa a"; "contraparte en" queda para cuando el asunto tenga roles, P-106): agregarlos obliga a volver a leer todo el archivo.
- **Por qué:** medido contra la hoja: de 30 vínculos de la hoja entre dos de sus entidades, el registro tiene 21 (70 %) entre las mismas entidades; acreedor, deudor y contratista, todos; los que faltan son "integra el equipo de", "pertenece a" y "trabaja en", que la lectura no extrae como relación.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/pages.rs` (sección de vínculos); `knowledge.json` (`inverseLabel`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K4/K5 de la tanda desatendida del 16 oct.

### D-215 · Las versiones del esquema de la capa van después de la de la vista de atención

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la vista de atención y la capa de conocimiento se construyeron en ramas separadas y cada una agregó su versión 8 del esquema de la base. La base real del dueño ya está en la versión 8 de la vista de atención (las decisiones sobre tipos), porque la app de desarrollo corrió con esa rama. Si la capa conservara su 8, esa base nunca recibiría las tablas de la capa.
- **Qué se decidió:** al rebasar la capa sobre la vista de atención, la versión 8 queda para las decisiones sobre tipos y las de la capa pasan a ser la 9 (registro de entidades), la 10 (páginas) y la 11 (cambios, contexto de las afirmaciones y asuntos), en ese orden. Una prueba migra una base en la versión 8 con un tipo decidido y comprueba que llegan las tablas de la capa y que la decisión sigue.
- **Por qué:** nadie corrió la app con la capa sobre una base real, así que no existe una base con la versión 8 de la capa: alcanza con renumerar, sin migraciones de reparación.
- **Alternativas descartadas:** una versión que detecte cuál de las dos versiones 8 tiene una base (complica el esquema para un caso que no existe).
- **Dónde se aplica:** `src-tauri/src/docbase/schema.rs`; P-084 y P-094 (hechos).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K6 de la tanda desatendida del 16 oct.

### D-216 · Las identidades dudosas que comparten una entidad son una sola decisión

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la resolución deja para una persona pares de grupos de menciones que no supo si son la misma entidad. Contar cada par como una decisión infla el contador: una persona con tres posibles iguales serían tres decisiones, y se miran juntas.
- **Qué se decidió:** los pares que comparten una entidad, en cadena, forman un grupo (como una familia de parecidos), y cada grupo es una decisión en el contador y una tarjeta en la vista de identidades dudosas. Un par cuyos lados todavía no tienen entidad se agrupa por su clave. Dentro de la tarjeta se ven todos los lados con su evidencia y se decide par por par ("son la misma" o "no son la misma").
- **Por qué:** sigue el principio de la vista de atención (lo que se resuelve mirando junto es una decisión, D-155) y deja decidir de a pares, que es lo que la base sabe guardar.
- **Alternativas descartadas:** una decisión por par (infla el contador); una decisión por grupo que una todo de un clic (unir en cadena es inferir sobre lo inferido, R-02).
- **Dónde se aplica:** `src/components/knowledge/knowledge.ts` (`groupDoubts`); `src/components/knowledge/doubts-view.tsx`; `src/components/attention/sources/knowledge.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K6 de la tanda desatendida del 16 oct.

### D-217 · Qué lleva la capa de conocimiento a la vista de atención

- **Fecha:** 16 oct 2026
- **Tema:** Interfaz
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la especificación de la etapa pedía las identidades dudosas, los conflictos, las uniones recientes por identificador, las lecturas que fallaron, las que perdieron muchas citas y el avance, pero no la clase ni la prioridad de cada una, ni qué es "en gran proporción" o "reciente". Además quedaban de las etapas anteriores las propuestas de asunto del modelo, las páginas que se conservan por sus notas y las entidades que se van con documentos excluidos (P-107), y las oraciones descartadas de la síntesis (P-102).
- **Qué se decidió:** problemas, prioridad normal: lecturas que fallaron (con el motivo) y lecturas que perdieron muchas citas. Decisiones, prioridad normal: identidades dudosas (D-216), datos en que los documentos difieren (uno por entidad y dato) y el asunto que el modelo propone para un documento suelto (con "Confirmar" y "Descartar" en la línea). Prioridad baja, sin sumar al contador: uniones por identificador de nombres distintos traídas por un documento leído hace menos de 30 días, páginas que se conservan por sus notas o enlaces, entidades que dejaron el registro con documentos excluidos y oraciones descartadas de la síntesis. En curso: la lectura (con su avance), la resolución y las páginas (las que esperan su síntesis). "Muchas citas" es al menos 4 descartes cuya cita no está en el texto y más del 40 % de lo que dijo el modelo; esos números y los 30 días son datos del vocabulario de la capa (`attention` en `knowledge.json`), no del programa. El motor da los datos (`docbase_knowledge_attention`); los textos se arman en la pantalla con el vocabulario (D-156).
- **Por qué:** lo que bloquea saber quién es quién es un problema; lo que espera el criterio de una persona es una decisión; lo que casi siempre está bien o no se puede resolver de inmediato no debe mantener el contador encendido (D-155).
- **Alternativas descartadas:** contar las uniones por identificador como decisiones normales (son correctas casi siempre y encenderían el contador sin motivo); fijar los umbrales en el código.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/screens.rs` (`knowledge_attention`); `src/components/attention/sources/knowledge.ts`; `knowledge.json` (`attention`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K6 de la tanda desatendida del 16 oct.

### D-218 · Sacar un documento de un asunto, y responder una propuesta del modelo

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** las decisiones de una persona sobre asuntos son por par de documentos ("del mismo asunto", "de asuntos distintos", D-213). Para sacar un documento de un asunto no alcanza con separarlo de uno: puede volver a entrar por otro de sus documentos. Y una propuesta del modelo para un documento suelto quedaba a la vista para siempre, sin manera de contestarla.
- **Qué se decidió:** sacar un documento de un asunto guarda, de una vez y con una sola resolución, una separación entre ese documento y cada uno de los demás documentos del asunto. Descartar una propuesta del modelo hace lo mismo; confirmarla guarda "del mismo asunto" con el primer documento del asunto. Una propuesta deja de mostrarse cuando una persona separó el documento de alguno de los del asunto. Cada separación se puede deshacer desde el asunto.
- **Por qué:** usa las decisiones que la base ya sabe guardar y aplicar, sin un tipo nuevo de decisión ni una versión nueva del esquema; una separación no se cruza nunca, así que el documento queda afuera aunque comparta datos.
- **Alternativas descartadas:** una decisión nueva "este documento no es de este asunto" (el asunto se recalcula y cambia de identificador: la decisión quedaría colgada de algo que no es estable); separar solo del documento que lo unió (vuelve a entrar por otro).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/screens.rs` (`knowledge_leave_matter`, `knowledge_join_matter`); `decide.rs` (`knowledge_decide_many`); `matters.rs` (`proposals`); P-110.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K6 de la tanda desatendida del 16 oct.

### D-219 · Pantallas de la capa de conocimiento; toda corrección es una decisión

- **Fecha:** 16 oct 2026
- **Tema:** Interfaz
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la etapa pedía pantallas mínimas: la lista de entidades, su detalle con sus acciones, la vista de identidades dudosas, las entidades en el detalle de cada documento y, por P-107, los asuntos. Faltaba decidir dónde viven, cómo se llega y qué se puede corregir.
- **Qué se decidió:** dos pantallas nuevas en la navegación, con nombres del vocabulario: "Personas y organizaciones" (lista con buscador y filtro por tipo; detalle con forma de artículo: cómo se escribe, identificadores, documentos por asunto con su rol y cada mención, lo que dice cada documento con su cita, los datos en que difieren, los vínculos desde otras entidades, su página del wiki y las decisiones) y "Asuntos" (lista; detalle con sus documentos y el motivo de cada uno, sus partes, las propuestas del modelo y las decisiones). "Identidades dudosas" es una vista para resolver en bloque, a la que llevan la vista de atención, la lista de entidades y el detalle de una entidad con dudas. El detalle de un documento suma una sección con las entidades que nombra, su rol y su asunto. Las acciones son decisiones sobre el registro, con confirmación cuando cambian algo: unir con otra (buscándola por nombre entre las del mismo tipo), separar una mención, "no es una entidad", marcar un dato o lo que dice un documento como incorrecto, sacar un documento de un asunto y deshacer cualquiera. Nada se corrige editando el texto de una página (D-175).
- **Por qué:** el registro es la verdad y las páginas su proyección: corregir en la página se perdería en la próxima proyección; y las decisiones guardadas son firmes y reversibles (R-08, R-09).
- **Alternativas descartadas:** una sola pantalla para entidades y asuntos (los asuntos son grupos de documentos, no entidades con menciones); editar el nombre de una entidad a mano (el nombre sale de sus alias; queda para después si hace falta).
- **Dónde se aplica:** `src/components/knowledge/`; `src/components/docbase/document-detail.tsx`; navegación en `icon-sidebar.tsx`, `content-area.tsx`, `wiki-store.ts` y `attention/navigate.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K6 de la tanda desatendida del 16 oct.

### D-220 · La fusión de duplicados de Mantenimiento, sobre las páginas del registro

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la fusión de duplicados de Ajustes > Mantenimiento reescribe y borra archivos y no se puede deshacer. Desde la etapa 3 las páginas de entidad del archivo salen del registro, y se dejaron fuera de esa fusión (D-202); pero la herramienta sigue siendo la manera de encontrar páginas que parecen la misma, y dejarlas fuera la volvía inútil para el archivo.
- **Qué se decidió:** la búsqueda de duplicados vuelve a ofrecer las páginas de entidades del registro (no las de asuntos ni las que se conservan solo por sus notas). Fusionar dos o más páginas del registro guarda una decisión de unir sus entidades, con la de la página elegida como la que queda, en vez de reescribir archivos; decir que no son duplicados guarda una separación. Un grupo que mezcla una página del registro con una que escribió el wiki no se fusiona y se dice por qué.
- **Por qué:** convierte la herramienta en una fuente de decisiones firmes y reversibles (R-08, R-09) sin tocar archivos que la app vuelve a escribir.
- **Alternativas descartadas:** dejar las páginas del registro fuera (la herramienta no servía para el archivo); fusionar los archivos y después avisar al registro (la próxima proyección los reescribiría).
- **Dónde se aplica:** `src/lib/dedup-runner.ts` (`projectedEntityId`, `joinOnTheRecord`, `separateOnTheRecord`); `src/components/settings/sections/maintenance-section.tsx`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K6 de la tanda desatendida del 16 oct.

### D-221 · Enlaces dentro del texto de la página de cada documento

- **Fecha:** 16 oct 2026
- **Tema:** Wiki
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
- **El problema:** la página de cada documento la escribe el wiki y termina con un bloque, escrito por la app, con las entidades que nombra (D-198). Dentro del texto, los nombres no llevan a ninguna parte, y P-102 pedía enlazarlos si se podía hacer sin modelo y sin riesgo de enlazar mal.
- **Qué se decidió:** la app enlaza el primer lugar del texto donde aparece un nombre del documento escrito exactamente como lo escribe el documento, hacia la página de su entidad, una vez por entidad, solo si: la entidad tiene página; el nombre tiene dos palabras o más y no es genérico; ninguna otra entidad del mismo documento se escribe igual o con un nombre que lo contiene (una organización sin su forma societaria cuando otra la tiene); el lugar es texto corrido (no el encabezado, un título, código ni otro enlace) y el nombre está entero (no le sigue otra palabra con mayúscula). Cada enlace lleva una marca de la app (un comentario), así la proyección los saca y los vuelve a poner, y no cuentan como enlaces que hizo el wiki o una persona. El lector del wiki no muestra los comentarios de las páginas: hasta ahora las marcas del bloque se veían como texto.
- **Por qué:** un enlace equivocado manda a la persona a la página de otro, que es peor que no tener enlace; con estas reglas solo se enlaza lo que no puede ser otra entidad del documento.
- **Alternativas descartadas:** enlazar todos los alias de la entidad (un alias de otro documento puede ser otra persona en este); que el modelo marque los nombres al escribir la página (un uso más del modelo para algo que el cálculo resuelve en los casos seguros); no enlazar nada dentro del texto.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/links.rs`; `pages.rs` (bloque y enlaces de las páginas de documentos); `src/lib/wikilink-transform.ts` (`hideHtmlComments`); P-109.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K6 de la tanda desatendida del 16 oct.

### D-222 · La prueba de R-24 revisa todo el programa

- **Fecha:** 16 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la prueba que comprueba que ningún caso de las hojas de respuestas está en el programa (R-24) miraba solo la base documental, sus pantallas, el vocabulario y los catálogos. Por eso no veía nombres de la carpeta de prueba en ejemplos y pruebas anteriores a la regla (la búsqueda en español, la lectura del encabezado de las páginas, el puente con el wiki, las etiquetas del grafo y el ejemplo de etiqueta del diseño).
- **Qué se decidió:** la prueba lee todo el programa y sus pruebas, en Rust y en TypeScript, los recursos y los textos de diseño; no los registros de lo que pasó (`docs/sessions/`, este registro, los pendientes), que no se reescriben. Los nombres que encontró se reemplazaron por nombres inventados. Las palabras que el programa usa por motivos propios y que un caso comparte (el proveedor de búsqueda y el modelo con que habla la app, palabras comunes en inglés de pruebas traídas del proyecto original, una letra griega) quedan en una lista aparte de la prueba, dicha como tal; un nombre de pila en un texto de prueba del proyecto original se abrevió.
- **Por qué:** una regla que se comprueba sobre una parte del programa no se comprueba.
- **Alternativas descartadas:** seguir mirando solo la base documental (dejaba afuera justamente el código viejo).
- **Dónde se aplica:** `src-tauri/src/docbase/similar/check.rs` (`no_case_of_the_answer_sheet_is_in_the_program`, `NOT_OF_CASES`); los archivos corregidos están en el commit de la etapa.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K6 de la tanda desatendida del 16 oct.

### D-223 · Los nombres de la capa de conocimiento en el vocabulario de cada vertical

- **Fecha:** 16 oct 2026
- **Tema:** Interfaz
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el vocabulario de la capa (`knowledge.json`) trae identificadores y descripciones para el modelo, y etiquetas en el idioma del wiki para las páginas. Las pantallas necesitan los nombres para el usuario, por vertical y por idioma (P-091), y los términos del dominio no pueden ir escritos en un componente.
- **Qué se decidió:** el vocabulario de cada vertical (`src/vocabulary/`) suma una parte `knowledge`: el nombre de las pantallas ("Personas y organizaciones", "Asuntos", "Identidades dudosas"), el de un asunto y el de los documentos sin asunto, los tipos de entidad en singular y plural, los roles, los predicados leídos desde el sujeto y desde el objeto ("Representa a", "Representada por") y los tipos de identificador, en español y en inglés. Un identificador del vocabulario de la capa que la vertical no nombra se muestra con su identificador en palabras. El rol "otro" se muestra como "nombrado".
- **Por qué:** separa lo que lee el modelo (que cambia la lectura, D-193) de lo que lee una persona, que cambia con la profesión y el idioma.
- **Alternativas descartadas:** leer las etiquetas de `knowledge.json` desde las pantallas (están en el idioma del wiki, no en el de la pantalla, y son una por app, no por vertical).
- **Dónde se aplica:** `src/vocabulary/index.ts` y `legal.ts`; P-091.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K6 de la tanda desatendida del 16 oct.

### D-224 · Una decisión sobre una identidad guarda lo que se mostró de cada lado

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** R-08 pide guardar cada decisión con la evidencia que se le mostró a la persona. El motor, cuando la pantalla no dice nada, guarda solo los nombres de los dos lados (D-189).
- **Qué se decidió:** las pantallas mandan lo que mostraron: en la vista de identidades dudosas, el motivo de la duda y de cada lado su nombre, cómo se escribe, sus identificadores y sus documentos; al unir desde una entidad, los nombres y alias de las dos; al separar una mención, la mención y su documento; al marcar un dato, cuál. Se guarda como texto con su huella, como en los parecidos.
- **Por qué:** permite saber después qué vio la persona al decidir y, cuando haya evidencia nueva y distinta, volver a mostrar una separación (P-099).
- **Alternativas descartadas:** guardar solo los nombres (no alcanza para comparar con evidencia nueva).
- **Dónde se aplica:** `src/components/knowledge/doubts-view.tsx` y `entity-detail.tsx`; `src-tauri/src/docbase/knowledge/decide.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa K6 de la tanda desatendida del 16 oct.

### D-225 · Lo que no es un nombre no tiene página

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** en la primera generación del wiki sobre el archivo real, una de cada cinco páginas de entidad no era de nadie: domicilios y códigos postales leídos como organizaciones (con el rol "domicilio", que da página), pronombres y roles ("Uds.", "mi representada", "apoderado"), el hueco de una plantilla ("[Nombre / Razón social]") y, en una relectura, las direcciones web de las facturas.
- **Qué se decidió:** una forma de escribir que es solo de palabras genéricas, que tiene una palabra de dirección y un número, un número de calle o un código postal, corchetes de un formulario en blanco, una dirección de correo o una dirección web no es un nombre: la mención queda en el registro, pero no da página, no aparece en el bloque de entidades de la página de un documento, no se enlaza y no nombra a una entidad ni a un asunto. Las palabras genéricas y las de dirección son datos del vocabulario (`identity.generic`, `identity.addressWords`).
- **Por qué:** una página de una dirección o de un pronombre hace ruido y le quita confianza al wiki; borrar la mención perdería lo que el documento dice (R-09).
- **Alternativas descartadas:** pedirle al modelo que no las extraiga (no alcanza: lo hace igual en algunas lecturas); borrarlas del registro.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/vocabulary.rs` (`is_a_name`), `pages.rs`, `matters.rs`; `src-tauri/resources/catalog/knowledge.json`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa W de la tanda desatendida del 16 oct.

### D-226 · Una afirmación cuenta solo si su cita la dice

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la cita de cada afirmación se comprueba contra el texto, pero eso solo asegura que la cita existe, no que diga la afirmación. En el archivo real, cerca de un tercio de las afirmaciones tenía una cita que no la decía: "acreedor de" con una cita que no nombra a ninguno de los dos, una línea de percepciones de una factura tomada como deuda, la fecha de una factura tomada como inicio de un vínculo, un CUIT o un DNI tomado como contacto.
- **Qué se decidió:** al consolidar el registro, una relación entre dos entidades necesita en su cita alguno de los dos nombres; los predicados con palabras propias (deudor, acreedor, inicio y fin de un vínculo) necesitan una de ellas en la cita (`cues`); los importes no cuentan si la cita tiene palabras que dicen que es otra cosa (`notCues`: percepción, IVA, subtotal…); un contacto no puede contener un número que los documentos dan como identificador. Lo que no pasa queda en la lectura, contado como "no corresponde", y no llega a las páginas.
- **Por qué:** es un cálculo que verifica lo que dijo el modelo (R-19), con las palabras como datos del vocabulario (R-26). Medido sobre una muestra: las afirmaciones correctas pasaron de 69 % a 74 % con esta regla y a 84 % con D-230 y D-231.
- **Alternativas descartadas:** una segunda pregunta al modelo por cada afirmación (queda como P-112: es la forma de llegar más lejos, pero cuesta una pasada nueva).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/claims.rs` (`says`); `knowledge.json` (`cues`, `notCues` de los predicados).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa W de la tanda desatendida del 16 oct.

### D-227 · El registro y las páginas se rehacen cuando cambian las reglas del programa

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la resolución corría solo con marcas de trabajo pendiente, la verificación de una lectura solo cuando cambiaba la pregunta o el vocabulario, y la proyección solo cuando cambiaban el registro, el vocabulario o las páginas. Un arreglo del programa no llegaba a una base ya calculada.
- **Qué se decidió:** la resolución guarda con qué reglas se calculó (versión del programa, de la pregunta de identidad y del vocabulario) y se rehace si cambian; la verificación de lecturas tiene una versión que cuenta en lo que usa una lectura (se vuelve a verificar con las respuestas guardadas, sin preguntar); la proyección tiene una versión en su base.
- **Por qué:** R-09 (lo calculado se puede rehacer entero desde las fuentes) y R-13 (repetir sin cambios no cambia nada): las versiones solo cambian cuando cambia el programa.
- **Alternativas descartadas:** pedirle a la persona que "recalcule" desde una pantalla.
- **Dónde se aplica:** `resolve.rs` (`RESOLUTION_VERSION`, ajuste `knowledge-resolution`), `vocabulary.rs` (`READING_CHECK`), `pages.rs` (`PROJECTION_VERSION`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa W de la tanda desatendida del 16 oct.

### D-228 · Otro cargo u otra sociedad no separan a una persona

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la persona que más documentos nombran quedó en dos entidades: el modelo respondió que quien firma por una sociedad no es quien firma por otra del mismo grupo, aunque los dos lados tenían el mismo correo personal.
- **Qué se decidió:** la pregunta de identidad (versión `identidad-3`) dice que otro cargo u otra organización no alcanza para decir "distinta" y que el mismo correo personal (no una casilla general) es un dato fuerte; cada lado lista sus correos. Un veredicto dicho a otra versión de la pregunta no se usa: se pregunta de nuevo (R-20).
- **Por qué:** la regla de la pregunta llevaba al modelo a separar por el contexto, que en un grupo de sociedades siempre difiere.
- **Alternativas descartadas:** unir por correo sin preguntar (una lectura le dio a otra persona el correo de esta: habría unido a dos personas distintas).
- **Dónde se aplica:** `resolve.rs` (`question`, `describe`, `load_verdicts`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa W de la tanda desatendida del 16 oct.

### D-229 · Un veredicto que separó se vuelve a preguntar cuando los lados muestran otra cosa

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** los veredictos del modelo se guardan por par de grupos y se reusan para siempre. Un "dudosa" dicho cuando un lado mostraba poco seguía valiendo cuando lecturas nuevas le dieron el nombre completo, el DNI y el correo: la persona principal volvió a quedar en dos.
- **Qué se decidió:** cada veredicto guarda la huella de lo que se mostró de los dos lados (esquema V12, `knowledge_verdict.evidence`). Uno que los separó ("distinta" o "dudosa") se pregunta de nuevo cuando esa huella cambió; uno que los unió se mantiene. Los veredictos anteriores no tienen huella: los que separaron se preguntan una vez más.
- **Por qué:** es la misma regla que R-08 pide para las decisiones de una persona: una separación se revisa ante evidencia nueva y distinta.
- **Alternativas descartadas:** volver a preguntar todo cuando cambia algo (más preguntas y más cambios de opinión del modelo sin evidencia nueva); no volver a preguntar nunca (lo de antes).
- **Dónde se aplica:** `src-tauri/src/docbase/schema.rs` (V12), `resolve.rs`. Al rebasar, la V12 de esta rama va después de cualquier versión nueva de `main`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa W de la tanda desatendida del 16 oct.

### D-230 · La pregunta de lectura dice quién es el sujeto de una deuda, una representación y una fecha

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la lectura atribuía la deuda a quien la reclama ("Ud. adeuda" dicho del remitente), daba vuelta las representaciones ("la sociedad, representada por X" como "la sociedad representa a X"), tomaba la fecha de firma como inicio de un vínculo, le daba el correo de una persona a otra y tomaba un número de comprobante por un asunto.
- **Qué se decidió:** las descripciones de esos predicados en el vocabulario (que la pregunta muestra) dicen quién es el sujeto y qué no es el valor. Todos los documentos se leen de nuevo (75 preguntas).
- **Por qué:** es un cambio de datos, no de programa (R-26). Las representaciones invertidas desaparecieron de la muestra; las deudas atribuidas al acreedor bajaron, pero no del todo (P-112).
- **Alternativas descartadas:** corregir la dirección con reglas en el programa (no hay una señal general confiable).
- **Dónde se aplica:** `src-tauri/resources/catalog/knowledge.json` (`representa_a`, `deudor_de`, `acreedor_de`, `monto_reclamado`, `monto_adeudado`, `parte_en`, `inicio_de_vinculo`, `fin_de_vinculo`, `contacto`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa W de la tanda desatendida del 16 oct.

### D-231 · Los números que no identifican a la mención no la separan

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** una relectura dio como identificadores de una sociedad su número de oficina y su código postal en unos documentos y su número de registro en otros: los números distintos dejaron a cada mención con el mismo nombre como duda y la sociedad quedó en veinte entidades. En una ficha de cliente, el DNI del cónyuge, escrito debajo del cliente, se leyó como del cliente y lo separó de todos sus documentos. Un documento dio el DNI de la persona como "otro número".
- **Qué se decidió:** al verificar una lectura, un identificador cuyo valor está dentro de un domicilio que da la misma lectura, o que sigue a una palabra de dirección en su cita, se descarta ("parte-de-un-domicilio"); el número de una persona que en todo el texto aparece justo después del nombre de otra persona de la lectura, y no del suyo, se descarta ("de-otra-persona"). En la resolución, un número de seis dígitos o más que otros documentos dan como identificador fuerte de una sola clase, para el mismo tipo de entidad, cuenta como ese identificador.
- **Por qué:** la regla "identificador distinto nunca se une" (aprobada por el dueño) hace que un solo número mal atribuido separe a una entidad entera; hay que cuidar qué es un identificador antes de aplicarla.
- **Alternativas descartadas:** relajar la regla de identificadores distintos (aumenta las uniones falsas).
- **Dónde se aplica:** `extract.rs` (`verify`), `resolve.rs` (`clusters`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa W de la tanda desatendida del 16 oct.

### D-232 · Solo espera una síntesis la página que tiene afirmaciones

- **Fecha:** 16 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la vista Atención mostraba "Escribiendo lo que dicen los documentos en 51 páginas" para siempre: contaba las páginas sin síntesis, y las de entidades sin afirmaciones y las de asuntos no tienen síntesis nunca.
- **Qué se decidió:** cuenta solo las páginas cuya entidad tiene afirmaciones vigentes y todavía no tienen síntesis.
- **Por qué:** un aviso "en curso" que no termina nunca enseña a ignorar la vista.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/screens.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa W de la tanda desatendida del 16 oct.

### D-233 · El wiki anterior se apartó copiándolo y moviendo el original fuera de la carpeta de trabajo

- **Fecha:** 16 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la etapa pedía mover el wiki del 14 de octubre a `wiki-descartado-2026-10-16`; la herramienta del agente bloqueó el movimiento directo.
- **Qué se decidió:** se copió a `LocalRepo\llm_wiki_mi\wiki-descartado-2026-10-16\`, se comprobó que la copia es idéntica (474 archivos) y recién entonces se movió el original a `desatendido-2\wiki-original-movido-2026-10-16\`. Quedan dos copias iguales; ninguna se borró (D-093).
- **Por qué:** no perder la línea de base aunque falle un paso.
- **Dónde se aplica:** carpetas de la tanda (fuera del repositorio).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa W de la tanda desatendida del 16 oct.

### D-234 · Un `$` suelto es siempre el signo de moneda en los lectores

- **Fecha:** 17 oct 2026
- **Tema:** Wiki
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** los lectores del wiki, de la vista previa, del chat y del panel de investigación dibujaban como fórmula todo lo que quedaba entre dos `$` de una misma línea. En un archivo jurídico eso son justo los importes ("reclama $ 1.500 y $ 300 de intereses"): el texto del medio perdía los espacios y las palabras aparecían pegadas. La revisión del 17 de octubre contó 39 casos en 21 páginas.
- **Qué se decidió:** en los cuatro lectores solo `$$…$$` es una fórmula; el `$` simple no abre fórmula nunca (opción `singleDollarTextMath: false`, en un único lugar compartido, `src/lib/markdown-math.ts`). El chat dejó de tratar `$…$` como fórmula al convertir LaTeX a texto.
- **Por qué:** arreglarlo en el lector cubre de una vez todas las páginas: las que proyecta la app, las que escribe el modelo del wiki (que no controla el programa) y las notas que escribe una persona, también las ya escritas. Escapar al escribir habría dejado afuera las páginas del modelo y las notas, y obligaría a volver a escribir todo el wiki. En un estudio jurídico la fórmula en línea con `$` simple no se usa; quien la necesite tiene `$$`.
- **Alternativas descartadas:** escapar `$` al escribir las páginas (no cubre lo que escribe el modelo ni lo que escribe una persona).
- **Dónde se aplica:** `src/lib/markdown-math.ts`, `wiki-reader.tsx`, `file-preview.tsx`, `chat-message.tsx`, `research-panel.tsx`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q1 de la tanda desatendida del 17 oct.

### D-235 · Una lectura vacía de un texto largo es una lectura fallida

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** tres documentos largos (entre 1.300 y 3.800 palabras, uno de ellos la cronología del reclamo) volvieron del modelo sin ninguna mención ni afirmación y quedaron como leídos ("done"). Nadie se enteró: sus personas y sus datos faltan en el registro y en las páginas, y la respuesta vacía quedó guardada, así que nunca se volvían a leer.
- **Qué se decidió:** una parte de un documento con al menos 300 palabras (`attention.emptyMinWords` del vocabulario de la capa) que el modelo lee sin nombrar a nadie ni decir nada no cuenta como lectura. Se vuelve a preguntar en la misma tanda, hasta 3 veces (`attention.emptyAttempts`), sin mirar la respuesta guardada. Una respuesta vacía no se guarda, y una vacía que ya estaba guardada de antes no cuenta. Si después de los intentos sigue vacía, la lectura queda **fallida** con el motivo ("el modelo leyó el texto sin encontrar a nadie, aunque es largo; se preguntó 3 veces") y aparece en Atención como problema, con una acción "Volver a leer" que olvida las lecturas fallidas para que la próxima tanda las vuelva a pedir (lo que el modelo contestó bien de las otras partes se conserva y no se pregunta de nuevo). La versión de la comprobación de lectura pasó a `lectura-3`: las lecturas se vuelven a comprobar desde las respuestas guardadas, sin preguntar, salvo las partes que habían vuelto vacías.
- **Por qué:** en un estudio jurídico un documento largo sin nadie nombrado es casi siempre una falla del modelo, no un documento vacío; callarlo es una pérdida silenciosa (R-23: lo que falta es un resultado). Reintentar con un tope evita tanto la pérdida como un bucle de preguntas. Un documento corto sin nombres (una receta, una constancia) sí puede ser vacío y sigue valiendo.
- **Alternativas descartadas:** reintentar en cada arranque sin tope (preguntas sin fin si el documento de verdad no nombra a nadie); guardar las otras partes como leídas y marcar solo la parte vacía (la lectura parecería completa y no lo es).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/extract.rs` (`knowledge_run`, `knowledge_read_again_failed`), `vocabulary.rs`, `resources/catalog/knowledge.json` (`attention`), comando `docbase_knowledge_read_again_failed`, `src/components/attention/sources/knowledge.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q1 de la tanda desatendida del 17 oct.

### D-236 · Lo que la síntesis no usa se lista igual en la página

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la página de una entidad mostraba solo las afirmaciones que la síntesis del modelo citaba. Las que el modelo no usó, o las de una oración descartada por la comprobación, desaparecían de la página aunque estaban en el registro con su cita: en la revisión del 17 de octubre se perdieron así los reclamos de la contraparte. Lo mismo pasaba con las afirmaciones después de la número 120 de una entidad y con los datos de un asunto después del número 60.
- **Qué se decidió:** después de las referencias de la síntesis, una sección "Otros datos de los documentos" lista, sin redactar, cada afirmación que la síntesis no usó, con su número, su dato y su cita con el documento. Si todavía no hay síntesis, la misma lista se llama "Lo que dicen, dato por dato" y tiene todas. La síntesis se sigue escribiendo desde las primeras 120 afirmaciones (para no agrandar la pregunta), pero ninguna queda fuera de la página. La página de un asunto lista todos sus datos, sin tope. La versión de la proyección pasó a `proyeccion-3`, así que todas las páginas se vuelven a escribir sin preguntar nada.
- **Por qué:** una página que parece completa y omite en silencio lo que el registro sabe es peor que una página sin síntesis (R-23); la cita a la vista permite que una persona vea lo que el modelo dejó de lado.
- **Alternativas descartadas:** volver a preguntar la síntesis hasta que use todo (no garantiza nada y repite preguntas); listar todo dos veces, en la síntesis y aparte (ruido).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/pages.rs` (`render`, `render_matter`, `numbered`), `resources/catalog/knowledge.json` (`pages.words.saidList`, `saidOthers`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q1 de la tanda desatendida del 17 oct.

### D-237 · Lo que escribe el modelo del wiki pasa por un último control antes de guardarse

- **Fecha:** 17 oct 2026
- **Tema:** Wiki
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** en el wiki generado el 16 de octubre había 5 conceptos guardados sin `.md` (el lector y el índice no los ven), 2 páginas tituladas "Source: <ruta>", 51 páginas de documento con campos de referencia académica (`authors`, `year`, `url`, `venue`) heredados de la plantilla de investigación, 25 oraciones en 17 páginas que hablaban del propio wiki o "del material" ("ya figura en el wiki", "no está incluida en el material proporcionado", "el índice del wiki incluye…") y 30 enlaces en 24 páginas que no llevaban a ninguna página. Además, 3 páginas tenían restos de bloques `---FILE: …---` dentro del texto.
- **Qué se decidió:** cada página que escribe el modelo del wiki, después de la fusión con la que ya existía y antes de guardarse, pasa por un control determinista (`src/lib/ingest-legal-finish.ts`): la ruta siempre termina en `.md`; el título del encabezado es el primer `#` de la página (y un "Source: <ruta>" se vuelve el nombre del documento); se quitan los campos académicos; se quita cada oración que habla del wiki o de su material, y cuando solo la cola de la oración lo hace ("…las facturas ya registradas en el wiki") se quita la cola y queda lo que dice de los documentos; se quitan las líneas `---FILE:` sueltas; un enlace `[[…]]` que no lleva a una página del wiki en disco ni a una que se escribe en la misma respuesta queda como su texto. Lo quitado se anota en los avisos de la incorporación. Las instrucciones al modelo dicen lo mismo, para que lo haga bien de entrada. Las páginas de reserva que arma la app cuando falla el modelo se titulan con el nombre del documento, no "Source: …".
- **Por qué:** son defectos de forma que el programa detecta sin dudas; dejarlos al modelo no alcanzó. Una página que habla de "el material" o del wiki le habla al sistema, no al abogado. Las oraciones de ese tipo se buscan con frases acotadas para no tocar texto jurídico ("defectos en el material entregado" se conserva).
- **Medición:** sobre el wiki del 16 de octubre (sin escribir nada, `desatendido-3\Q1-medir-incorporacion.mjs`), el control habría corregido las 152 páginas que escribió el modelo así: 5 rutas sin `.md`, 4 títulos (2 "Source:" y 2 que no coincidían con su encabezado), 51 páginas con campos académicos, 25 oraciones sobre el wiki en 17 páginas (revisadas a mano: ninguna era texto jurídico) y 30 enlaces rotos en 24 páginas. Los 5 conceptos sin extensión del wiki actual se arreglan al regenerarlo (etapa Q5).
- **Alternativas descartadas:** pedírselo solo al modelo (ya estaba en parte y no alcanzó); crear páginas vacías para los enlaces rotos (agrega páginas sin contenido, que la revisión del 17 ya marcó como problema).
- **Dónde se aplica:** `src/lib/ingest-legal-finish.ts`, `src/lib/ingest.ts` (`writeFileBlocks`, `buildGenerationPrompt`, `buildFallbackSourceSummary`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q1 de la tanda desatendida del 17 oct.

### D-238 · El esquema de un proyecto nuevo es el de un estudio jurídico

- **Fecha:** 17 oct 2026
- **Tema:** Wiki
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el `schema.md` que la app escribe al crear un proyecto (también el proyecto general del archivo) era la plantilla de investigación del proyecto original, en inglés: hablaba de papers, pedía `authors`, `year`, `url` y `venue` en cada fuente y nombres de archivo `autor-año`. El modelo del wiki lo lee como instrucción, y de ahí salieron los campos académicos de 51 páginas (P-114).
- **Qué se decidió:** el esquema nuevo, en español, describe el wiki de un estudio jurídico: páginas de documento, conceptos jurídicos, entidades que escribe la app, encabezado sin campos académicos, enlaces solo a páginas que existen, que las páginas hablan de los documentos y no del wiki, y que los datos que difieren se muestran con sus dos valores. La tabla de tipos conserva su forma (la lee el programa para saber en qué carpeta va cada tipo).
- **Por qué:** el esquema es parte de la pregunta al modelo; una plantilla de otro dominio empuja al modelo hacia ese dominio.
- **Dónde se aplica:** `src-tauri/src/commands/project.rs` (`create_project_impl`). Vale para los proyectos que se creen desde ahora; el wiki del archivo se regenera en la etapa Q5 y recibe el esquema nuevo.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q1 de la tanda desatendida del 17 oct.

### D-239 · Cómo se presentan las páginas de entidad: nombre, archivo, vínculos, fechas y plurales

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la revisión del 17 de octubre encontró en las páginas que escribe la app: archivos cuyo nombre no coincide con el título (la entidad cambió de nombre y el archivo quedó con el de la primera proyección, D-195); páginas "conservadas" sin notas de nadie, solo porque otra página las enlazaba, y alguna con el id como título; "1 documentos en común" (197 veces); el mismo vínculo dos veces ("Deudor de X" dicho por la entidad y "Deudor de X" como inversa de lo que dice X); fechas en inglés ("December 30, 2021") en el texto de la página; nombres en mayúsculas ("APELLIDO, NOMBRE") cuando otros documentos los escriben como nombre propio; nombres partidos en dos líneas.
- **Qué se decidió:**
  - Cuando el nombre de una entidad o de un asunto cambia, su página pasa al archivo del nombre nuevo (con el mismo id corto al final), con las notas que tenía, y los enlaces de todas las páginas del wiki al archivo viejo (también en `related:`) pasan al nuevo. Reemplaza en parte a D-195 (nombre de archivo fijo).
  - Una página cuya entidad ya no tiene página se conserva **solo si una persona escribió notas** en ella; se titula con el nombre de la entidad (o el título que tenía), nunca con su id. Si no tiene notas se borra aunque otra página la enlace: esos enlaces pasan a la página de la entidad con la que se unió o, si no hay, quedan como texto. Reemplaza en parte a D-206.
  - "1 documento en común" en singular (`pages.words.inCommonOne`).
  - Un vínculo y su inversa dicha desde la otra entidad van en una sola línea, con las citas de los dos lados sin repetir.
  - Una fecha en inglés de un dato o de una oración de la síntesis se escribe en el idioma de las páginas ("30 de diciembre de 2021"), con los meses y la forma del vocabulario (`pages.words.months`, `dateFormat`). Las citas no se tocan nunca: quedan como las escribe el documento.
  - El nombre de una entidad va siempre en una línea, y un nombre escrito todo en mayúsculas toma la forma de nombre propio cuando los documentos escriben de otra manera cada una de sus palabras (se respeta su orden y su puntuación; si alguna palabra solo aparece en mayúsculas, queda como está). Se aplica al elegir el nombre en la resolución y al armar las páginas.
- **Por qué:** son defectos de presentación que hacen dudar de todo lo demás (R-23) y que el programa resuelve sin dudas. Mover el archivo con sus enlaces mantiene la regla de que ningún enlace se rompe; conservar páginas vacías por un enlace dejaba huérfanas en el wiki.
- **Medición:** ver el reporte de la etapa Q1 (evaluación sobre una copia de la base del 16 de octubre, `desatendido-3\Q1-eval-*.txt`).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/pages.rs` (`project`, `write_page`, `render`, `redirect_links`, `localized_dates`), `names.rs` (`shown_name`), `resolve.rs`, `resources/catalog/knowledge.json`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q1 de la tanda desatendida del 17 oct.

### D-240 · El resumen del archivo cuenta lo leído y ordena a los más nombrados por su papel

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el resumen decía "63 documentos leídos" de 66: contaba los documentos que nombran a alguien, no los leídos. Y "los más nombrados" se ordenaban por cantidad de documentos en que aparecen, aunque fuera de pasada (un servicio de correo o una jurisdicción nombrada en cada boleta subían a la cabeza).
- **Qué se decidió:** "Documentos leídos" cuenta los documentos cuya lectura de la capa salió bien; si alguna falló, una línea aparte dice cuántas (el detalle está en la vista de atención). "Los más nombrados" se ordenan por la cantidad de documentos en que la entidad tiene un papel de sustancia (parte, apoderado, firmante, remitente, destinatario, emisor: `pages.mostNamedRoles` del vocabulario); quien solo aparece de pasada no entra en esa lista, y el número que se muestra es el de esos documentos.
- **Por qué:** el resumen es lo primero que se lee del archivo; tiene que decir la verdad sobre lo procesado y destacar a quienes los documentos tratan, no a quienes mencionan.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/pages.rs` (`overview`), `vocabulary.rs`, `resources/catalog/knowledge.json`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q1 de la tanda desatendida del 17 oct.

### D-244 · El dinero es una relación entre un deudor y un acreedor

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la revisión del 17 de octubre encontró que cerca de 20 de 38 afirmaciones de dinero tenían mal el sujeto, la dirección o la naturaleza del importe. Los predicados `monto_adeudado` y `monto_reclamado` guardaban un importe suelto, sin la otra parte: la página decía "consta un monto adeudado" sin decir a quién, y faltaba la relación central del reclamo (quién le debe a quién).
- **Qué se decidió:**
  - La pregunta de lectura tiene una lista propia, `importes`: deudor (quien debe, paga o pone el dinero), acreedor (quien lo recibe, lo cobra o lo reclama), importe, moneda, tipo (total de factura, saldo, reclamo, cuota pagada, intereses, precio, honorarios, préstamo, inversión, otro), concepto y fecha, con su cita. Pide quién debe a quién **después de resolver los términos definidos** ("la Empresa" es la sociedad que el documento definió así).
  - Se guarda como una afirmación `importe` cuyo sujeto es el deudor y cuyo objeto es el acreedor, con el importe y su tipo (esquema V13: `amount` y `detail` en la evidencia y en la afirmación). Dos importes entre las mismas partes son dos afirmaciones. Se eliminan `monto_adeudado`, `monto_reclamado` y `acreedor_de` (que era la inversa de `deudor_de`); `deudor_de` queda para una deuda sin importe.
  - **Control de dirección, por regla general:** si la cita pone al deudor delante de un verbo de quien cobra ("X reclama a Y") o al acreedor delante de un verbo de quien paga ("Y le adeuda a X"), o al deudor detrás de "a favor de", el programa da vuelta las partes y lo cuenta (`direccion-invertida`). Un verbo en voz pasiva o dos verbos que dicen cosas distintas no deciden nada. Los verbos son datos del vocabulario (`money.payerCues`, `payeeCues`, `creditorMarkers`).
  - Un importe sin una de las partes no se guarda (`sin-contraparte`), tampoco uno con una palabra de impuesto justo antes ("Percepción IVA", `es-un-impuesto`) ni uno entre dos menciones de la misma entidad.
  - Como las partes salen de la lectura del documento entero ("Ud. adeuda la suma de…": el deudor es el destinatario), no se exige que la cita nombre a las dos.
  - Las páginas lo cuentan desde cada lado, con las palabras de cada tipo ("Debe a X", "Le debe X", "Reclama a X", "Le reclama X", "Pagó a X", "Cobró de X"…: `money.kinds` del vocabulario): el importe figura en la página del deudor y en la del acreedor, y no se repite en "Vínculos".
- **Por qué:** para un abogado el importe sin las partes no sirve, y la dirección es lo que más se equivocaba.
- **Medición:** sobre una copia de la base del 16 de octubre, con `gpt-6-luna` y una caché nueva (`desatendido-3\Q2-eval-*.txt`): vínculos de la hoja hallados entre las mismas entidades del registro, de 8 de 30 (26,7 %) a 21 de 32 (65,6 %); "acreedor de" de 0 a 7 de 8 y "deudor de" de 0 a 3 de 3. Muestra de atribución juzgada a ciegas: ver el reporte de la etapa Q2.
- **Dónde se aplica:** `src-tauri/resources/catalog/knowledge.json` (`importe`, `money`), `knowledge/extract.rs` (`amount_evidence`), `read_checks.rs` (`money_direction`, `amount_about_else`), `claims.rs`, `pages.rs` (`claim_label`, `amount_text`), `matters.rs`, `schema.rs` (V13).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q2 de la tanda desatendida del 17 oct.

### D-245 · Los términos definidos y los rótulos de formulario son alias locales, no nombres

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** "Empresa", "Cliente", "Trabajadora", "Contratista" quedaban como alias de entidades y se filtraban al texto de las páginas; "PROVINCIA" (el rótulo de un formulario) llegó a ser el nombre de una entidad con página.
- **Qué se decidió:** la pregunta pide los términos definidos ("en adelante «la Empresa»", "(«la Locataria»)", "hereinafter the «Company»") en una lista `definiciones`, con la mención que definen y una cita que tenga el término y el nombre. El programa además los busca por su forma en el texto (un término entre comillas precedido de "en adelante", "denominado", "hereinafter" o de un paréntesis) y, si el modelo no dijo a qué se refieren, toma el nombre más cercano antes de la definición. Una mención escrita como un término definido queda marcada (`knowledge_mention.defined`) y se une, dentro del documento, con lo que define: nunca da nombre a la entidad, no es alias en las páginas y no se compara con otros documentos. Una forma de escribir de una sola palabra que el texto escribe siempre seguida de dos puntos ("PROVINCIA:") es un rótulo de formulario y se descarta (`rotulo-de-formulario`).
- **Por qué:** un término definido vale solo dentro de su documento; tomarlo como nombre une o nombra mal.
- **Dónde se aplica:** `knowledge/extract.rs` (`define_terms`), `read_checks.rs` (`defined_terms`, `is_form_label`), `resolve.rs` (el principal de un grupo nunca es un término definido), `pages.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q2 de la tanda desatendida del 17 oct.

### D-246 · Una pregunta de lectura nueva: se vuelve a leer el archivo entero

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** los arreglos de esta etapa cambian lo que se le pide al modelo (importes, definiciones, clase de un dato, rótulo apartado, fecha del documento) y lo que el programa comprueba.
- **Qué se decidió:** versión de la pregunta `conocimiento-2` y comprobación `lectura-4`; el vocabulario de lectura incluye ahora las clases de los datos, los rótulos de los identificadores, los tipos de importe y los controles de lectura. Todo el archivo se vuelve a leer una vez (66 documentos, 75 preguntas). La síntesis pasa a `sintesis-2` y la proyección a `proyeccion-4`.
- **Por qué:** la regla de siempre (R-20): una pregunta distinta es otra pregunta.
- **Medición:** la relectura con `gpt-6-luna` tardó unos 10 minutos con 3 preguntas a la vez; ninguna lectura vacía ni fallida (antes, 2 fallidas).
- **Dónde se aplica:** `knowledge/extract.rs`, `vocabulary.rs`, `pages.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q2 de la tanda desatendida del 17 oct.

### D-247 · El domicilio tiene clase, tiempo y titular

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** 7 páginas de entidad y 2 de asunto decían "los documentos difieren sobre su domicilio" por un cambio en el tiempo, el domicilio de la sociedad por la que se firma, un domicilio laboral o constituido, o el mismo dato escrito distinto. El domicilio no tenía clase ni tiempo, y la lectura le daba al firmante la dirección del bloque de firma.
- **Qué se decidió:**
  - El modelo dice la **clase** de un domicilio (real, legal o sede, laboral, fiscal, constituido; "ninguna" si el texto no la dice). Las palabras de la cita ("constituye domicilio") siguen mandando para "constituido". Domicilios de clases distintas no se comparan.
  - El **tiempo** es el que dice la cita o, si no dice, la **fecha del documento**, que la lectura ahora pide (`fecha_documento`, comprobada en el texto; esquema V13). Dos domicilios con fechas distintas son una historia.
  - **Titular:** un domicilio que el modelo le dio a una persona, cuando entre su nombre y la dirección la cita nombra a una sola organización del documento ("X, por Ejemplo S.A., con domicilio en…"), pasa a la organización (`domicilio-de-la-sociedad`). La descripción del predicado dice que la dirección de un bloque de firma o de un membrete es de la organización.
  - Un código postal solo no es un domicilio.
- **Medición:** páginas con "los documentos difieren": de 8 a 2 (las dos de la misma persona, por dos direcciones del mismo documento que el modelo le dio como reales: la revisión ciega dice que una es de la sociedad que firma, ver P-121).
- **Dónde se aplica:** `knowledge.json` (`domicilio`), `extract.rs` (`signature_block`), `claims.rs`, `values.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q2 de la tanda desatendida del 17 oct.

### D-248 · Formularios: cada valor con su campo y su persona, y el rótulo aparte

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** las fichas de cliente perdían casi todo: el nombre está en dos campos ("Apellidos" y "Nombres") y ninguna cita lo contiene como una sola frase; en una factura o un formulario en columnas el rótulo ("CUIT:") y su valor quedan separados en el texto. Y un dato del cónyuge quedaba en el cliente.
- **Qué se decidió:**
  - El nombre de una mención está en su cita si están todas sus palabras (dos o más), aunque no en orden.
  - Una cita de rótulo y valor que no está seguida en el texto vale si el valor está en el texto y el rótulo, a lo sumo a `reading.labelWindow` palabras antes (y lo que siga, a lo sumo a esa distancia después). El modelo puede dar el rótulo aparte (`rotulo`); un rótulo que no está cerca del valor se ignora, nunca hace perder el valor.
  - Un dato o un identificador que la cita pone bajo un rótulo de otra persona (`reading.otherPersonLabels`: "cónyuge", "concubino") pasa a la única persona del documento nombrada con ese rótulo; si no hay una sola, no queda en nadie. Lo mismo para un número que el texto escribe siempre después del nombre de otra persona.
- **Medición:** identificadores de la hoja hallados, de 19 de 26 (73 %) a 21 de 26 (81 %); la primera versión, sin la cita en dos partes, perdía 116 identificadores por "cita no encontrada" (bajaron a 21 en total, contando todo).
- **Dónde se aplica:** `extract.rs` (`locate_apart`, `label_verified`, `other_person`), `read_checks.rs` (`name_in_quote`), `quote.rs`, `knowledge.json` (`reading`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q2 de la tanda desatendida del 17 oct.

### D-249 · Los identificadores por su rótulo; los números que no identifican a nadie

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** ~12 identificadores mal asignados: números de carta documento, de cliente, de sobre de firma, códigos postales, un EIN rotulado como CUIT, números de factura tomados como "expediente" (que es identificador fuerte y une). Y el mismo número escrito con y sin guiones se mostraba dos veces.
- **Qué se decidió:** clases nuevas de identificador, cada una con sus rótulos (datos del vocabulario): EIN (fuerte), número de cliente, número de factura, número de envío o carta documento, sobre de firma y código postal. El rótulo que el texto pone justo antes del número decide la clase, sea cual sea la que dijo el modelo (`clase-por-su-rotulo`); entre un rótulo débil y uno fuerte seguidos ("CUIT del cliente") gana el fuerte. Factura, envío, sobre y código postal **no identifican** a una persona ni a una organización: solo se guardan en la mención del asunto del documento (y sirven para agrupar asuntos, `matters.identifierKinds`); en una persona u organización se descartan (`no-identifica`). El valor de un identificador se compara y se muestra por sus letras y dígitos, sin separadores.
- **Medición:** uniones falsas de entidades de la hoja: de 89 pares de menciones (2 pares de entidades de la hoja unidas: la sociedad anónima con la LLC y la empresa de la contraparte con su grupo) a 5 pares (0 entidades de la hoja unidas). Separaciones falsas: de 10,4 % a 13,2 % de los pares (ver P-120).
- **Dónde se aplica:** `knowledge.json` (`identifiers`), `read_checks.rs` (`kind_by_label`), `extract.rs`, `vocabulary.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q2 de la tanda desatendida del 17 oct.

### D-250 · Lo que tiene consecuencias jurídicas nombra a su objeto en la cita

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** "beneficiario final de la operación" leído como de la sociedad; "representa a" sacado de una frase sobre un dominio de correo; la provincia como "parte".
- **Qué se decidió:** predicados nuevos `beneficiario_final_de` y `conyuge_de`. "Representa a", "socio de", "beneficiario final de", "cónyuge de" y "parte en" (`objectInQuote` en el vocabulario) se guardan solo si la cita nombra al objeto (por cualquiera de sus nombres en el documento, también un término definido); si no, se descartan (`objeto-fuera-de-la-cita`).
- **Por qué:** son los datos que un abogado usa para actuar; sin el objeto en la cita no hay cómo comprobarlos.
- **Costo:** se pierden representaciones reales cuyo bloque de firma pone la sociedad en otra línea (46 descartes de este tipo). Ver P-122.
- **Dónde se aplica:** `knowledge.json`, `extract.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q2 de la tanda desatendida del 17 oct.

### D-251 · El rol "domicilio constituido" y los roles de una constancia de firma

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el rol "domicilio" se usaba para "tiene domicilio" (17 menciones, 3 o 4 reales), y los roles salidos de la constancia de un servicio de firma (quién firmó el sobre, quién lo envió) se tomaban como los del acto.
- **Qué se decidió:** el rol pasa a `domicilio_constituido`, descrito como el lugar donde otra parte constituye domicilio, distinto de tener un domicilio (que es una afirmación). En las páginas del texto que tienen las marcas de una constancia de firma del catálogo de tipos (`reading.certificateMarks`), parte, remitente, destinatario y apoderado pasan a firmante (o a "otro" para una organización), según `reading.certificateRoles`.
- **Medición:** menciones con ese rol, de 17 a 3; 3 papeles corregidos por la constancia.
- **Dónde se aplica:** `knowledge.json`, `extract.rs`, `read_checks.rs` (`certificate_pages`), `src/vocabulary/legal.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q2 de la tanda desatendida del 17 oct.

### D-252 · Un conflicto es el mismo dato, de la misma clase, del mismo titular y del mismo tiempo

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464) · reemplaza en parte a D-191
- **El problema:** D-191 marcaba en conflicto dos valores de un dato de un valor a la vez cuando no tenían tiempo, y la síntesis decía "los documentos difieren" también para historias y datos de distinta clase.
- **Qué se decidió:** hay conflicto solo con el mismo predicado, la misma clase, el mismo titular, el mismo tiempo y valores distintos. Sin tiempo, los valores se muestran todos, sin compararlos. El dinero y los cargos nunca son "un solo valor". La consigna de la síntesis dice que no se hable de diferencias salvo para lo marcado en conflicto, que los datos de clases distintas no se contradicen y que un importe se cuenta con la otra parte.
- **Dónde se aplica:** `claims.rs`, `pages.rs` (`synthesis_question`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q2 de la tanda desatendida del 17 oct.

### D-253 · La muestra de atribución: 40 afirmaciones al azar, juzgadas a ciegas

- **Fecha:** 17 oct 2026
- **Tema:** Forma de trabajo
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la hoja de respuestas mide si se encontró un dato, no si quedó atribuido a quien corresponde. Hacía falta medir la atribución de forma repetible para comparar esta etapa y las variantes de modelo de la Q4.
- **Qué se decidió:** se toman con semilla fija 40 afirmaciones de una base (14 de dinero, 13 de domicilio y 13 de relaciones) con su cita y el texto completo de su documento; las muestras que se comparan se mezclan sin decir su origen y las juzga un revisor (un sub-agente) contra el documento: correcta, sujeto equivocado, dirección o naturaleza equivocada, objeto equivocado, o el documento no lo dice. Los scripts quedan fuera del repositorio porque las muestras nombran personas reales (`desatendido-3\Q2-muestra-atribucion.py`, `Q2-mezclar-muestras.py`, `Q2-contar-juicios.py`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q2 de la tanda desatendida del 17 oct.

### D-259 · Una entidad se nombra por su forma más completa que sea nombre propio

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el nombre visible de una entidad era el más repetido de sus nombres, y "PROVINCIA" (dos veces, un rótulo de formulario) le ganaba a "Gobierno de la Provincia de …" (una vez); un nombre partido por un escaneo ("Ejem plo", dos palabras) le ganaba como nombre principal del grupo de menciones a la razón social con su forma; y quedaban como nombres frases y palabras de rol ("la Provincia", "accionistas", "su carta documento").
- **Qué se decidió:** el nombre de una entidad es el más completo que sea nombre propio: el más usado de los que son nombres es el ancla, y entre los que tienen todas sus palabras (y su forma societaria) gana el de más palabras. Las palabras genéricas se amplían como datos del vocabulario (jurisdicciones como provincia, ciudad, estado o gobierno; plurales y palabras de rol; posesivos y demostrativos; nombres de documento como factura, carta o recibo). Un nombre que empieza con un artículo, un posesivo o un demostrativo en minúscula seguido de una palabra en minúscula es una frase (`identity.determiners`). Un asunto no se nombra por el nombre de un archivo ni por palabras comunes. En un grupo de menciones, la forma societaria pesa antes que la cantidad de palabras.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q3 de la tanda desatendida del 17 oct.

### D-260 · Grupos de sociedades

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464); ampliada en D-473 (el grupo se crea siempre que haya una señal firme)
- **El problema:** la empresa propia aparece como tres sociedades (S.A., LLC, LTD) con el mismo nombre: está bien que sean tres, pero lo que se nombra sin forma y la casilla de firma que usan todas terminaban unidos a una sola, que heredaba documentos de las otras; y un grupo nombrado en el texto ("Grupo X") no tenía relación con sus sociedades.
- **Qué se decidió:** varias organizaciones con el mismo nombre propio que difieren solo en la forma societaria (formas del vocabulario) y con al menos otra señal que las vincule (la misma persona como su representante, socia o empleada, `identity.groupLinks`, o como apoderada en documentos de cada una; la misma casilla general en documentos de varias; un documento que nombra a dos y dice grupo o afiliada, `identity.affiliateWords`) son las sociedades de un grupo: lo que se nombra sin forma y no se pudo atribuir a una sola, y esa casilla, son del grupo, que se nombra por el nombre compartido. Solo el nombre compartido no alcanza: quedan las dudas para una persona (R-07). Un grupo nombrado en los documentos (`identity.groupWords`) existe por su cita, y sus sociedades son las que empiezan con el resto de su nombre y aparecen con él en un documento. Se guarda en `knowledge_group` (esquema V14) con los motivos; la página del grupo lista sus sociedades con el porqué y la de cada sociedad dice de qué grupo es. El grupo existe como entidad solo cuando hay algo que atribuirle (P-128).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q3 de la tanda desatendida del 17 oct.

### D-261 · La sociedad sin forma, dentro de un documento, es la única forma completa del documento

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** dentro de un mismo documento, "X" y "X S.A." quedaban como dos grupos de menciones cuando el modelo no las había dado como la misma, y "X" iba después a una duda entre todas las sociedades de ese nombre.
- **Qué se decidió:** dentro de un documento, el nombre de una organización sin forma societaria se une con la única forma completa de ese nombre que escribe el documento (las mismas palabras con forma, o un nombre más largo que empieza con ellas), cuando ninguna otra organización del documento tiene esas palabras y ningún identificador fuerte los separa. Si el documento escribe dos formas, no se une.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q3 de la tanda desatendida del 17 oct.

### D-262 · Áreas y casillas generales

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** "Legales X", "X Finance", "Línea Ética de X", `legal@…` o `noreply@…` se comparaban por su nombre con las sociedades: la casilla de firma del grupo se unió con una de ellas, y las áreas de una organización quedaban como organizaciones aparte.
- **Qué se decidió:** un nombre sin forma societaria con palabras de un área (`identity.areaWords`) o una casilla cuyo usuario es una de ellas es un contacto: nunca se une por nombre. Va a la organización que nombra su mismo documento (las palabras que quedan sin las del área, sus iniciales o las letras del dominio de la casilla) o, si el documento no nombra ninguna, a la única organización del archivo con el nombre de su dominio. La misma área en varios documentos es una: si todos dicen la misma organización, es de ella; si dicen varias, es de su grupo (D-260). Un área que no nombra a nadie ("Cobranzas" sola) y una casilla general leída como persona no son entidades (motivo `casilla`). Una persona y una organización no se unen nunca. Un área o una casilla no da nombre a la entidad a la que pertenece.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q3 de la tanda desatendida del 17 oct.

### D-263 · Dos asuntos del mismo nombre entre otras partes no se preguntan

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el modelo de identidad unía asuntos que se llaman igual ("Acuerdo de terminación") en documentos de personas distintas, y eso armaba un asunto con documentos de tres acuerdos (P-123).
- **Qué se decidió:** dos menciones de asunto de documentos distintos no son candidatas cuando cada documento tiene dos partes o más (con rol de parte, emisor, remitente o destinatario) y las de ninguno están dentro de las del otro: un contrato es entre sus partes. Un recibo de cuota y su acuerdo, con las mismas partes, siguen siendo candidatos.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q3 de la tanda desatendida del 17 oct.

### D-264 · Todos los nombres de un grupo de menciones

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la resolución comparaba y buscaba candidatos solo por el nombre principal de cada grupo de menciones; el nombre corto que el propio documento da a una organización ("Universidad de Ejemplo (UDE)") no servía para encontrarla en otro documento que solo escribe el corto.
- **Qué se decidió:** un grupo de menciones se une por nombre exacto con cualquiera de sus nombres (salvo los términos que define el documento), se encuentra por las claves de todos ellos y se compara por el principal o, si no se parecen, por el mejor par de sus otros nombres. La sigla, la errata o el nombre corto quedan como candidatos para el modelo, nunca unidos solos.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q3 de la tanda desatendida del 17 oct.

### D-265 · La errata de una razón social

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** una razón social con una letra de menos en dos cartas documento quedaba separada: el modelo decía "dudosa" aunque los documentos tenían las mismas partes.
- **Qué se decidió:** la pregunta de identidad (identidad-4) dice que una razón social con la misma forma societaria que difiere en una o dos letras, en documentos donde aparecen las mismas otras partes, es la misma (el error es del documento), salvo que algún documento muestre otra organización con ese nombre. Medido: separaciones falsas 6,9 % → 1,6 % de los pares, sin uniones falsas.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q3 de la tanda desatendida del 17 oct.

### D-266 · Qué persona u organización tiene página

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** una entidad tenía página si aparecía en dos documentos, aunque solo se la nombrara al pasar: bancos de las facturas, jurisdicciones de las boletas, servicios de correo, productos.
- **Qué se decidió:** una persona o una organización tiene página si en algún documento tiene un rol de sustancia (parte, apoderado, firmante, remitente, destinatario, emisor, domicilio constituido, testigo) o si los documentos dicen algo de ella (`withClaims`); aparecer en muchos documentos no alcanza (`minDocuments` 0). Un grupo de sociedades tiene página siempre. Se dejaron emisor, domicilio constituido y testigo además de los cinco roles que nombra la especificación porque son papeles del acto (quien factura, el estudio donde se constituye domicilio, el escribano).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q3 de la tanda desatendida del 17 oct.

### D-267 · El nombre de un asunto

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** los asuntos se llamaban por un comprobante ("Carta Documento — …", "BOLETA N° …", "factura N° …"), por el título de un sobre de firma o por una frase, con alias que eran números y frases.
- **Qué se decidió:** un asunto se nombra por la relación entre sus partes y su objeto: si sus documentos dicen que una de sus partes le reclama a otra (`matters.claimKinds`), "Reclamo de X a Y"; si no, lo que sus documentos lo nombran, entre sus partes ("Acuerdo de terminación entre X y Y", en minúscula si estaba todo en mayúsculas); si no, "Asunto entre X y Y". Un nombre de asunto no es un archivo, una frase, un título de más de ocho palabras, un comprobante por su rótulo ni un nombre con un número de cuatro dígitos o más, salvo la carátula de una causa ("Expediente 1234/2020"); lo mismo vale para sus alias. Las partes de un documento para los asuntos incluyen emisor, remitente y destinatario.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q3 de la tanda desatendida del 17 oct.

### D-268 · Documentos centrales por evidencia y el porqué plegado

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** documentos centrales de un reclamo (la cronología, correos) quedaban fuera del asunto cuando su lectura no tomaba el número que comparten; y la explicación de por qué van juntos era una lista larga e ilegible.
- **Qué se decidió:** un número de un asunto (uno que ya une documentos por su identificador o por los datos de su tipo) escrito en el texto de otro documento también lo pone en el asunto. En la página del asunto, el porqué se resume (cuántos documentos por cada clase de motivo) y el detalle queda plegado, con la sintaxis de llamada plegada de Obsidian (`> [!info]- …`), que el lector del wiki dibuja como un desplegable cerrado (`src/lib/markdown-fold.ts`, sin leer HTML de la página).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q3 de la tanda desatendida del 17 oct.

### D-269 · Una representación con la sociedad en la línea anterior

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** D-250 descartaba "representa a" cuando la cita no nombraba a la representada; en un bloque de firma ("Por: X S.A." / "Nombre: Y" / "Cargo: Apoderado") o en "X S.A., representada por Y" la sociedad está en otra línea, y se perdían representaciones reales (P-122).
- **Qué se decidió:** vale si la última organización que el texto nombra antes de la cita, a menos de 30 palabras (`reading.objectWindow`), es la representada y la cita dice cómo actúa por ella (`reading.representationCues`: apoderado, representante, representada, cargo, title, by…). Se vuelve a comprobar desde las respuestas guardadas (lectura-5). Medido: 3 de las 12 representaciones descartadas vuelven, las tres correctas; ninguna inventada.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q3 de la tanda desatendida del 17 oct.

### D-274 · El razonamiento de Codex, parte del motor

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** para comparar modelos con distinto razonamiento hacía falta pasarle el nivel a Codex y que las respuestas de un nivel no se mezclaran con las de otro en la base ni en las cachés de evaluación.
- **Qué se decidió:** el nivel va a Codex como `-c model_reasoning_effort=<nivel>` (en las preguntas sobre documentos, `source_volume/asking.rs`, y en las llamadas del wiki, `codex-cli-transport.ts`), y forma parte del nombre del motor: `codex-cli:modelo@nivel`. Solo se ofrecen `low`, `medium` y `high` (por decisión del dueño, el proceso se mejora ajustando las preguntas, no gastando más en cada respuesta); otro nivel es un error. Ajustes tiene un selector para el preset de Codex (`src/lib/codex-reasoning.ts`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q4 de la tanda desatendida del 17 oct.

### D-275 · Un CUIT con prefijo de persona dado a una sociedad

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** en una segunda corrida del mismo modelo, una sociedad recibió el CUIT de su apoderada (el número de la persona, escrito junto a su DNI). Con dos identificadores fuertes distintos la sociedad ya no se unía por nombre y quedó partida en diez.
- **Qué se decidió:** el vocabulario dice qué primeros dígitos del CUIT son de persona y cuáles de sociedad; la comprobación de una lectura (lectura-6) pasa ese número a la única mención del otro tipo que lo contiene (el DNI dentro del CUIT) o lo descarta. Medido en la corrida que lo mostró: la sociedad vuelve a ser una.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q4 de la tanda desatendida del 17 oct.

### D-276 · Cómo se comparan modelos

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** elegir modelo y razonamiento por impresión no sirve: hay que medir lo mismo para todos.
- **Qué se decidió:** cuatro variantes (`gpt-6-luna` y `gpt-6.1-sol`, con razonamiento medio y alto), cada una con lectura, identidad, asuntos y síntesis completas sobre la carpeta de prueba y su propia caché de respuestas. Se mide contra la hoja (menciones, roles, afirmaciones, uniones y separaciones falsas, asuntos), con una muestra de atribución de 40 afirmaciones de dinero, domicilio y relación por variante juzgada contra el documento por un revisor que no sabe de qué variante sale cada una (las muestras mezcladas), con los casos de control y con el tiempo por respuesta. Para volver a medir con el programa nuevo sin volver a preguntar, el arnés puede correr solo con las respuestas guardadas (`MICELYA_ASK_OFFLINE=1`).
- **Resultados** (atribución a ciegas, dos lotes): luna medio 33/40 (82 %; dinero 71 %); luna alto 40/40 y 38/40 en su segunda corrida (97,5 % entre las dos); sol medio 39/40 (98 %); sol alto 38/40 (95 %). Uniones falsas: la misma única unión falsa en las cuatro (P-134). Separaciones falsas: luna medio 2,5 %, luna alto 2,2 %, sol medio 1,3 %. Menciones 84/92/92/92 %; afirmaciones 56/61/74/77 %; asuntos agrupados 70/76/56/56 %, sin agrupaciones falsas. Segundos por respuesta: 9,6 / 13,8 / 22,9 / ~64.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q4 de la tanda desatendida del 17 oct.

### D-277 · El mismo modelo cambia entre corridas

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** dos corridas desde cero de la misma variante no dan lo mismo.
- **Qué se decidió:** se mide y se tiene en cuenta: con luna alto, 9,6 % de los pares de menciones presentes en las dos corridas quedan juntos en una y separados en la otra (luna medio, 2,0 %), y solo 26 a 31 % de las afirmaciones son las mismas. La segunda corrida de luna alto mostró además la sociedad partida por un CUIT mal atribuido (D-275). Una diferencia de un par de puntos entre variantes o entre versiones de una pregunta, medida en una sola corrida, no alcanza para decidir; las respuestas guardadas hacen que el resultado de un archivo ya leído no cambie mientras no cambie la pregunta (R-20). Cómo hacerlo más estable queda en P-133.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q4 de la tanda desatendida del 17 oct.

### D-278 · gpt-6.1-sol como alternativa

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** sol medio empata en atribución con luna alto y encuentra más afirmaciones; sol alto es el que más encuentra, pero es muy lento.
- **Qué se decidió:** `gpt-6.1-sol` queda como el otro modelo que ofrece el preset de Codex, no como el de por defecto. Sol alto se descarta por tiempo: unos 64 segundos por respuesta (4,6 veces luna alto), y en el tiempo de la comparación no llegó a completar la identidad (116 de 146 preguntas sin responder), así que sus uniones y separaciones no están medidas del todo (P-132). Sol medio agrupa peor los asuntos (56 % frente a 76 %) y tarda 1,7 veces más que luna alto.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q4 de la tanda desatendida del 17 oct.

### D-279 · Por defecto, gpt-6-luna alto

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la app usaba por defecto un modelo de Codex que no es de los permitidos y razonamiento medio.
- **Qué se decidió:** por defecto, `gpt-6-luna` con razonamiento alto (`DEFAULT_CODEX_REASONING`, `defaultModel` del preset de Codex). Por calidad primero: atribución 97,5 % en dos corridas (empata con sol medio, 98 %, y supera a luna medio, 82 %, y a sol alto, 95 %), la misma única unión falsa que todas, separaciones falsas 2,2 %; entre las que empatan, por tiempo (13,8 segundos por respuesta, frente a 22,9 de sol medio) y por asuntos (76 % frente a 56 %). Lo que cede: menos afirmaciones que sol (61 % frente a 74 %) y más variación entre corridas que luna medio (D-277). El motor guardado en `docbase_setting` y el modelo del wiki se fijan al regenerar el wiki (etapa Q5).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q4 de la tanda desatendida del 17 oct.

### D-280 · Una carpeta nueva tiene un solo id de proyecto y su cola nunca queda huérfana

- **Fecha:** 17 oct 2026
- **Tema:** Desktop
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Al regenerar el wiki, las 77 incorporaciones fallaron todas con "Project not found in registry". El arranque de la app corre dos veces en desarrollo y el proyecto por defecto se abre desde más de un lugar: dos aperturas a la vez de una carpeta sin `.llm-wiki/project.json` crearon dos id; la cola se quedó con uno y el registro con el otro, y además dos escrituras del registro a la vez perdieron una entrada.
- **Qué se decidió:** Las consultas del id de una misma carpeta comparten una sola promesa (`ensureProjectId`), las escrituras del registro van de a una (`upsertProjectInfo`), y al restaurar la cola de una carpeta se adoptan las tareas de su propio archivo encoladas con otro id: las que fallaron solo por "proyecto no encontrado" vuelven a esperar. Así se recuperaron las 77 sin clic, con una recarga de la página. Commit 6f47009, con prueba.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q5 de la tanda desatendida del 17 oct.

### D-281 · Un nombre comercial con otra sociedad entre paréntesis no es esa sociedad

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** La única unión falsa que quedaba en las cuatro variantes de D-276 (P-134): "Marca (Sociedad S.A.)" unía dos organizaciones por el "es la misma" del modelo.
- **Qué se decidió:** Dentro de un documento, el "es la misma" del modelo ya no une un nombre completo de sociedad con un nombre de organización que no tiene nada en común con él; los términos definidos siguen a su definición. Uniones falsas contra la hoja: 0. Commit 5e6fb20.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q5 de la tanda desatendida del 17 oct.

### D-282 · Una abreviatura sin su último punto cuenta por la forma con punto

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** "S.A" y "S.A." se mostraban como dos formas de escribir.
- **Qué se decidió:** Se cuentan como una y se muestra la forma con punto. Commit 5e6fb20.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q5 de la tanda desatendida del 17 oct.

### D-283 · Fechas en el idioma del wiki también con ordinales en inglés y con la forma de las escrituras

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Quedaban fechas como "30th of December" o "23 días del mes de Octubre de 2025" sin llevar al idioma del wiki, y el concepto de un importe repetía su tipo.
- **Qué se decidió:** Los días ordinales en inglés y la forma de las escrituras se muestran como "23 de octubre de 2025"; el concepto de un importe que repite su tipo se dice una vez. Commits 5e6fb20 y 988c04d.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q5 de la tanda desatendida del 17 oct.

### D-284 · Los datos de contacto solos no le dan página a una entidad

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Las listas de teléfonos y oficinas de un formulario creaban páginas de organizaciones que el archivo no trata.
- **Qué se decidió:** Una afirmación de contacto no es razón para tener página (criterio de D-266). Commit 5e6fb20; ampliada en D-288 al domicilio.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q5 de la tanda desatendida del 17 oct.

### D-285 · El nombre propio de un asunto sin artículo inicial ni el nombre de una parte detrás de un guion

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Asuntos llamados "This Agreement" o "Acuerdo de terminación - <persona> entre … y <persona>".
- **Qué se decidió:** Se quita el determinante inicial y el tramo después de un guion o dos puntos que solo nombra a una de sus partes, que el nombre ya dice después. Commit 5e6fb20.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q5 de la tanda desatendida del 17 oct.

### D-286 · Más palabras genéricas y de áreas en el vocabulario

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** "cada", "cualquier" o "each" pasaban como nombres; "payroll" o "sueldos" no se reconocían como áreas.
- **Qué se decidió:** Se agregan como determinantes y genéricos, y como palabras de área, en `knowledge.json`. Commit 5e6fb20.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q5 de la tanda desatendida del 17 oct.

### D-287 · Un código postal o un DNI no es un teléfono

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Un código postal junto a un domicilio o un DNI escrito con puntos aparecían como contacto de la persona.
- **Qué se decidió:** Un contacto es un correo o un número de siete dígitos o más que no esté escrito con puntos cada tres dígitos (`lectura-7`). Commit 988c04d.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q5 de la tanda desatendida del 17 oct.

### D-288 · Ronda 1 de la revisión del wiki regenerado: valores que no lo son, constancias de firma, nombres y asuntos

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** La revisión del wiki regenerado con luna alto (Q5) encontró errores generales del programa: "By: ____" como cargo, un horario ("días hábiles de 9 hs a 18 hs") como domicilio, la fecha y hora de emisión de una boleta como teléfono, la fecha de firma electrónica como inicio de un vínculo, el título del formulario de un servicio de firma como nombre de un asunto, asuntos nombrados por un firmante en vez de la otra parte o como "Asunto de <persona>", el nombre de pila y el apellido de dos campos de una ficha que dejaban a una persona nombrada solo por sus nombres, y páginas de jurisdicciones de una boleta que solo tenían la dirección de una oficina.
- **Qué se decidió:** Reglas generales, sin nombrar casos: una línea para completar no es valor de nada; un domicilio no es un horario (palabras `schedule` del vocabulario, zona horaria); una fecha u hora no es un teléfono (`lectura-8`). En una página que es la constancia de un servicio de firma solo se toman contacto, cargo y domicilio (`certificateClaims`), y el asunto que se lee ahí no es el asunto del documento (`certificateRoles`). El domicilio solo tampoco da página (amplía D-284). El nombre de una entidad es el que contiene más de lo que escriben los documentos (la forma que junta nombre y apellido gana a la que solo tiene los nombres). El nombre de un asunto pone primero a quien es parte del acto, prefiere el nombre que contienen más de sus documentos y, sin nombre propio, usa el tipo común de todos sus documentos ("Boleta de …") (`resolucion-7`). Medido sin preguntar al modelo sobre una copia de la base de la app (`Q5-R1-eval.txt`): identidad igual (separaciones falsas 4,7 %, uniones falsas 0), asuntos 75,9 % sin agrupaciones falsas, roles 73 → 75 %, afirmaciones de la hoja 67 % igual; desaparecen los 4 tipos de valor falso y los 7 asuntos quedan bien nombrados. Commit f29dab7.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q5 de la tanda desatendida del 17 oct.

### D-289 · Presentación de identificadores, alias y nombres de personas

- **Fecha:** 17 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Las páginas mostraban correos como "Otro número", el mismo DNI dos veces (con su tipo y como "otro número"), alias como "<nombre> (President)", sobres de firma y facturas como si nombraran un asunto, y nombres de personas en mayúsculas ("PERSONA UNO", "Persona Dos TRES") cuando no había otra forma escrita.
- **Qué se decidió:** Un correo no es identificador; un número dado con su tipo y como "otro número" se muestra una vez, con el tipo; una forma que es otra más una nota entre paréntesis no es alias; en un asunto solo se muestran los números que lo nombran por sí solos (los fuertes, como un expediente); el nombre de una persona escrito en mayúsculas se muestra como nombre propio (una persona no tiene siglas; el de una organización queda como está) (`proyeccion-6`). Commit f29dab7.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa Q5 de la tanda desatendida del 17 oct.

### D-290 · Chat, búsqueda y grafo se conservan, sin redundancias

- **Fecha:** 17 oct 2026
- **Tema:** Producto
- **Estado:** vigente
- **El problema:** al revisar el wiki generado aparecieron partes heredadas del proyecto original (conceptos por documento, páginas de fuente, consultas, enlaces escritos en las páginas) que repiten lo que ya hacen la base documental y la capa de conocimiento, algunas con usos del modelo.
- **Qué se decidió:** el chat, la búsqueda y el grafo son valiosos y se conservan. Hay que ajustar todo para que no haya redundancias, ni en la interfaz ni en el proceso, y para que no se haga procesamiento innecesario, en particular con el modelo. Lo que tenía sentido en el proyecto original y ya no lo tiene en el nuestro se quita. La propuesta está en `docs/design/wiki-sobre-la-base-propuesta.md`, para revisar.
- **Por qué:** cada dato en un solo lugar; menos costo y menos errores.
- **Alternativas descartadas:** quitar el chat, la búsqueda o el grafo.
- **Dónde se aplica:** `wiki-sobre-la-base-propuesta.md`; P-142.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 17 oct.

### D-291 · Servicios de IA con proveedor configurable, preparados para Micelya Core

- **Fecha:** 17 oct 2026
- **Tema:** IA
- **Estado:** vigente
- **El problema:** la búsqueda por significado necesita calcular vectores, y Micelya Core va a ofrecer ese y los demás servicios de IA, con sus proveedores y modelos configurables allí.
- **Qué se decidió:** la app tiene que poder usar Core y, a la vez, mantener la posibilidad de configurar el proveedor de cada servicio de IA (generativa y no generativa). Conviene empezar a aislar toda la configuración y el uso de servicios de IA en un solo lugar, con un contrato por servicio, para que la integración con Core sea sencilla. No es el foco ahora.
- **Por qué:** Core será la vía principal, pero la app no debe depender de un solo proveedor.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** `wiki-sobre-la-base-propuesta.md` (sección 9); P-141; P-063; P-140.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 17 oct.

### D-292 · La comprobación corre en todo PR; no apilar PR salvo que sea inevitable

- **Fecha:** 18 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente
- **El problema:** la comprobación obligatoria de `main` solo corría en los PR que apuntan a `main`. Los PR apilados (cada uno sobre la rama del anterior) nunca la ejecutaban, y cuando GitHub les cambiaba la base a `main` tampoco arrancaba: quedaban bloqueados hasta que alguien actualizaba la rama a mano (#51, #52, #53).
- **Qué se decidió:** la comprobación corre en todo PR, apunte a la rama que apunte (`.github/workflows/ci.yml`, PR #56). Además, no se apilan PR salvo que sea inevitable: un proceso, un PR contra `main`.
- **Por qué:** el problema lo causó la forma de armar los PR, no el dueño; el arreglo evita que vuelva a pasar aunque haga falta apilar.
- **Alternativas descartadas:** seguir actualizando la rama a mano en cada PR apilado.
- **Dónde se aplica:** `.github/workflows/ci.yml`; `CLAUDE.md`.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 18 oct.

### D-293 · Ajustes al rediseño del wiki sobre la base

- **Fecha:** 18 oct 2026
- **Tema:** Wiki
- **Estado:** vigente
- **El problema:** la propuesta "el wiki como presentación de la base" (P-142) prometía más de lo que garantizaba en varios puntos: llamaba "verificadas" a afirmaciones cuya cita existe pero cuya lectura puede estar mal; dejaba que el grafo recortara la búsqueda, con lo que un documento relevante sin relación cargada desaparecería; hablaba de "una sola lectura" sin decir qué se reutiliza ni qué pasa con las correcciones de una persona en una relectura; juntaba en una página documentos que no son versiones de uno; y ataba el asunto a la cantidad de documentos.
- **Qué se decidió:** (1) Cada afirmación lleva su certeza y quién la confirmó; la síntesis escribe lo dudoso como dudoso; se dice "con cita comprobada", no "verificada"; una corrección hecha una vez se propaga porque las páginas se arman desde el registro. (2) El grafo ordena y amplía, no recorta: la búsqueda siempre recorre todo lo visible, salvo que la persona elija a propósito un conjunto (por ejemplo, un asunto); se mide cuántos documentos relevantes se perderían si el grafo recortara. (3) "Una lectura" quiere decir no repetir trabajo: una extracción principal por contenido y configuración, reutilizada por todo; una comprobación extra con el modelo solo si su mejora está medida; las decisiones de una persona (unir, separar, incorrecto) sobreviven a una relectura. (4) Página común solo para versiones del mismo documento (borrador, firmado, vista previa); cada afirmación dice de qué versión sale y, si difieren, se ven las dos; documentos de la misma plantilla son páginas distintas. (5) El asunto no depende de cuántos documentos tiene (la regla de dos documentos vale solo para la agrupación automática); un solo concepto de asunto (P-061) que lo propone la extracción, lo crea una persona o lo propone una regla de carpeta.
- **Por qué:** que el producto no prometa más de lo que garantiza, que la búsqueda no pierda documentos por una relación faltante y que el trabajo del modelo y de las personas no se repita ni se pierda.
- **Alternativas descartadas:** la propuesta del 17 oct tal como estaba (grafo que acota, "afirmaciones verificadas", asunto solo con dos o más documentos).
- **Dónde se aplica:** `docs/design/wiki-sobre-la-base-propuesta.md` (secciones 2, 3, 5, 7 y 10); `docs/design/conocimiento-propuesta.md`; P-142; P-061.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 18 oct (análisis externo revisado contra el código).

### D-294 · Decisiones abiertas del rediseño del wiki, resueltas

- **Fecha:** 18 oct 2026
- **Tema:** Wiki
- **Estado:** vigente
- **El problema:** la propuesta del 17 oct dejaba cuatro decisiones abiertas (sección 11) que bloqueaban su construcción: cómo existe la página del documento, cómo nace un asunto, qué pasa con la investigación en internet y con qué proveedor de vectores se empieza.
- **Qué se decidió:** (1) La página del documento es por ahora un archivo generado, como las de entidad (regenerable, notas protegidas); pasa a armarse al mostrarse cuando la búsqueda y el grafo lean de la base. (2) El asunto queda como en D-293, punto 5; no se construye todavía. (3) La investigación en internet es una herramienta aparte, a pedido, que nunca escribe en la base ni en el wiki del archivo; por ahora solo se desconecta de la generación del wiki. (4) El proveedor de vectores no se decide todavía; los vectores no entran en esta etapa.
- **Por qué:** destrabar la construcción con lo que hoy se puede medir, sin adelantar decisiones que dependen de Micelya Core o de cómo se usa el asunto.
- **Alternativas descartadas:** armar la página del documento al mostrarse desde ya (requiere que la búsqueda y el grafo lean de la base primero); quitar la investigación en internet.
- **Dónde se aplica:** `docs/design/wiki-sobre-la-base-propuesta.md` (secciones 4, 5 y 11); P-142; P-061.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 18 oct (análisis externo revisado contra el código).

### D-295 · Conceptos por cálculo: por ahora solo medición de pasajes repetidos

- **Fecha:** 18 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente
- **El problema:** la propuesta quita los conceptos por documento (casi todos cláusulas de un solo contrato) y plantea, como hipótesis, conceptos propuestos por cálculo: pasajes que se repiten en documentos distintos y la misma idea dicha con otras palabras. La segunda mitad necesita vectores, que no entran todavía.
- **Qué se decidió:** por ahora solo se mide la mitad de los pasajes repetidos, fuera del repositorio y sin pantallas: cuántos grupos salen y cuántos son cláusulas o figuras reconocibles. La mitad "la misma idea con otras palabras" espera a los vectores.
- **Por qué:** decidir con números si los conceptos por cálculo valen la pena antes de construirlos.
- **Alternativas descartadas:** construir los conceptos por cálculo ya; conservar los conceptos por documento.
- **Dónde se aplica:** `docs/design/wiki-sobre-la-base-propuesta.md` (sección 8); P-142.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 18 oct (análisis externo revisado contra el código).

### D-296 · Orden de prioridad y criterio "demostrar mejora"

- **Fecha:** 18 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente
- **El problema:** el wiki generado el 17 oct tiene errores críticos (atribución a la persona equivocada, importes mal leídos, uniones falsas) junto con mejoras de forma pendientes. Medir solo que algo "no empeora" no alcanza para saber si un error crítico quedó corregido.
- **Qué se decidió:** el orden de prioridad es (1) atribución, importes y uniones falsas; (2) estabilidad y documentos perdidos en la búsqueda; (3) corregir y deshacer visibles en la app; (4) presentación. Para los errores críticos conocidos hay que demostrar mejora con casos de aceptación concretos y dos corridas comparables (mismo modelo, nivel y carpeta), antes y después; no basta con no empeorar.
- **Por qué:** lo que más daña la confianza de un estudio es un dato mal atribuido o un importe equivocado; la mejora tiene que poder mostrarse con números.
- **Alternativas descartadas:** priorizar la presentación del wiki; aceptar una corrección sin casos de aceptación.
- **Dónde se aplica:** `docs/design/wiki-sobre-la-base-propuesta.md` (secciones 10 y 11); R-17.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 18 oct (análisis externo revisado contra el código).

### D-300 · Casos de aceptación de los errores críticos, comprobables sobre cualquier base

- **Fecha:** 18 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** D-296 pide demostrar mejora en los errores críticos con casos concretos y dos corridas comparables, pero los errores conocidos (importes con naturaleza mal, casillas de las fichas, afirmaciones mal atribuidas, entidades partidas, regresiones de Q5) solo estaban descritos en prosa, y la lista de importes de la revisión de Q5 no estaba detallada.
- **Qué se decidió:** una hoja de casos de aceptación fuera del repositorio (`conjunto-prueba/casos-criticos.json`, nombra personas reales) con 46 casos: documento, cita textual comprobada contra el texto de la base, lo correcto, lo que sale mal y una regla que se comprueba sola sobre cualquier base (por ruta del documento, no por número interno; lee columnas nuevas de las afirmaciones sin cambiar el comprobador). Tipos de regla: naturaleza de un importe, casilla de un formulario, afirmación ausente o presente, identificador ausente, menciones en el mismo ente, ente con página, correos como contacto y encabezado de página. Un script fuera del repositorio da "n de m casos pasan" por categoría. Tres casos de importes son de control (lo que hoy está bien no puede romperse). Con la base de partida pasan 3 de 46.
- **Por qué:** que N2a, N3 y N6 midan los mismos casos antes y después, con números, sin depender de una revisión a mano.
- **Alternativas descartadas:** pruebas en el repositorio con los casos reales (R-24 lo prohíbe); medir solo con la hoja de entidades (no cubre importes, casillas ni atribuciones).
- **Dónde se aplica:** fuera del repositorio (`llm_wiki_mi/conjunto-prueba/casos-criticos.json`, `desatendido-4/N1-comprobar-casos.py`); P-135, P-136, P-137, P-151.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N1 de la tanda desatendida del 18 oct.

### D-301 · Línea de base de la búsqueda con preguntas de un abogado

- **Fecha:** 18 oct 2026
- **Tema:** Wiki
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
- **El problema:** el rediseño (D-293) cambia la búsqueda a la base con el grafo que ordena y amplía, pero no había forma de saber cuántos documentos relevantes encuentra la búsqueda de hoy ni de comparar la nueva.
- **Qué se decidió:** una hoja de 30 preguntas fuera del repositorio (`conjunto-prueba/preguntas.json`) con sus documentos esperados, mezclando formas de nombrar (exacta, parcial, otra forma escrita, sin nombre) y categorías (entidades partidas, contradicciones y versiones, documentos sin asunto, importes, identificadores, cláusulas, relaciones). La medida es por documento (las copias cuentan como uno): esperados en los primeros 5 y 10, cuáles faltan y tiempo. Variantes de hoy: la búsqueda del wiki que usa el chat contando solo páginas de documento (61 de 104 en los 10 primeros), el índice de texto de la base (79 de 104) y el índice más el grafo de hoy (81 de 104). Para medir la búsqueda del chat sin copiarla, una prueba ignorada del repositorio la corre sobre las preguntas que le pasa una variable de entorno (sin ningún caso en el código).
- **Por qué:** fijar el piso que la búsqueda desde la base (N5) tiene que superar y contar los documentos perdidos con números.
- **Alternativas descartadas:** reimplementar la búsqueda del wiki fuera del repositorio (podía divergir de la real); contar como encontrado todo documento que figure en cualquier página devuelta (una página de entidad lista decenas: se informa aparte como cota).
- **Dónde se aplica:** `src-tauri/src/commands/search.rs` (`search_questions_from_env`, ignorada); fuera del repositorio `desatendido-4/N1-medir-busqueda.py` y `N1-linea-de-base.md`; P-142, P-150.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N1 de la tanda desatendida del 18 oct.

### D-310 · Cada importe lleva su estado, leído por el modelo y listado en el vocabulario

- **Fecha:** 18 oct 2026
- **Tema:** Capa de conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** El tipo de un importe mezclaba qué es con qué pasó con él: «cuota» quería decir «cuota pagada», y no había forma de decir que un importe era un ejemplo, un precio pactado, una propuesta, un tope o una deuda a vencer. Las páginas decían «pagó» de cuotas que vencían meses después del documento (P-136).
- **Qué se decidió:** Al importe se le agrega un estado, que la lectura pregunta con su lista cerrada del vocabulario (`money.states`): pagado, a pagar, pactado, propuesto, condicionado y ejemplo (más «no lo dice»). El tipo deja de decir el estado («cuota» ya no es «cuota pagada») y se suman dos tipos: bonificación o crédito (un descuento o crédito promocional, que no es un pago ni una inversión) y total acumulado (la suma de otros importes del documento, que no es un pago aparte). El estado va en el detalle de la evidencia y de la afirmación; las páginas lo muestran junto al tipo y la síntesis tiene la regla de decirlo y de no decir «pagó» de lo que no dice «pagado».
- **Por qué:** Es la información que un abogado necesita de un importe (debe, pagó, se pactó, es un ejemplo) y estaba mal en 13 de los 16 casos de aceptación. Lista en el vocabulario (R-26), no en el código.
- **Alternativas descartadas:** Deducir el estado solo por palabras de la cita (no distingue un recibo de un acuerdo de pago en cuotas); cambiar solo las descripciones de los tipos (seguían mezclando dos cosas).
- **Dónde se aplica:** `src-tauri/resources/catalog/knowledge.json` (`money.states`, `money.kinds`); `knowledge/extract.rs`, `claims.rs`, `pages.rs`; P-136.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2a de la tanda desatendida del 18 oct.

### D-311 · Un importe que el documento da como ejemplo no se guarda como relación entre las partes

- **Fecha:** 18 oct 2026
- **Tema:** Capa de conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Una propuesta comercial explicaba con un ejemplo cómo se facturaría («se factura de la siguiente manera: USD …»); la lectura lo guardó como una factura entre las partes y la página decía «Facturó a …». Los importes además sirven para agrupar asuntos (R-05: los mismos importes).
- **Qué se decidió:** Un importe con estado de ejemplo (`notAFact` en el vocabulario) se descarta de la lectura con el motivo «no-es-un-hecho», como un impuesto o una cita que no está en el texto. Queda contado en los descartes del documento.
- **Por qué:** Un ejemplo no dice nada de las partes (R-01): guardarlo como relación lo muestra en sus páginas y puede unir asuntos por un importe que nunca existió.
- **Alternativas descartadas:** Guardarlo con el estado «ejemplo» (seguiría en las páginas de las partes y en la agrupación por importes).
- **Dónde se aplica:** `knowledge/extract.rs` (`amount_evidence`); `knowledge.json`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2a de la tanda desatendida del 18 oct.

### D-312 · Regla de fecha: un pago con fecha posterior al documento no está pagado

- **Fecha:** 18 oct 2026
- **Tema:** Capa de conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** El modelo puede decir «pagado» de un pago que el documento programa para más adelante (una cuota que vence meses después de la firma).
- **Qué se decidió:** Después de la lectura, el programa compara la fecha del importe con la fecha del documento: si el estado es de pago hecho (`paid`) y la fecha del importe es con seguridad posterior, el estado pasa a `dueState` (a pagar) y se cuenta como corrección («vence-despues-del-documento»). Las fechas se leen como el intervalo de días que pueden ser: un mes sin día es el mes entero y una fecha numérica con día y mes en orden desconocido cubre las dos lecturas; si los intervalos se tocan, no se cambia nada.
- **Por qué:** Es una regla general y verificable sin el modelo; ante la duda no corrige (una cuota de «febrero» en un documento del 26 de febrero no se toca).
- **Alternativas descartadas:** Confiar solo en el modelo; comparar fechas con un orden de día y mes supuesto.
- **Dónde se aplica:** `knowledge/extract.rs` (`due_after_the_document`), `knowledge/values.rs` (`date_span`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2a de la tanda desatendida del 18 oct.

### D-313 · Una afirmación de importe con estados distintos según el documento los muestra todos

- **Fecha:** 18 oct 2026
- **Tema:** Capa de conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** La misma cuota, entre las mismas partes, aparece como «a pagar» en un acuerdo y como «pagada» en un recibo posterior; la afirmación es una sola (mismo importe, partes y tipo) y mostraba el estado de la primera evidencia.
- **Qué se decidió:** La afirmación guarda el estado si todas sus evidencias dicen el mismo y, si difieren, la lista de estados (`estados`); la página los muestra juntos («a pagar o pagado»). El estado no entra en la clave de la afirmación (el índice único y los asuntos usan el tipo).
- **Por qué:** No esconder un dato que un documento dice; separar afirmaciones por estado habría cambiado el índice único y la agrupación de asuntos.
- **Alternativas descartadas:** Separar una afirmación por estado.
- **Dónde se aplica:** `knowledge/claims.rs`, `knowledge/pages.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2a de la tanda desatendida del 18 oct.

### D-314 · Una casilla marcada se escribe con «(marcada)» y su pregunta por la primera oración

- **Fecha:** 18 oct 2026
- **Tema:** Base documental
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** En las fichas de cliente, el texto guardado en la base no decía qué casilla estaba marcada: la extracción que lo dice existía desde el 14 de octubre, pero los textos se habían extraído antes. Además, la lectura y el wiki tomaban el rótulo impreso («SI NO») como respuesta (P-135).
- **Qué se decidió:** Cada casilla marcada u opción elegida se escribe en la sección de campos del formulario con su rótulo seguido de «(marcada)», y una pregunta impresa que sigue con instrucciones («… de la Operación. En caso negativo, …») se nombra por su primera oración. La consigna de la lectura dice que las opciones impresas son rótulos y que, si el texto no dice cuál está marcada, no se afirma la respuesta.
- **Por qué:** Que nadie (persona ni modelo) confunda el rótulo impreso de una opción con la respuesta, y que la pregunta y la respuesta queden juntas.
- **Alternativas descartadas:** Escribir solo la opción (no distingue un valor tipeado de una casilla marcada).
- **Dónde se aplica:** `src-tauri/src/commands/pdf_form.rs`; `knowledge/extract.rs` (consigna).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2a de la tanda desatendida del 18 oct.

### D-315 · Las marcas de un formulario aplanado se leen por su lugar en la tabla, y si no se puede se dice

- **Fecha:** 18 oct 2026
- **Tema:** Base documental
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Un formulario completado y después firmado con un servicio de firma electrónica queda sin campos: cada casilla marcada es un «✔» impreso donde estaba la casilla, y el texto los trae sueltos al final de la página, sin decir a qué pregunta corresponden.
- **Qué se decidió:** Cada glifo de marca (✔ ✓ ☑ ☒ ✗ ✘) de una página sin casillas se lee como una casilla marcada en su lugar: su opción es el texto de la celda vecina de la tabla del lado donde todas las opciones de esa fila tienen su casilla (una celda vacía), usando las líneas verticales de la tabla; la pregunta es lo impreso antes de las opciones, en las líneas alrededor. Sin tabla, vale el único rótulo al alcance. Si no se puede saber, el campo dice «marcada, sin saber cuál de las opciones» y nombra las opciones posibles: nunca se elige una adivinando.
- **Por qué:** Es determinista, general y no inventa: en la carpeta de prueba lee bien las tres preguntas de la ficha aplanada; donde la geometría no alcanza, se abstiene (P-135 pide no afirmar lo que no se sabe).
- **Alternativas descartadas:** Leer la imagen con el modelo (queda para las fichas escaneadas, P-160); tomar el rótulo más cercano (en la ficha de prueba elegía la opción equivocada).
- **Dónde se aplica:** `src-tauri/src/commands/pdf_form.rs` (`tick_fields`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2a de la tanda desatendida del 18 oct.

### D-316 · Versión de la extracción de texto: lo extraído con una versión vieja se revisa una vez

- **Fecha:** 18 oct 2026
- **Tema:** Base documental
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** El texto de un documento se extrae una sola vez; cuando la extracción mejora (leer las casillas), los documentos ya extraídos se quedan con el texto viejo para siempre.
- **Qué se decidió:** La base guarda la versión de la extracción con la que están sus textos (`docbase_setting`). Si es anterior a la del programa, el trabajador revisa de a poco los documentos extraídos sin reconocimiento: a un PDF cuyos campos de formulario ahora se leen distinto lo vuelve a extraer (su texto cambia y lo que depende de él se rehace); a los demás no los toca. Al terminar guarda la versión nueva. Una base sin textos queda en la versión actual. La evaluación hace lo mismo antes de leer.
- **Por qué:** Que una mejora de la extracción llegue a lo que ya está en la base sin volver a extraer todo el archivo.
- **Alternativas descartadas:** Volver a extraer todos los textos (cambia la fecha de todos y rehace parecidos); dejarlo a mano.
- **Dónde se aplica:** `src-tauri/src/docbase/text.rs` (`refresh_extraction`), `worker.rs`, `knowledge/check.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2a de la tanda desatendida del 18 oct.

### D-317 · Una definición con forma jurídica es un nombre, no un término; los términos se reconocen aunque tengan marcas de énfasis

- **Fecha:** 18 oct 2026
- **Tema:** Capa de conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Al releer, el modelo dio como «término definido» un nombre de sociedad entre paréntesis después de una marca comercial, y el programa unió las dos menciones sin mirar los nombres (una unión falsa); en otro documento un término definido entre marcas de negrita («(**“X**”)») no se reconoció como término y se unió por nombre a otra organización.
- **Qué se decidió:** Una definición cuyo término tiene forma jurídica (S.A., LLC…) se descarta («termino-con-forma-juridica») y la unión de las dos menciones queda sujeta a la comprobación de nombres (D-281). La forma de un término definido ignora los asteriscos y guiones bajos de énfasis.
- **Por qué:** R-07: ante la duda no se une. Con los dos cambios, las uniones falsas vuelven a 0.
- **Alternativas descartadas:** Volver a preguntar esos documentos hasta que el modelo no lo haga (no corrige la causa).
- **Dónde se aplica:** `knowledge/extract.rs` (`define_terms`), `knowledge/read_checks.rs` (`defined_terms`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2a de la tanda desatendida del 18 oct.

### D-318 · Medición de N2a: releer solo los documentos afectados reutilizando las respuestas de los demás

- **Fecha:** 18 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** La consigna nueva cambia la pregunta de todos los documentos, pero solo puede cambiar el resultado de los que tienen importes o un texto nuevo; releer todo (75 preguntas) no estaba pedido y mezcla la variación del modelo.
- **Qué se decidió:** Para medir, fuera del repositorio, se releyeron 40 documentos (48 preguntas: 37 con importes en su lectura anterior y las 3 fichas cuyo texto cambió) y a los otros 26 se les copió la respuesta anterior bajo la pregunta nueva, marcada como trasplante (`N2a-trasplante.py`). Una prueba ignorada del repositorio lista las preguntas nuevas de una copia de la base. La base resultante no se usa en la app.
- **Por qué:** Medir el cambio con el menor ruido y costo, como pide la especificación.
- **Alternativas descartadas:** Releer todo el archivo (lo hace N3 con su lectura completa).
- **Dónde se aplica:** Fuera del repositorio: `desatendido-4/N2a-trasplante.py`, `N2a-correr.sh`; `knowledge/check.rs` (`reading_questions_to_a_file`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2a de la tanda desatendida del 18 oct.

### D-320 · Cada afirmación lleva su certeza: con cita comprobada, confirmada, dudosa o corregida

- **Fecha:** 18 oct 2026
- **Tema:** Capa de conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Que la cita de una afirmación esté en el documento no prueba que la lectura sea correcta (D-293): el modelo puede atribuir un dato a quien no es, la identidad del sujeto puede estar en duda o dos documentos pueden decir cosas distintas. Las páginas y las pantallas mostraban todo igual, como hecho firme.
- **Qué se decidió:** Cada afirmación tiene una certeza calculada en cada resolución, sin el modelo, desde lo que ya guarda la base: **con cita comprobada** (la cita está en el texto y nadie la confirmó), **confirmada** por una persona (decisión nueva «confirmar», que se deshace como las otras), **dudosa** (motivos: los documentos difieren, que es el conflicto de siempre; la identidad del sujeto o del objeto está en duda en todas sus evidencias, por las dudas de identidad; la cita nombra a otra persona u organización del documento y no a su sujeto, en todas sus evidencias (una dirección de correo no nombra a la organización de su dominio, y un apellido que comparten no nombra al sujeto); los documentos dicen estados distintos de un importe) y **corregida** (una persona marcó como incorrecta una de sus evidencias o unió o separó una de sus menciones). Gana la confirmación; una duda gana sobre una corrección. Se guarda como resumen calculado en `knowledge_claim.certainty` (nivel y motivos) para que páginas, pantallas y chat lean lo mismo; la confirmación es un tipo más de `knowledge_decision` (migración V15).
- **Por qué:** El dueño pidió que cada afirmación lleve certeza y quién la confirmó, y que lo dudoso nunca se presente como hecho firme. Las señales son generales (R-24) y reutilizan el estado de la afirmación, las dudas y las decisiones; la única columna nueva es el resumen, para no recalcularlo en cuatro lectores.
- **Alternativas descartadas:** Calcularla al mostrar en cada pantalla (tres copias de la misma regla); ampliar `knowledge_claim.state` (mezcla el conflicto entre documentos con la palabra de una persona y obliga a reconstruir la tabla); preguntarle al modelo cuán seguro está (no es verificable).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/claims.rs` (`certainty_json`, `names_someone_else`, `doubted_mentions`), `decide.rs` (`Confirm`), `schema.rs` (V15), `entities.rs`; `src/components/knowledge/entity-detail.tsx`, `knowledge.ts` (`certaintyInWords`); P-142.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2b de la tanda desatendida del 18 oct.

### D-321 · Las decisiones de una persona se colocan sobre lo que dicen los documentos en cada resolución; lo que ya no encuentran queda para revisar

- **Fecha:** 18 oct 2026
- **Tema:** Capa de conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Una decisión (unir, separar, marcar incorrecto) nombra un grupo de menciones por su documento y su nombre, o una evidencia por su documento, sujeto, predicado y objeto. Al releer un documento con otra consigna, el modelo puede escribir el mismo nombre o valor de otra forma («Nueve Ejemplo» por «Persona Nueve Ejemplo», «Calle Siete 700» por «Calle Siete 700 piso 3»): la decisión dejaba de aplicarse sin aviso (una separación se perdía y las dos menciones volvían a unirse por su identificador). La prueba nueva lo mostró fallando antes del arreglo.
- **Qué se decidió:** Antes de cada resolución cada decisión activa se coloca: **se aplica** (lo que nombra está, o está en el documento que tomó el lugar del suyo); **se movió** (no está igual, pero en su documento hay una sola cosa del mismo tipo con un nombre que puede ser el mismo, o una sola evidencia del mismo predicado con el mismo valor escrito de otra forma: la decisión sigue con ella y se guarda a qué se aplica, `applied_as`); **espera** (su documento no está o no se leyó todavía: vuelve sola cuando vuelve el documento); **para revisar** (el documento está leído y lo que nombraba no aparece, o aparece más de una vez). Nunca se aplica a una adivinanza ni se pierde en silencio: Atención muestra «decisión tuya ya no encuentra lo que decidía» con el documento y un botón para deshacerla, y la pantalla de la entidad lo dice en su lista de decisiones. Las decisiones sobre afirmaciones se colocan igual, siguiendo además a la entidad en la que se unió la suya.
- **Por qué:** Pedido del dueño (las decisiones tienen que sobrevivir a una relectura, D-293) y R-08/R-09: lo que decide una persona no lo pisa un cálculo, y si deja de aplicarse alguien tiene que saberlo.
- **Alternativas descartadas:** Guardar las decisiones por identificador de mención (cambian en cada lectura); aplicar la decisión al candidato más parecido aunque haya varios (puede unir o separar a quien no es); borrar las decisiones que no encuentran nada.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/anchors.rs` (nuevo), `resolve.rs` (`load_decided`), `claims.rs` (`place_claim_decisions`), `screens.rs` (`decisions_to_review`); `src/components/attention/sources/knowledge.ts`; prueba `tests_survive.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2b de la tanda desatendida del 18 oct.

### D-322 · La síntesis y las páginas escriben lo dudoso como dudoso; se dice «con cita comprobada»

- **Fecha:** 18 oct 2026
- **Tema:** Capa de conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** La síntesis del modelo redactaba todas las afirmaciones como hechos, y las páginas no distinguían un dato dudoso de uno firme.
- **Qué se decidió:** En la pregunta de la síntesis, cada afirmación dudosa por un motivo que no sea solo el conflicto (que ya tiene su bloque y su regla) lleva «DUDOSA (motivo)», y solo en esas preguntas se agrega la regla de escribirlas como dudosas («según un documento…, sin confirmar»), en oraciones aparte y nunca como algo seguro: las demás entidades conservan su pregunta y su respuesta. Además, un cálculo marca «(sin confirmar)» toda oración que se apoye en una afirmación dudosa y no lo diga. Las cifras del nombre del documento de una afirmación respaldan la oración que lo nombra («según «Carta 26-06-26.pdf»»): sin eso, la comprobación de números descartaba justo las oraciones dudosas. En la página, cada dato listado lleva su marca («sin confirmar: motivo», «confirmado por una persona», «corregido por una persona»), también en los vínculos y en lo que dicen los documentos de un asunto, y debajo de la síntesis una línea explica que cada dato tiene su cita comprobada, que eso no garantiza que la lectura sea correcta y qué quiere decir «sin confirmar». En la interfaz se dice «con cita comprobada», nunca «verificado» (también en el origen del tipo de un documento: «marcas comprobadas»). Las palabras están en el vocabulario de las páginas.
- **Por qué:** Pedido del dueño (D-293): lo dudoso nunca como hecho firme. La marca calculada lo garantiza aunque el modelo no siga la regla; limitar la regla a las preguntas con dudas evita volver a pedir todas las síntesis por un cambio que no les toca.
- **Alternativas descartadas:** Cambiar la consigna de todas las síntesis (las mismas respuestas, todas preguntadas de nuevo); sacar las afirmaciones dudosas de la síntesis (se perderían datos que pueden ser ciertos; siguen listadas con su marca).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/pages.rs` (`synthesis_question`, `render`, `certainty_mark`, versión de proyección 8), `resources/catalog/knowledge.json` (`pages.words`); `src/components/docbase/document-types.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2b de la tanda desatendida del 18 oct.

### D-323 · Medición de N2b: sin preguntar salvo las síntesis cuya pregunta cambió

- **Fecha:** 18 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** La certeza no cambia lo que se lee ni cómo se resuelven las identidades, pero sí la pregunta de la síntesis de las entidades con datos dudosos y el texto de todas las páginas.
- **Qué se decidió:** Se midió sobre una copia de la base de N2a con las respuestas guardadas; solo se preguntó a `gpt-6-luna` alto la síntesis de las entidades cuya pregunta cambió (5 en la primera medición y 2 que faltaban después del ajuste del correo: 7 preguntas en total), por `con-modelo.sh`. Las páginas antes y después de esas entidades quedaron fuera del repositorio (`desatendido-4/N2b-muestra/`).
- **Por qué:** Lo pide la especificación de la etapa: medir el cambio sin ruido del modelo en lo que no cambió.
- **Alternativas descartadas:** Volver a escribir todas las síntesis (las demás preguntas no cambiaron).
- **Dónde se aplica:** Fuera del repositorio: `desatendido-4/N2b-correr.sh`, `N2b-muestra.py`, `N2b-medicion.md`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N2b de la tanda desatendida del 18 oct.

### D-330 · El resumen del documento sale de la lectura de la capa: tema y puntos, cada punto con su cita comprobada

- **Fecha:** 18 oct 2026
- **Tema:** Capa de conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** El resumen de cada documento lo escribía el wiki en una segunda y tercera lectura con el modelo (análisis y generación), sin cita y sin comprobación, mientras la capa ya leía el documento entero (P-142, principio 2: una extracción principal).
- **Qué se decidió:** La pregunta de la lectura (versión `conocimiento-4`) pide además un `resumen`: un `tema` (una oración: qué documento es, entre quiénes y para qué) y de dos a seis `puntos` con lo esencial, cada uno con una cita copiada del texto, su página y las menciones que nombra; lo abierto (borrador, propuesta, sin firmar, condicionado) se dice así; en castellano aunque el documento esté en otro idioma. Un cálculo conserva un punto solo si su cita está en el texto y todos sus números están en su cita; un tema solo si sus números están en el texto. Lo descartado se cuenta con motivos propios (`resumen-…`) que no cuentan como lectura ruidosa. Se guarda en la base (`knowledge_summary`, migración V16) con la lectura y se reemplaza con ella: cada punto apunta a las menciones que nombra y a las evidencias del mismo pasaje (una cita contiene a la otra o comparten 8 palabras seguidas), y por ellas a las afirmaciones y su certeza. Un documento largo da un tema (el de la primera parte que lo da) y hasta 10 puntos de todas sus partes.
- **Por qué:** Pedido del dueño (D-293, una lectura por contenido y configuración) y R-17: medido sobre la lectura completa de los 66 documentos, 66 resúmenes (289 puntos, 4,4 por documento; 38 puntos descartados por el cálculo: 26 con números fuera de su cita, 12 con cita que no está). Revisor a ciegas sobre 10: los 10 fieles; completitud 1,9 de 3 y utilidad 2,4 de 3 (faltan importes y fechas clave: P-171).
- **Alternativas descartadas:** Una pregunta aparte para el resumen (otra lectura del mismo documento); resumir desde las afirmaciones ya guardadas (pierde lo que no es una afirmación: de qué trata, qué se pide); guardar el resumen sin cita.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/extract.rs` (`prompt`, `schema`, `summarize`, `same_passage`, `write_reading`), `src-tauri/src/docbase/schema.rs` (V16); pruebas `tests_reading.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N3 de la tanda desatendida del 18 oct.

### D-331 · La página de cada documento la escribe la base desde el resumen, sin modelo, hasta que N4 la arme como página del documento

- **Fecha:** 18 oct 2026
- **Tema:** Wiki
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
- **El problema:** Si el wiki deja de leer documentos con el modelo, nadie escribe la página de cada documento.
- **Qué se decidió:** La proyección de la capa escribe el texto de cada página de documento del wiki del archivo: título (el de su encabezado o el nombre del archivo), tema, tipo y etapa, fecha del documento, «Lo esencial» (cada punto con la cita que lo respalda, su página y la marca de certeza de las afirmaciones del mismo pasaje: «sin confirmar», «confirmado», «corregido»), «Importes entre las partes» (los importes que el registro guarda de ese documento, con su estado y certeza, aunque el resumen no los diga) y la línea que explica qué es una cita comprobada. Conserva el encabezado de la página (sus fuentes) con la marca `generated_by: micelya-conocimiento` y quita `tags` y `related` que había escrito el modelo; las notas de una persona se conservan y el bloque de entidades queda al final. Una página que escribió la incorporación anterior queda como está hasta que su documento se lee con la pregunta nueva. Versión de proyección 9.
- **Por qué:** D-293 y la decisión abierta 1 resuelta (D-294): página del documento como archivo generado por ahora. Los importes se agregan sin modelo porque el revisor a ciegas marcó que los resúmenes los omiten.
- **Alternativas descartadas:** Dejar la página del modelo hasta N4 (seguiría habiendo dos lecturas); escribirla desde TypeScript (no sabe cuándo terminó la lectura).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/pages.rs` (`document_page`, proyección), `resources/catalog/knowledge.json` (`pages.words`: `docSummary`, `docKind`, `docDate`, `docPending`, `docQuote`, `docAmounts`); prueba `tests_pages.rs`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N3 de la tanda desatendida del 18 oct.

### D-332 · En el wiki del archivo la incorporación no llama al modelo: solo crea la página del documento para que la base la complete

- **Fecha:** 18 oct 2026
- **Tema:** Wiki
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
- **El problema:** La incorporación heredada leía cada documento con el modelo 2 o 3 veces (análisis, generación y revisión, más fragmentos, combinación de páginas y reparación de truncados): escribía la página de fuente, conceptos por documento, consultas inventadas y avisos de revisión.
- **Qué se decidió:** En el wiki del archivo (el proyecto general) la incorporación hace el reconocimiento de texto (sin cambios), crea la página del documento si no existe (encabezado con su fuente y la marca de la base; si ya existe no la toca), actualiza índice y registro sin modelo y guarda su caché. No hay análisis, generación, revisión, conceptos, consultas, avisos de revisión, combinación con el modelo ni descripción de imágenes. Una respuesta del chat guardada en ese wiki se guarda tal cual, sin releerla con el modelo. Los demás proyectos conservan la incorporación anterior.
- **Por qué:** Principio 2 de la propuesta y especificación de N3. Medido: llamadas del modelo por documento al leer, antes unas 3,1 a 4,1 (2 o 3 del wiki, unas 200 en total, más 75 de la capa para 66 documentos) y ahora 1,14 (75 preguntas para 66 documentos: una lectura, en partes los largos), 0 del wiki.
- **Alternativas descartadas:** Borrar todo el código de la incorporación anterior (los otros proyectos quedarían sin wiki; queda como P-172); conservar la descripción de imágenes (escribía en la página, no en la base: P-173).
- **Dónde se aplica:** `src/lib/ingest.ts` (`ingestIntoArchiveWiki`, `buildArchiveSourcePage`), `src/components/chat/chat-message.tsx`; prueba `src/lib/ingest-knowledge-layer.test.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N3 de la tanda desatendida del 18 oct.

### D-333 · Se quitan la revisión del wiki con el modelo y el enriquecimiento de enlaces; la limpieza de avisos con el modelo no corre en el wiki del archivo

- **Fecha:** 18 oct 2026
- **Tema:** Wiki
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
- **El problema:** La revisión de enlaces tenía una parte con el modelo (contradicciones, páginas que faltan) y la limpieza de avisos de revisión preguntaba al modelo; el enriquecimiento de enlaces con el modelo ya no tenía quién lo llamara.
- **Qué se decidió:** Se quita la revisión con el modelo (la opción de la pantalla y su código); queda la estructural, sin modelo. Se borra el enriquecimiento de enlaces (código muerto). La limpieza de avisos de revisión con el modelo no corre en el wiki del archivo: lo pendiente queda para una persona. Las contradicciones y los duplicados los detecta la capa con evidencia y van a Atención.
- **Por qué:** Sección 4 de la propuesta.
- **Alternativas descartadas:** Dejar la revisión con el modelo como opción (lee páginas que ya salen de la base y repite lo que la capa hace con evidencia).
- **Dónde se aplica:** `src/lib/lint.ts`, `src/components/lint/lint-view.tsx`, `src/lib/sweep-reviews.ts`; borrados `src/lib/enrich-wikilinks*.ts` y `lint.real-llm.test.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N3 de la tanda desatendida del 18 oct.

### D-334 · La investigación en internet guarda su resultado fuera del wiki del archivo

- **Fecha:** 18 oct 2026
- **Tema:** Wiki
- **Estado:** reemplazada por D-397
- **El problema:** La investigación en internet escribía su resultado como página de consulta del wiki y lo indexaba para la búsqueda, mezclando conocimiento externo con el del archivo (D-294).
- **Qué se decidió:** En el wiki del archivo el resultado se guarda en `.llm-wiki/research/` y no se indexa; la herramienta sigue disponible a pedido. En otros proyectos, como antes.
- **Por qué:** D-294: herramienta aparte que nunca escribe en la base ni en el wiki del archivo.
- **Alternativas descartadas:** Quitar la herramienta (la persona la puede querer a pedido).
- **Dónde se aplica:** `src/lib/deep-research.ts`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N3 de la tanda desatendida del 18 oct.

### D-335 · Medición de N3: lectura completa desde la base de partida, sin trasplantes

- **Fecha:** 18 oct 2026
- **Tema:** Proceso de trabajo
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** La pregunta de la lectura cambió para todos los documentos: hacía falta leerlos todos y medir sin las respuestas trasplantadas de N2a.
- **Qué se decidió:** Una lectura completa de los 66 documentos con `gpt-6-luna` alto desde una copia de `base-partida.sqlite3` y un caché vacío (75 preguntas de lectura, 27 de identidad, 43 síntesis; 18 minutos), por `con-modelo.sh`; luego dos corridas sin preguntar con los cambios de páginas. La base queda como `desatendido-4/N3-base.sqlite3` con su caché `N3-cache.json`, base de N4 a N6. Resultados: casos críticos 29 → 33 de 46 (importes 13 → 16 de 16, atribución 3 → 4 de 8, casillas 9 de 9, partidas 3 → 2, regresiones 1 → 2); uniones falsas 0; separaciones falsas 201 → 185; menciones de la hoja 370 → 373; afirmaciones de la hoja 69 → 66. Atribución a ciegas (40 y 40 al azar, mezcladas): 35 → 34 correctas (88 % → 85 %; dinero 10 → 12 de 14, domicilios 13 → 10 de 13).
- **Por qué:** R-17 y la especificación de la etapa.
- **Alternativas descartadas:** Medir sobre las bases de N2a/N2b (tienen respuestas trasplantadas); otra lectura completa para ajustar el resumen (excede las lecturas autorizadas de la tanda: queda para N6, P-171).
- **Dónde se aplica:** Fuera del repositorio: `desatendido-4/N3-correr.sh`, `N3-resumenes.py`, `N3-juicio/`, `N3-resumenes/`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N3 de la tanda desatendida del 18 oct.

### D-340 · Una página por documento o por familia de versiones, escrita por la base

- **Fecha:** 18 oct 2026
- **Tema:** Wiki
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
- **El problema:** Cada archivo tenía su página de documento: una copia en Word y en PDF, un borrador y su ejemplar firmado eran páginas distintas que repetían lo mismo, y la página dependía de que la incorporación la hubiera creado. Lo acordado con el dueño (D-293, punto 4) es una página común solo para versiones del mismo documento, y que cada dato diga de qué versión sale.
- **Qué se decidió:** La proyección de la capa escribe la página de cada documento del wiki del archivo, haya o no página previa. Los documentos unidos por una relación de `pages.versionRelations` (mismo documento, prácticamente el mismo, ejemplar firmado, versión, vista previa), propuesta por el cálculo de parecidos —de certeza alta o baja— o confirmada por una persona, y nunca descartada, comparten una página (también dos que una persona unió eligiendo cuál lee el wiki). La página lleva el nombre del archivo de la versión elegida por la persona, o de la que es sujeto de una relación de `pages.principalRelations` (el ejemplar firmado), o de la primera; su identificador es `doc-<número del primer contenido>`, y su archivo es el que ya tenía (o la página vieja de una de sus versiones, para que no se rompan enlaces ni notas) o uno nuevo con su título. Contenido, solo desde la base: título, tema del resumen, tipo, etapa y fecha, el bloque de personas, organizaciones y asunto con su rol (enlazados), las versiones con su tipo, fecha, ubicaciones y relación entre ellas (con «propuesta por el cálculo de parecidos, sin confirmar» o «confirmada por una persona»), las ubicaciones (si es una sola versión), los datos del tipo, «Lo esencial», los importes del registro, los documentos relacionados que no son versiones (misma plantilla, contenido en, constancia de firma…) enlazados a su página, y las notas. Los documentos de la misma plantilla son páginas distintas. Proyección versión 10; el cálculo de lo que cambió incluye las familias, resúmenes, tipos y datos.
- **Por qué:** Principios 1 y 3 de la propuesta, D-293 y D-294. Medido sobre `N3-base` contra la hoja de parecidos: con solo certeza alta, 16 de 19 pares de «mismo documento» quedan juntos; con certeza alta y baja, 18 de 19, y en los dos casos 0 de 60 pares distintos unidos. La certeza baja se muestra en la página y una persona la descarta con la acción de parecidos que ya existe (las versiones vuelven a páginas separadas).
- **Alternativas descartadas:** Solo relaciones de certeza alta (dos pares de la hoja quedan partidos); incluir «contenido en» y «misma plantilla» (no son versiones: D-293); páginas armadas al mostrarse (D-294: archivo generado por ahora).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/pages/family.rs` (familias, ruta, texto), `pages.rs` (proyección, bloque de varias versiones), `resources/catalog/knowledge.json` (`pages.documentsFolder`, `versionRelations`, `principalRelations`, palabras `doc…` y `rel-…`), `vocabulary.rs`; pruebas `tests_pages.rs`. Reemplaza en parte D-331 (ya no se conserva la página vieja hasta la relectura; el bloque va arriba, debajo de lo que es el documento).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N4 de la tanda desatendida del 18 oct.

### D-341 · Cada punto, dato e importe de una página de versiones dice de qué versión sale

- **Fecha:** 18 oct 2026
- **Tema:** Wiki
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
- **El problema:** En una página común de versiones hay que ver de dónde sale cada cosa, y si dos versiones difieren en un dato, ver los dos (D-293).
- **Qué se decidió:** Las versiones se nombran por su etapa (con un número si dos tienen la misma) o «versión». Un punto del resumen aparece una vez y lleva entre paréntesis las versiones cuyo texto tiene su cita, salvo que estén todas; un punto de otra versión del mismo pasaje no se repite. Los datos del tipo van en una línea por campo con cada valor y sus versiones; los importes, con las versiones de su evidencia. Una nota bajo la lista de versiones explica que lo que no dice versión está en todas.
- **Por qué:** D-293, punto 4: si las versiones dicen cosas distintas se muestran las dos.
- **Alternativas descartadas:** Un resumen completo por versión (repite casi todo, porque las versiones comparten casi todo el texto); mostrar solo la versión principal (esconde lo que cambia).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/pages/family.rs` (`render_document`); prueba `the_versions_of_a_document_share_a_page_and_say_which_version_says_what`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N4 de la tanda desatendida del 18 oct.

### D-342 · La página de un documento solo tiene lo que sabe la base y las notas; las páginas viejas se absorben

- **Fecha:** 18 oct 2026
- **Tema:** Wiki
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
- **El problema:** El wiki tenía una página por archivo escrita por la incorporación anterior, con texto del modelo, y la incorporación nueva crea una página mínima por archivo. Había que pasar a una página por documento sin perder notas ni enlaces, y comprobar que nada en el wiki diga algo que no sale de la base (principio 1).
- **Qué se decidió:** En cada proyección, una página de documento del wiki que no es la de su familia (la página vieja de una copia o versión, la página mínima que crea la incorporación para otra copia, la de una versión que se unió a otra) se borra: sus notas pasan a las notas de la página del documento y los enlaces a ella se redirigen ahí. El texto que no es notas se reescribe siempre con lo que dice la base: un cambio a mano fuera de las notas se pierde en la proyección siguiente. La incorporación sigue creando la página mínima de un archivo nuevo (así el documento tiene página enseguida); si es otra copia de un documento que ya tiene página, la base la absorbe.
- **Por qué:** Principio 1 y especificación de N4 (migración sin perder notas, prueba automática de redundancia). Medido sobre `N3-base` con el wiki real de partida copiado: 66 páginas de documento → 59 (6 con versiones, 7 absorbidas, 9 enlaces redirigidos), 0 páginas no escritas por la base y 0 reescritas al proyectar otra vez. Prueba `a_document_page_holds_only_what_the_base_knows_and_the_notes`.
- **Alternativas descartadas:** Conservar el texto viejo como nota (mezclaría texto del modelo con las notas de una persona para siempre); que la incorporación deje de crear la página mínima (el documento quedaría sin página hasta la proyección y cambiaba más código de TypeScript con el ciclo de vida de los archivos).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/pages.rs` (`project`), `check_pages.rs` (medición), `src/lib/ingest.ts` (comentario de `ingestIntoArchiveWiki`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N4 de la tanda desatendida del 18 oct.

### D-343 · El resumen de la lectura trae importes, fechas clave y números, y en una constancia de firma qué se firmó

- **Fecha:** 18 oct 2026
- **Tema:** Capa de conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** El revisor a ciegas de N3 encontró los resúmenes fieles pero incompletos (completitud 1,9 de 3): faltaban totales, fechas de emisión y vencimiento, el número de la factura que se rechaza, y en una constancia de firma qué se firmó y quiénes (P-171).
- **Qué se decidió:** La consigna de la lectura (`conocimiento-5`) pide que los puntos traigan, tal como están escritos, los importes (el total y cómo se compone), las fechas clave y los números que identifican el documento, cada uno en un punto cuya cita lo tenga (dos datos en lugares distintos, dos puntos), que no diga «la factura mencionada» si el texto dice cuál, y que en la constancia de un servicio de firma electrónica diga qué documento se firmó, quiénes, cuándo y si quedó completo. Hasta ocho puntos. Todos los documentos se releen con la consigna nueva (en N6).
- **Por qué:** Medido sin lectura completa: 7 documentos releídos con `gpt-6-luna` alto sobre una copia de `N3-base` (los demás con su respuesta anterior trasplantada), revisor a ciegas (otro agente, sin la clave): fieles 7 de 7 las dos; completitud 1,71 → 2,57 de 3; utilidad 1,86 → 2,71; prefirió la nueva en 6 y empató en 1 (una boleta donde ninguna da total ni vencimiento). Casos críticos de esa copia 33 de 46 (igual). Las dos lecturas completas de N6 miden el resto.
- **Alternativas descartadas:** Agregar los importes al resumen por cálculo (ya están en la página desde el registro, D-331; el resumen es para quien no leyó el documento); una pregunta aparte para el resumen (rompe la lectura única).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/extract.rs` (consigna y `KNOWLEDGE_PROMPT`). Medición fuera del repositorio: `desatendido-4/N4-trasplante.py`, `N4-p171-comparar.py`, `N4-p171-juicio/`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N4 de la tanda desatendida del 18 oct.

### D-344 · La carta en Word y en PDF propuesta como «parecido» está bien: no se cambia la regla (P-210)

- **Fecha:** 18 oct 2026
- **Tema:** Base documental
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** N7 anotó que una carta en Word y otra en PDF con cobertura 0,94 quedaban como «parecido» y no como versión, y pidió revisar la regla de propuesta del cálculo de parecidos (P-210).
- **Qué se decidió:** No cambiar la regla. Revisado contra los textos y la hoja de parecidos: las dos cartas rechazan facturas distintas con fechas distintas; la hoja las tiene como «misma plantilla», confirmado por una persona. La regla del tipo («si cambia el número de factura son documentos distintos») acierta; el juicio de N7 fue el que se equivocó. Ninguno de los pares de «mismo documento» de la hoja queda mal por esta regla (18 de 19 juntos; el que falta es una vista previa que el cálculo propone como «contenido en», P-180).
- **Por qué:** R-17: un arreglo sin mejora medida no entra; acá empeoraría (uniría dos documentos distintos).
- **Alternativas descartadas:** Proponer versión por cobertura alta aunque cambie un dato que distingue documentos (uniría dos cartas sobre facturas distintas).
- **Dónde se aplica:** Ningún cambio de código. Medición: `desatendido-4/N4-familias.py`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N4 de la tanda desatendida del 18 oct.

### D-350 · La búsqueda del wiki del archivo sale de la base: el texto recorre todo lo visible y el grafo ordena y amplía, nunca recorta

- **Fecha:** 18 oct 2026
- **Tema:** Búsqueda
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** La búsqueda del chat leía las páginas del wiki y perdía un tercio de los documentos (61 de 104 esperados en los 10 primeros, D-301); el índice de texto de la base encontraba más (79) pero no hallaba documentos en inglés con preguntas en castellano ni números escritos con puntos (P-150). La propuesta (sección 7, D-293) pide que el grafo ordene y amplíe sin recortar y que toda consulta reciba lo visible.
- **Qué se decidió:** `DocBase::search_documents(visible, pregunta, límite, conjunto elegido)` fusiona por rango recíproco (k = 10) tres listas de la base, siempre restringidas a los documentos con una ubicación visible: (1) las palabras de la pregunta en el texto (bm25; las de seis letras o más también por su raíz, la palabra sin sus dos últimas letras); (2) sus números e identificadores en cualquier forma, en una columna nueva que guarda cada número escrito con puntos, guiones, barras, coma de miles o hasta dos letras como solo sus dígitos (y los importes con centavos también sin ellos); (3) sus palabras en el resumen de cada documento (tema y puntos, en el idioma del archivo). Después el grafo: las personas, organizaciones y asuntos que la pregunta nombra (por los nombres que el registro conoce) suben sus documentos y los suman aunque el texto no los halle, con menos peso si la entidad está en muchos documentos; un nombre que aparece en muchos más documentos que su entidad («acuerdo», «cliente») o una entidad presente en la mitad del archivo no cuentan. Los primeros cinco documentos suman con peso 0,03 los que la base relaciona con ellos (parecidos no descartados). El grafo nunca saca un documento; solo un conjunto que una persona elige a propósito (`within`) achica la búsqueda. Esquema V17: `content_search (numbers, summary)`, que se llena al abrir la base y se actualiza cuando cambian el texto o el resumen.
- **Por qué:** Medido con las 30 preguntas de N1 (104 esperados, en los 10 primeros, sobre la base de N4): solo texto 79; con números 88; con resumen 93; con raíz 94; con entidades y familias 94 (en los 5 primeros 73 → 78); con la ampliación 96 (en 5: 79). En la base de la consigna `conocimiento-5` da 95. El chat (páginas) pasa de 61 a 95 y la variante «el grafo recorta» perdería 4 esperados (1 en la otra base): no se recorta.
- **Alternativas descartadas:** Premiar cuántas palabras de la pregunta tiene el documento (bajó de 94 a 93); ampliar con más peso (0,1: 93; 0,2: 91, desplaza resultados de texto); el grafo de hoy (páginas proyectadas a documentos en los últimos lugares, 81); recortar a los documentos de las entidades nombradas (pierde 4); sinónimos o traducción de la pregunta (el resumen en el idioma del archivo alcanzó: las dos preguntas en otro idioma pasan de 0 a 1 de 1, sin modelo).
- **Dónde se aplica:** `src-tauri/src/docbase/search.rs`, `schema.rs` (V17), `text.rs`, `identity.rs`, `knowledge/extract.rs`; `src-tauri/src/commands/search.rs` (`search_from_base`). Medición fuera del repositorio: `desatendido-4/N5-proto.py`, `N5-busqueda.md`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N5 de la tanda desatendida del 18 oct.

### D-351 · El grafo y los enlaces del wiki del archivo son las relaciones de la base

- **Fecha:** 18 oct 2026
- **Tema:** Wiki
- **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
- **El problema:** El grafo y el panel de enlaces releían todas las páginas y convertían todo enlace en la misma flecha sin tipo (P-068); la propuesta (sección 6) pide que salgan de la base, con tipo, cita y estado, y reemplaza D-076 para lo que sale de la base.
- **Qué se decidió:** `DocBase::graph(visible)`: nodos = las páginas que escribe la base (documento o familia de versiones, persona, organización, grupo, asunto), con el nombre de archivo como id (el que ya usa la vista); aristas = un documento y las entidades que nombra (por rol), lo que dicen documentos visibles entre dos entidades (por predicado, con la certeza de la afirmación), dos documentos que el cálculo de parecidos relaciona (nunca los descartados), un asunto y sus documentos, un grupo y sus sociedades; cada una con su tipo, su etiqueta del vocabulario, una cita si la tiene y su estado. La vista del grafo (`wiki-graph.ts`) lo pide con `docbase_graph` y, si el proyecto no es el wiki del archivo, sigue con los enlaces de sus páginas; varias relaciones entre dos páginas son una arista con todas (su cantidad es el peso). El panel de enlaces de una página del wiki del archivo da salientes y entrantes desde esas relaciones (con la relación y la cita como texto), y los enlaces que una persona escribe en las notas se resuelven por la página que nombran o, si no, por un nombre que conoce el registro (una entidad renombrada se sigue encontrando); un enlace de notas que no nombra nada queda como faltante.
- **Por qué:** Un enlace existe solo si existe la relación; el resto de la página ya se escribe desde la base (D-342), así que no puede haber enlaces rotos fuera de las notas.
- **Alternativas descartadas:** Seguir con el índice de enlaces de las páginas (P-068 tal como estaba): mantiene dos verdades.
- **Dónde se aplica:** `src-tauri/src/docbase/graph.rs`, `commands.rs` (`docbase_graph`), `src-tauri/src/commands/search.rs` (`links_from_base`), `src/lib/wiki-graph.ts`, `src/components/graph/graph-view.tsx`.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N5 de la tanda desatendida del 18 oct.

### D-352 · El chat recibe una página por documento y la de la entidad más específica que nombra la pregunta

- **Fecha:** 18 oct 2026
- **Tema:** Búsqueda
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** El chat lee páginas, no documentos: hay que traducir lo que halla la base a páginas sin perder lugares.
- **Qué se decidió:** Cada documento hallado se contesta con la página de su familia de versiones (dos versiones ocupan un lugar); el último lugar es para la página de la persona, organización o asunto que la pregunta nombra con más especificidad (la que está en menos documentos), si tiene página. Los vínculos con entidades nombradas viajan en `graph_related_to`. Toda consulta usa el conjunto de lo visible; hoy todas las pantallas del espacio privado ven toda la base (`Visible::All`) y el espacio compartido lo calculará por persona en ese mismo lugar.
- **Por qué:** Medido con el binario de pruebas sobre la base y las páginas de N4: sin página de entidad 96 documentos en las páginas devueltas; con una 95 (y 101 contando las fuentes de toda página); con dos 93. Una página de entidad trae lo que el registro sabe de esa persona con sus citas.
- **Alternativas descartadas:** Dos páginas de entidad (pierde 3 documentos); ninguna (el chat tendría que abrir la página de la entidad por un enlace).
- **Dónde se aplica:** `src-tauri/src/commands/search.rs` (`search_from_base`, `NAMED_ENTITY_PAGES`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N5 de la tanda desatendida del 18 oct.

### D-360 · Un término que el documento define nombra una sola cosa en él

- **Fecha:** 18 oct 2026
- **Tema:** Capa de conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** En una lectura completa (N6), en un contrato el modelo dijo que la línea de notificaciones "Si a <término de la otra parte>:" era "la misma que" la sociedad propia. Una mención escrita como término definido salta el control de nombres (D-281), así que las dos partes del contrato quedaron en una sola entidad (16 pares de menciones unidos por error; antes de esta etapa siempre 0).
- **Qué se decidió:** Después de marcar los términos definidos, si una mención posterior del mismo término (misma clase) quedó enlazada por el modelo a otro grupo que el de la primera mención del término, pierde ese enlace y conserva solo su nombre, que ya la junta con la primera mención. Nunca se une algo que el nombre no juntaba.
- **Por qué:** Un término definido en un documento nombra una sola cosa en él; ante la duda no se une (R-07). Medido con las respuestas guardadas de las dos lecturas de N6: uniones falsas 16 → 0 y separaciones 88 → 66 en la primera; la segunda queda idéntica.
- **Alternativas descartadas:** Reemplazar el enlace del modelo por el destino de la definición: cuando la definición se conoce solo por su forma en el texto (el nombre más cercano antes), ese nombre puede ser el de la empresa de un domicilio "c/o" y deshizo un enlace correcto (separaciones de la segunda lectura 363 → 395).
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/extract.rs` (`define_terms`); prueba `tests_reading.rs` (`a_defined_term_names_one_thing_in_its_document`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N6a de la tanda desatendida del 18 oct.

### D-361 · Con un número en la pregunta, el resumen solo ordena los documentos que lo tienen

- **Fecha:** 18 oct 2026
- **Tema:** Búsqueda
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Con los resúmenes completos de la consigna `conocimiento-5` (P-171), palabras como "número", "cliente" o "DNI" aparecen en los resúmenes de muchos documentos; en una pregunta con un número, la lista del resumen desplazaba a los documentos que el número encuentra (DNI con puntos 9 → 6 de 9; número de cliente 10 → 6 de 13).
- **Qué se decidió:** Si la pregunta tiene un número (en cualquiera de sus formas), la lista del resumen se queda solo con los documentos que la lista de números halló; el resto de las listas no cambia.
- **Por qué:** Una pregunta con un número pide los documentos que lo tienen. Medido sobre las dos lecturas completas de N6 (30 preguntas, 104 esperados, primeros 10): chat 89 → 96 y 87 → 95; por documento 90/89 → 97/97; sobre la base de N4, 95 → 96.
- **Alternativas descartadas:** Bajar el peso del resumen (0,7; 0,5; 0,3; 0): no recupera las preguntas con número y pierde las de otro idioma.
- **Dónde se aplica:** `src-tauri/src/docbase/search.rs` (`search_documents`); prueba `search_tests.rs` (`a_question_with_a_number_is_not_pushed_out_by_summaries_without_it`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N6a de la tanda desatendida del 18 oct.

### D-362 · La medición de referencia del 18 de octubre son dos lecturas completas desde cero

- **Fecha:** 18 oct 2026
- **Tema:** Medición
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** El modelo varía entre corridas (P-133): una sola lectura no dice si un resultado es del programa o de la lectura.
- **Qué se decidió:** Los números del 18 oct se dan como dos lecturas completas desde una copia de la base de partida (luna high, una después de la otra) con el código de la rama, y los arreglos posteriores se miden con las respuestas guardadas de esas dos lecturas; solo se pregunta al modelo lo que el arreglo cambia (preguntas de identidad de grupos nuevos). Cada indicador se informa para las dos, más la estabilidad (pares de identidad, afirmaciones e importes iguales, casos críticos que pasan en las dos, en una o en ninguna). El juicio de atribución a ciegas mezcla 40 afirmaciones de cada lectura y de la base del 17 (misma semilla) para el mismo revisor.
- **Por qué:** Separa lo que mejora el programa de lo que cambia el modelo. Resultado (detalle fuera del repositorio, `desatendido-4/N6-resultados.md`): casos críticos 3 → 31 y 33 de 46 (28 en las dos); uniones falsas 0; separaciones 4,2 % → 1,3 % y 6,6 %; atribución a ciegas 38 → 37 y 33 de 40; búsqueda 61 → 96 y 95 de 104; llamadas de lectura 1,14 por documento.
- **Dónde se aplica:** Arneses fuera del repositorio (`desatendido-4/N6-correr.sh`, `N6-estabilidad.py`, `N6-indicadores.py`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N6a de la tanda desatendida del 18 oct.

### D-363 · Un documento que solo comparte la plantilla se tipa solo si el tipo del principal puede separar la familia

- **Fecha:** 18 oct 2026
- **Tema:** Tipos de documento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** En la app, con los datos reales, el tipo de un documento y ocho páginas cambiaban cada medio minuto sin fin. El documento compartía la plantilla de una carta documento: como miembro de esa familia recibía el tipo de la carta; con ese tipo, la regla «distinto número de envío, documentos distintos» deshacía la familia; solo, el modelo lo leía como acuse de recibo; la regla dejaba de aplicar y la familia volvía.
- **Qué se decidió:** Un miembro unido al principal solo por la plantilla (`same-template`) se tipa por su cuenta cuando el tipo actual del principal tiene una regla de datos distintos cuya relación no forma familia (hoy, la carta documento). Los demás miembros siguen recibiendo el tipo de la familia (una pregunta por familia).
- **Por qué:** Corta la dependencia circular entre el tipo y la familia sin cambiar las demás familias (facturas, recibos, acuerdos de la misma plantilla siguen con una sola pregunta). El tipo que el modelo lee en el documento solo es el correcto en el caso visto. Prueba: `a_member_of_the_template_whose_rule_parts_the_family_is_typed_alone_and_it_settles` (sin el arreglo, el tipo y la relación alternan en cada vuelta).
- **Dónde se aplica:** `src-tauri/src/docbase/similar/classify.rs` (`load_units`, `principal_type_parts_family`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N6b de la tanda desatendida del 18 oct.

### D-364 · Regeneración con los datos reales el 18 de octubre

- **Fecha:** 18 oct 2026
- **Tema:** Operación
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** El cierre de la tanda pedía aplicar el rediseño a la base y al wiki reales, con respaldo, y dejar la app abierta.
- **Qué se decidió:** Respaldo con sha256 de los datos de la app y del wiki (idéntico al de N0: nada había cambiado), wiki anterior apartado fuera del repositorio, carpeta del proyecto por defecto borrada (si existe vacía el arranque falla, P-209) y app abierta desde `llm_wiki_wt` con Codex `gpt-6-luna` `high`. La app migró la base a V17, releyó los 66 documentos con `conocimiento-5` (42 min), identidad y síntesis (10 min) y proyectó 59 páginas de documento y 60 de entidad. La app de desarrollo se abre con el puerto de depuración de WebView2 (`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`) para comprobar sin pantalla (búsqueda en modo base, grafo de la base) y sacar capturas.
- **Por qué:** Resultado medido sobre una copia (detalle fuera del repositorio, `desatendido-4/N6-defectos.md`): casos críticos 34 de 46; búsqueda del chat 94 de 104 en los diez primeros; uniones falsas 7 pares (P-206); separaciones 5,3 %; revisión final con 27 defectos de prioridad 1 (P-205).
- **Dónde se aplica:** datos de la app y `llm-wiki-workspace/default`; cómo volver atrás en `desatendido-4/N6-defectos.md`.
- **Quién decidió:** agente sin consulta (autorizado por el dueño en el plan de la tanda).
- **Fuente:** etapa N6b de la tanda desatendida del 18 oct.

### D-365 · Dos personas cuyo nombre de pila difiere en una letra se unen solo si el modelo está seguro

- **Fecha:** 18 oct 2026
- **Tema:** Identidad de entidades
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Con los datos reales, la identidad unió a dos personas de una misma familia: el mismo apellido, nombres de pila que difieren en una letra y ningún identificador ni casilla en común (P-206, 7 pares falsos contra la hoja). La unión no salió del nombre (un nombre con una letra distinta nunca une solo) sino del modelo: a la pregunta de identidad respondió «la misma, certeza media» porque las dos aparecían en documentos de la misma organización y «la diferencia de una letra puede ser un error de escritura». Otras versiones de la pregunta, sobre los mismos grupos, habían dicho «dudosa».
- **Qué se decidió:** Cuando el modelo dice «la misma» de dos grupos de personas que se parecen solo por nombres cuya primera palabra (casi siempre el nombre de pila) está a una letra de una palabra del otro y no está en él, la unión exige certeza alta; con certeza media o baja el par queda como duda para que lo decida una persona (`modelo-misma-nombre-distinto`, con el texto «puede ser un error de escritura o dos personas distintas con el mismo apellido»). No cuentan como parecido una casilla sin nombre, una sola palabra ni una inicial (sirven para los dos nombres); sí lo anulan un nombre igual o contenido en el otro, la misma casilla o un apellido mal escrito con los mismos nombres de pila (ahí sigue valiendo lo que dice el modelo). Se sube la versión de la resolución (`resolucion-10`) para recalcular el registro con los veredictos guardados, sin preguntas nuevas.
- **Por qué:** Un apellido compartido con un nombre de pila casi igual es justo lo que tienen dos parientes; sin un identificador o una relación que lo diga, juntarlos es adivinar, y una unión falsa mezcla datos de dos personas en una página (prioridad 1). Un nombre escrito con el apellido primero también cae en la regla, que es el lado prudente: queda una duda, no una unión. Medido sin preguntar al modelo (respuestas guardadas): uniones falsas en la copia de la app 7 → 0, en las dos lecturas completas de N6a 0 → 0; separaciones falsas iguales (5,3 %, 1,3 %, 6,6 %); casos críticos iguales (34 de 46 en la app). La primera versión de la regla (cualquier palabra a una letra) separaba además un apellido mal escrito y subía las separaciones 0,1 punto: se acotó a la primera palabra. Pruebas: `people_whose_first_names_differ_by_a_letter_join_only_when_the_model_is_sure` (sin el arreglo, la certeza media los une) y `a_first_name_a_letter_apart_is_told_from_a_surname_misspelled`.
- **Dónde se aplica:** `src-tauri/src/docbase/knowledge/resolve.rs` (`apply`, `alike_only_by_a_letter`, `said_with_high_certainty`), `names.rs` (`first_word_a_letter_apart`), `src/components/knowledge/knowledge.ts` (`doubtReason`).
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N6c de la tanda desatendida del 18 oct.

### D-366 · La identidad corregida se reaplica en la app con los datos reales

- **Fecha:** 18 oct 2026
- **Tema:** Operación
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** El arreglo D-365 cambia el registro de los datos reales (dos personas separadas), que la app ya tenía regenerado (D-364).
- **Qué se decidió:** Cerrar la app, compilar el arreglo y reabrirla desde `llm_wiki_wt` (mismo puerto de depuración), con el candado del modelo tomado a mano. Con la versión de la resolución nueva, la app recalcula la identidad desde los veredictos guardados al abrir y reescribe solo las páginas afectadas; no hace falta respaldo nuevo (el de N6b sigue valiendo para volver al 18 a las 00:48) ni relectura.
- **Por qué:** Medido en una copia de la base real después de reabrir (detalle fuera del repositorio, `desatendido-4/N6c-real-eval.txt`): uniones falsas 0 (antes 7 pares), separaciones 5,3 % (igual), casos críticos 34 de 46 (igual), ninguna pregunta al modelo; el par queda como duda en Atención.
- **Dónde se aplica:** datos de la app y `llm-wiki-workspace/default`.
- **Quién decidió:** agente sin consulta (autorizado por el dueño en el plan de la tanda).
- **Fuente:** etapa N6c de la tanda desatendida del 18 oct.

### D-370 · Para medir conceptos, una «unidad» junta las versiones y copias de un documento

- **Fecha:** 18 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** al buscar pasajes que se repiten en documentos distintos (D-295), un pasaje repetido entre dos versiones o dos copias de un mismo documento no es un concepto: es el mismo texto.
- **Qué se decidió:** para la medición, los documentos unidos por relaciones que significan «el mismo documento» (mismo documento, prácticamente el mismo, ejemplar firmado, versión), más vista previa y «contenido en», cuentan como una sola unidad. «Misma plantilla» queda aparte y señalada: dos contratos de la misma plantilla son documentos distintos.
- **Por qué:** sin eso, casi todos los grupos de pasajes serían versiones del mismo documento.
- **Alternativas descartadas:** No registradas.
- **Dónde se aplica:** medición N7, fuera del repositorio (`desatendido-4/N7-*`); P-211.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N7 de la tanda desatendida del 18 oct.

### D-371 · Qué es un pasaje repetido

- **Fecha:** 18 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el cálculo de parecidos cuenta secuencias de cinco palabras; un concepto necesita pasajes, no secuencias sueltas.
- **Qué se decidió:** un pasaje son al menos 8 secuencias contiguas (unas 12 palabras), admitiendo huecos de hasta 3. Dos pasajes se agrupan si se solapan en un 50 % o comparten secuencias. Las secuencias se cuentan **sin** el corte de «texto de rutina» del cálculo de parecidos, porque en un archivo grande ese corte se llevaría justamente las cláusulas estándar.
- **Por qué:** con distintos largos mínimos salen 82, 69, 47 y 42 grupos (4, 8, 15 y 30 secuencias); 8 conserva las cláusulas cortas sin llenarse de ruido.
- **Alternativas descartadas:** usar el corte de rutina del cálculo de parecidos.
- **Dónde se aplica:** medición N7; P-211.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N7 de la tanda desatendida del 18 oct.

### D-372 · Tres clases para un grupo de pasajes

- **Fecha:** 18 oct 2026
- **Tema:** Conocimiento
- **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** no todo pasaje repetido es un concepto: hay membretes, rótulos de formularios y copias no detectadas.
- **Qué se decidió:** cada grupo es *reconocible* (una cláusula o figura con nombre), *formulario* (texto fijo de un formulario, una plataforma de firma o una boleta) o *ruido* (copias no detectadas, listas de importes). Una regla general hace un primer filtro; la clase final la da el modelo, una vez por grupo.
- **Por qué:** medido: la regla sola marcó como formulario 7 cláusulas reales; el modelo y el juicio del agente coincidieron en el 81 %.
- **Alternativas descartadas:** solo la regla, sin el modelo.
- **Dónde se aplica:** medición N7; P-211.
- **Quién decidió:** agente sin consulta.
- **Fuente:** etapa N7 de la tanda desatendida del 18 oct.

### D-373 · Un solo concepto de asunto

- **Fecha:** 20 oct 2026 · **Tema:** Asuntos · **Estado:** vigente
- **El problema:** hoy hay tres cosas que hablan de asuntos (entidades de tipo asunto que salen de la lectura, grupos automáticos que cuentan copias como documentos distintos y etiquetas "Caso" con reglas de carpeta) y ninguna es confiable.
- **Qué se decidió y por qué:** un solo concepto. Nace de dos maneras: lo crea una persona o lo propone el sistema; nunca por reglas de carpeta. Lo propuesto se ve como "propuesto" hasta que una persona lo confirma en Atención. Un documento puede estar en varios asuntos (un poder, por ejemplo). Se cuentan familias de versiones, no copias. El término genérico es "Asunto"; en la vertical de abogados, "Caso" (vocabulario). Reemplaza en parte D-293 y D-294 y cierra la pregunta de P-061. Detalle en `docs/design/asuntos-propuesta.md`.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 19 y 20 oct.

### D-374 · Las referencias son datos del documento

- **Fecha:** 20 oct 2026 · **Tema:** Asuntos · **Estado:** vigente
- **El problema:** un número de acta, un expediente o "el contrato de X" se guardaban como entidades de tipo asunto (85 en la base real, ninguna con algo que decir), y llenaban Personas y organizaciones de fichas vacías.
- **Qué se decidió y por qué:** no son entidades sino datos del documento: se buscan y sirven de evidencia para proponer asuntos. Por ahora salen de la lectura que ya existe; no se escriben reglas de extracción propias (Amazon Textract, a evaluar más adelante, P-026).
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 19 y 20 oct.

### D-375 · El asunto tiene estado desde ahora; todos abiertos

- **Fecha:** 20 oct 2026 · **Tema:** Asuntos · **Estado:** vigente
- **El problema:** un caso se abre y se cierra; si el estado no existe desde el principio, agregarlo después obliga a rehacer datos y pantallas.
- **Qué se decidió y por qué:** el asunto guarda su estado desde ahora y todos quedan "abierto". Cómo se cierra o se archiva se decide aparte (P-213).
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 19 y 20 oct.

### D-376 · Una sola vista por cosa

- **Fecha:** 20 oct 2026 · **Tema:** Pantallas · **Estado:** vigente
- **El problema:** la misma persona, el mismo asunto o el mismo documento se ven dos veces: en una pantalla de la base y en una página del wiki, con información distinta.
- **Qué se decidió y por qué:** una sola vista por cosa: arriba el artículo (lo que dicen los documentos, con citas y certeza) y en una pestaña los datos y las correcciones. Personas y organizaciones, Asuntos y el detalle de Documentos se unifican con su página del wiki; Orígenes no tiene par. Se empieza por persona u organización; asunto y documento después (P-214).
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 19 y 20 oct.

### D-377 · Las referencias son las menciones de tipo asunto, sin tabla nueva

- **Fecha:** 20 oct 2026 · **Tema:** Asuntos · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** D-374 pide que las referencias (número de acta, expediente, "el contrato de X") sean datos del documento y no entidades, sin releer los documentos ni cambiar la consigna de lectura.
- **Qué se decidió y por qué:** no hay tabla nueva: una referencia es una mención del tipo `matters.kind` del catálogo (hoy `asunto`), que ya tiene lo escrito, la forma normalizada, la cita, la página y sus números. La resolución ya no les da entidad (quedan "referencia") y suelta la que tenían; las 85 entidades de tipo asunto quedan retiradas al resolver otra vez (versión `resolucion-11`), sin preguntas nuevas. Se ven en el detalle del documento (sección "Referencias", término del vocabulario), la búsqueda las encuentra por su nombre, y no aparecen en Personas y organizaciones ni en el grafo. Un término que el documento define o solo palabras comunes no son referencia.
- **Quién decidió:** agente sin consulta.

### D-378 · Una referencia junta documentos por su forma normalizada, nunca entre otras partes

- **Fecha:** 20 oct 2026 · **Tema:** Asuntos · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la agrupación en asuntos usaba "la misma entidad de tipo asunto"; sin esas entidades necesita otra evidencia.
- **Qué se decidió y por qué:** dos documentos comparten una referencia cuando tienen la misma forma normalizada; como en D-263, no se juntan si cada uno tiene dos partes o más y las de ninguno están dentro de las del otro (otro acto entre otros). Ya no hay unión aproximada de nombres: en la base de partida un documento sale de un asunto porque solo entraba por "Agreement". S3 rehace la agrupación por familias.
- **Quién decidió:** agente sin consulta.

### D-379 · Una afirmación sobre una referencia queda como valor escrito

- **Fecha:** 20 oct 2026 · **Tema:** Asuntos · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** sacar las referencias de la resolución cambiaba lo que muestran las preguntas de identidad y borraba afirmaciones como "Parte en: <contrato>": la app habría hecho 26 preguntas nuevas y perdido datos con cita.
- **Qué se decidió y por qué:** las referencias siguen en la resolución solo como contexto de lo que se pregunta (sin entidad, sin candidatos), y una afirmación cuyo objeto es una referencia queda con ese objeto como valor escrito, con su cita. Medido en la base de partida: 3 preguntas nuevas (2 síntesis y 1 propuesta de asunto), por debajo del límite de 10.
- **Quién decidió:** agente sin consulta.

### D-381 · La agrupación cuenta familias: versiones y lo que es parte de un documento

- **Fecha:** 20 oct 2026 · **Tema:** Asuntos · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** un grupo de dos documentos o más era un asunto aunque fueran un solo documento con sus copias, versiones o la constancia de su firma: en la base de partida 4 de 7 grupos eran eso.
- **Qué se decidió y por qué:** la agrupación (`knowledge/matters.rs`) cuenta familias: las relaciones de `pages.versionRelations` (como las páginas por familia) más las de `matters.sameDocument` del catálogo (constancia de firma, traducción, acuse de recibo: son parte del mismo acto), propuestas o confirmadas, nunca las descartadas. Un grupo de una sola familia no es asunto; sus documentos siguen en el grupo cuando hay otra familia. Medido: de 7 grupos quedan 4; el cuarto son dos boletas que la base relaciona como «misma plantilla, otros datos» (cambian identificadores y fechas), no como versiones: si son el mismo documento, una persona lo marca y desaparece.
- **Quién decidió:** agente sin consulta.

### D-382 · Confirmar o descartar se guarda en el asunto y sobrevive al recálculo

- **Fecha:** 20 oct 2026 · **Tema:** Asuntos · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** lo que propone el sistema es una propuesta hasta que una persona la confirma (D-373), y el recálculo de la agrupación no puede borrar esa decisión.
- **Qué se decidió y por qué:** `knowledge_matter` suma `status` (abierto), `origin` (propuesto o de una persona), `confirmation` (propuesto, confirmado, descartado), `name_by_person`, `decided_at` y `decided_documents` (migración 18: lo existente queda propuesto y abierto; las decisiones de una persona de `knowledge_decision` no se tocan). Confirmado: la agrupación nunca lo retira. Descartado: se retira y guarda sus documentos; vuelve como propuesta solo si la agrupación le suma un documento nuevo. Un asunto de una persona no lo toca la agrupación, y un nombre puesto por una persona se conserva. Atención lista los propuestos con Confirmar, Descartar y Ver, y el motivo que los une.
- **Quién decidió:** agente sin consulta.

### D-383 · El modelo propone documentos sueltos solo para un asunto confirmado o creado por una persona

- **Fecha:** 20 oct 2026 · **Tema:** Asuntos · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** con la agrupación nueva cambian los asuntos que se describen en la pregunta de propuesta y la app habría hecho 11 preguntas nuevas (límite 10) sobre grupos que todavía nadie confirmó.
- **Qué se decidió y por qué:** la pregunta «¿este documento suelto es de alguno de estos asuntos?» se hace solo contra asuntos confirmados o creados por una persona: preguntar sobre una propuesta es gastar en algo que quizás se descarte. Un documento que está en cualquier asunto, propuesto o no, no es suelto. Medido: 0 preguntas de propuesta y 2 de síntesis (las mismas de S2).
- **Quién decidió:** agente sin consulta.

### D-384 · «Caso» en las pantallas de abogados; el wiki sigue con las palabras del catálogo

- **Fecha:** 20 oct 2026 · **Tema:** Asuntos · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** D-373 pide «Asunto» genérico y «Caso» en la vertical de abogados; las pantallas toman el término del vocabulario (`src/vocabulary/legal.ts`), pero las páginas del wiki y los nombres de los asuntos usan las palabras del catálogo (`knowledge.json`), que entran en la descripción de las preguntas de propuesta.
- **Qué se decidió y por qué:** la vertical de abogados dice «Caso», «Casos» y «Sin caso identificado» en las pantallas, con «Propuesto», «Confirmado» y «Abierto» también desde el vocabulario. Las palabras del wiki quedan genéricas («Asunto») para no cambiar las preguntas guardadas; llevarlas a la vertical es P-217.
- **Quién decidió:** agente sin consulta.

### D-386 · Crear y editar un asunto a mano: lo que hace una persona se guarda por asunto y gana sobre la agrupación

- **Fecha:** 20 oct 2026 · **Tema:** Asuntos · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** D-373 pide que una persona pueda crear un asunto, nombrarlo y agregarle o sacarle documentos, y que eso gane sobre lo propuesto y sobreviva al recálculo. Las decisiones que había («mismo asunto», «asuntos distintos») son entre pares de documentos: no sirven para un asunto creado a mano (la agrupación no lo toca) y, con un documento en varios asuntos, sacarlo de uno lo separaba de todos los documentos del otro.
- **Qué se decidió y por qué:** cada documento puesto o sacado a mano se guarda por asunto (tabla `knowledge_matter_choice`, migración 19: `in` / `out`) y la agrupación lo aplica sobre lo que encuentra; el nombre puesto a mano queda (`name_by_person`; vacío devuelve el nombre de la app a uno propuesto). Un asunto creado a mano nace confirmado, con origen «persona», vacío o con documentos, y la agrupación no lo toca. Agregar o sacar un documento de un asunto propuesto lo confirma: es una decisión de una persona. A uno propuesto no se le saca el último documento (se descarta). Se hace desde Asuntos (nuevo, renombrar, buscador de documentos que usa la búsqueda de Documentos) y desde el detalle del documento (agregar a uno existente, crear uno con ese documento, sacar). Las decisiones entre pares que ya existen siguen valiendo; «descartar» la propuesta del modelo para un documento suelto sigue usándolas. Sin modelo: no agrega preguntas.
- **Quién decidió:** agente sin consulta.

### D-389 · Página única de persona u organización: el artículo es su página del wiki tal como la escribió la proyección; los enlaces del wiki llevan a ella

- **Fecha:** 20 oct 2026 · **Tema:** Pantallas · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464); reemplazada en parte por D-396: el artículo pasa a leerse de la base, no del archivo; las dos pestañas siguen valiendo
- **El problema:** D-376 pide una sola vista por cosa, empezando por persona u organización: arriba el artículo (síntesis con citas y certeza, documentos por asunto, vínculos), en una pestaña los datos y las correcciones. Había dos lugares: la pantalla de Personas y organizaciones (correcciones) y la página del wiki (lectura), con un botón para ir de una a la otra.
- **Qué se decidió y por qué:** la pantalla de la entidad tiene dos pestañas, «Artículo» y «Datos y correcciones». El artículo es el archivo de su página tal como lo escribió la última pasada del wiki, desde la base y la síntesis guardada (sin preguntar al modelo), con las notas de la persona; se muestra con el mismo lector del wiki. Así no hay un segundo armado del artículo que pueda diferir del wiki: el código de `knowledge/pages.rs` sigue siendo el único que lo escribe. La pestaña de datos es la pantalla de antes (unir, separar, incorrecto, confirmar, deshacer), con un contador de lo que espera una decisión. Una entidad sin página (nombrada de paso) abre en los datos. Los enlaces del lector del wiki, del panel de enlaces, de los relacionados y de Atención a la página de una entidad abren esta pantalla (comando `docbase_knowledge_entity_of_page`); abrir el archivo desde el árbol sigue abriendo el archivo, para escribir notas («Escribir notas»). Los enlaces dentro del artículo funcionan con el wiki del archivo abierto, como en el wiki. Sin modelo: no agrega preguntas.
- **Quién decidió:** agente sin consulta.

### D-395 · Se dejan de admitir los proyectos de wiki originales; la app abre en Inicio

- **Fecha:** 21 oct 2026 · **Tema:** Producto · **Estado:** vigente
- **El problema:** la app heredó los proyectos de wiki (cada uno, una carpeta con sus páginas) y crea uno general, el "Archivo", para poder mostrar las pantallas de la base. Hoy lo útil está en la base: elegir o cambiar de proyecto confunde y ata la base a una carpeta.
- **Qué se decidió y por qué:** la app deja de crear, abrir y listar proyectos de wiki (bienvenida, creación, cambio y registro) y abre en Inicio. Volver a tenerlos, con un propósito definido, queda como P-225. Reemplaza D-069 y D-077. Detalle en `docs/design/retiro-del-wiki.md`.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 20 y 21 oct.

### D-396 · Los archivos del wiki desaparecen: lo no repetido pasa a la base y la carpeta se borra tras migrar y respaldar

- **Fecha:** 21 oct 2026 · **Tema:** Wiki · **Estado:** vigente
- **El problema:** los archivos del wiki son una copia de lo que ya sabe la base: se desactualizan (P-221), se leen por caminos distintos y obligan a mantener las dos cosas.
- **Qué se decidió y por qué:** las páginas (de documento, de persona u organización y de asunto) y el resumen general los sigue armando el mismo código, pero los guarda en la base, con el mismo nombre de página: no se le pregunta nada nuevo al modelo y los enlaces siguen funcionando. Lo que solo estaba en archivos pasa a la base con una importación única: notas de una persona, consultas guardadas e historial del chat. La carpeta del wiki se borra al final, después de un respaldo con huellas y de comprobar que ningún origen apunta dentro de ella. Reemplaza en parte D-389 (el artículo sale de la base). Resuelve P-191 y lo de archivos de P-192.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 20 y 21 oct.

### D-397 · La investigación en internet queda, adaptada: se lanza desde una ficha y se guarda en la base

- **Fecha:** 21 oct 2026 · **Tema:** Investigación · **Estado:** vigente
- **El problema:** la investigación guardaba su resultado en archivos (D-334) y leía el índice del wiki; sin wiki no tiene dónde quedar ni a qué asociarse.
- **Qué se decidió y por qué:** se lanza desde una persona, organización o asunto. El resultado se guarda en la base, asociado a esa ficha, con sus fuentes, y se muestra separado de lo que dicen los documentos: es información de afuera, no del archivo del estudio. El panel general sigue para investigaciones sueltas, también guardadas en la base. Usa el modelo solo cuando una persona la pide. Reemplaza D-334.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 20 y 21 oct.

### D-398 · El grafo y su análisis se calculan sobre la base y suman los asuntos con su estado

- **Fecha:** 21 oct 2026 · **Tema:** Grafo · **Estado:** vigente
- **El problema:** el grafo sale de la base, pero solo con lo que tiene página del wiki; su relevancia lee archivos y los asuntos no muestran si están propuestos o confirmados.
- **Qué se decidió y por qué:** los nodos (personas y organizaciones, documentos, asuntos con su estado y confirmación), la relevancia y el análisis (comunidades, puntos clave) se calculan con lo que hay en la base. Un clic abre la pantalla de lo que se tocó.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 20 y 21 oct.

### D-399 · Lint sale; queda pendiente una revisión con el modelo que agregue valor

- **Fecha:** 21 oct 2026 · **Tema:** Wiki · **Estado:** vigente
- **El problema:** Lint revisa la forma de las páginas del wiki (enlaces rotos, páginas sueltas); sin archivos no tiene qué revisar. Lo que le sirve a un estudio es otra cosa: contradicciones y divergencias entre documentos.
- **Qué se decidió y por qué:** Lint sale con su código. Review también: lo que pide una decisión ya está en Atención. Una revisión con el modelo que busque contradicciones y divergencias en la estructura nueva queda como P-226.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 20 y 21 oct.

### D-400 · El servidor de API local y el recortador web (Clip) salen; se rehacen sobre la base más adelante

- **Fecha:** 21 oct 2026 · **Tema:** Producto · **Estado:** vigente
- **El problema:** los dos trabajan sobre los proyectos y los archivos del wiki; sin ellos no tienen dónde leer ni escribir.
- **Qué se decidió y por qué:** se quitan. Rehacerlos sobre la base queda como P-227.
- **Quién decidió:** dueño del producto.
- **Fuente:** conversación del 20 y 21 oct.

### D-402 · Las páginas se guardan en la base con su ruta lógica; la proyección no usa archivos ni el proyecto general

- **Fecha:** 21 oct 2026 · **Tema:** Wiki · **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
- **El problema:** la proyección escribía cada página en la carpeta del proyecto general y exigía ese proyecto; la carpeta se va a borrar (D-396) y las preguntas de síntesis no pueden cambiar.
- **Qué se decidió y por qué:** el mismo código arma cada página y la guarda en la base: `knowledge_page.body` (sin las notas, que van aparte) y el resumen general en `knowledge_overview`; la ruta lógica (`wiki/entities/…`) sigue siendo su nombre, así los enlaces internos siguen valiendo. Los documentos salen de todos los orígenes de la base, nombrados `<origen>/<ruta>`, no de los montados. Medido sobre una copia de la base real: las 116 páginas iguales al archivo que había, cero preguntas, cero archivos. La tabla `knowledge_research` queda creada para W5.
- **Quién decidió:** agente sin consulta.

### D-403 · Las notas y consultas guardadas del wiki entran una sola vez en la base; una nota sin dueño queda aparte

- **Fecha:** 21 oct 2026 · **Tema:** Wiki · **Estado:** reemplazada por D-396 (era del wiki retirado; D-464)
- **El problema:** lo único que escribía una persona en el wiki eran las notas de cada página y las consultas guardadas; no pueden perderse al borrar la carpeta.
- **Qué se decidió y por qué:** al abrir la app, si la carpeta sigue, se leen una vez (marca `wiki-files-imported`): las notas de cada página van a su objeto (por la ruta en la base, por el encabezado o, en una página de documento, por la familia del documento); las que no corresponden a ningún objeto quedan como nota aparte con el título de su página; cada archivo de `wiki/queries` pasa a `knowledge_query`. No se borra nada. Una página de documento escrita por la incorporación vieja ya no presta su ruta a la página del documento: solo sus notas pasan.
- **Quién decidió:** agente sin consulta.

### D-404 · Lo que las pantallas leen de las páginas sale de comandos de la base

- **Fecha:** 21 oct 2026 · **Tema:** Interfaz · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** las pantallas, la búsqueda y el chat leían el archivo de cada página.
- **Qué se decidió y por qué:** comandos nuevos: artículo de cualquier objeto o de un documento (cuerpo y notas aparte), el objeto que nombra una ruta lógica, una ruta vieja o un enlace, notas (leer, guardar, las apartadas), resumen general y consultas guardadas (listar, guardar, borrar). La búsqueda toma título y texto de la base. Los dos comandos que usa hoy la pantalla de una persona quedan como envoltorio hasta W3; su `path` pasa a ser la ruta lógica.
- **Quién decidió:** agente sin consulta.

### D-405 · La lectura automática se rige por su propio ajuste, no por «Activar wiki»

- **Fecha:** 21 oct 2026 · **Tema:** Base documental · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la lectura de fondo solo corría con el wiki del proyecto general activo.
- **Qué se decidió y por qué:** ajuste `background-reading` en la base, que la migración toma del valor que tenía el interruptor del wiki (en la base real, encendido). El interruptor viejo escribe los dos hasta que W6 lo reemplace por «Lectura automática».
- **Quién decidió:** agente sin consulta.

### D-407 · Las pantallas muestran el artículo y las notas desde la base; todo enlace abre una pantalla, nunca un archivo

- **Fecha:** 21 oct 2026 · **Tema:** Interfaz · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el artículo de una persona se leía del archivo, las notas se escribían en el archivo con el wiki abierto, el detalle de documento y de asunto remitían a páginas del wiki e Inicio contaba páginas.
- **Qué se decidió y por qué:** persona u organización: el artículo de la base y debajo un editor de notas (texto, Guardar; vacío las borra), sin «Escribir notas». Documento: arriba su artículo (sin ubicaciones ni bloque de entidades, que la pantalla ya muestra), luego el texto y sus notas; sin «En el wiki». Asunto: «lo que dicen sus documentos» del artículo y sus notas; sin «Su página del wiki». Inicio: el resumen general; sin el contador de páginas. Cada artículo dice cuándo lo escribió la app (P-221). `openWikiPath` resuelve cualquier ruta lógica, ruta vieja o enlace con `objectOfPage` y abre la pantalla de la entidad, el asunto, el documento o Inicio; si la base no lo conoce, no abre nada. El mismo editor de notas en las tres pantallas.
- **Quién decidió:** agente sin consulta.

### D-408 · Una pantalla toma una sección del artículo por las palabras de la proyección; la búsqueda de la base no necesita proyecto

- **Fecha:** 21 oct 2026 · **Tema:** Interfaz · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el asunto y el documento muestran solo parte de su artículo, pero los títulos de sus secciones salen del vocabulario de la proyección, que puede reemplazarse; y la búsqueda solo usaba la base con el proyecto general abierto.
- **Qué se decidió y por qué:** comando `docbase_knowledge_page_words` que da las palabras con que se escriben las páginas, para buscar la sección por su título sin escribirlo en la interfaz. Comando `docbase_search` que busca en la base sin proyecto (la misma búsqueda de D-350); la vista Buscar lo usa y abre pantallas; sus imágenes salen (las páginas de la base no tienen).
- **Quién decidió:** agente sin consulta.

### D-410 · El grafo muestra los objetos de la base, con o sin página, y cada nodo abre su pantalla

- **Fecha:** 21 oct 2026 · **Tema:** Grafo · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el grafo salía de la base pero solo con lo que tenía página (`knowledge_page`), identificaba cada nodo por el nombre del archivo, no decía si un asunto estaba propuesto o confirmado y al hacer clic abría un editor del archivo.
- **Qué se decidió y por qué:** nodos: las personas y organizaciones activas (una organización con sociedades en su grupo es un «grupo de sociedades», término nuevo del vocabulario), cada documento visible con sus versiones (uno por familia, la misma de la proyección) y los asuntos activos con su confirmación y su estado. Cada nodo lleva el id de su objeto (`ent-…`, `mat-…`, `doc-…`) y la ruta lógica de su página si la tiene. Aristas como antes. El comando `docbase_graph` ya no pide proyecto. Clic en un nodo abre su pantalla (`openPageObject`); fuera el editor del grafo. Las aristas muestran su nombre; al pasar sobre una o tocarla, sus relaciones con su estado en palabras y su cita. Un asunto propuesto va en un verde más claro que uno confirmado, su estado se lee al enfocarlo, la leyenda los separa y un filtro oculta propuestos o confirmados. En la copia de la base real suman las personas y organizaciones sin página y los grupos, con sus aristas (medición fuera del repositorio, W4-medicion.txt).
- **Quién decidió:** agente sin consulta.

### D-411 · Relevancia, comunidades y análisis salen de las relaciones de la base, con textos del vocabulario

- **Fecha:** 21 oct 2026 · **Tema:** Grafo · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la relevancia de las aristas leía los archivos del wiki (enlaces y fuentes del encabezado) y el análisis hablaba de páginas, en inglés, y sugería escribir enlaces.
- **Qué se decidió y por qué:** la relevancia se calcula en memoria con cuatro señales: relaciones directas entre los dos (hasta tres), documentos que comparten, vecinos en común (Adamic-Adar) y cercanía de sus tipos. Comunidades (Louvain) igual que antes, sobre ese grafo. Análisis en español con los nombres del vocabulario: nodos sin ninguna relación (antes, una o ninguna: en la base casi todos tienen una), comunidades poco relacionadas por dentro y puntos clave (nodos que tocan tres comunidades o más); nunca pide escribir enlaces.
- **Quién decidió:** agente sin consulta.

### D-412 · Los textos del grafo se escriben en español, fuera de los paquetes de traducción

- **Fecha:** 21 oct 2026 · **Tema:** Interfaz · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la pantalla del grafo era la última de la base con sus textos en los paquetes de traducción heredados, que la muestran en inglés.
- **Qué se decidió y por qué:** sus textos se escriben en español en la pantalla, como en las demás de la base, con los términos del dominio del vocabulario; la sección `graph` sale de los cuatro paquetes.
- **Quién decidió:** agente sin consulta.

### D-413 · El chat lee y guarda todo en la base: buscar, leer un objeto, guardar una consulta, pasajes y relaciones

- **Fecha:** 21 oct 2026 · **Tema:** Chat · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** las herramientas del agente del chat buscaban y leían páginas de `wiki/`, buscaban texto en `raw/sources`, armaban el grafo con los enlaces de los archivos, tomaban el contexto de `overview.md` y `schema.md` y guardaban respuestas como páginas de `wiki/queries`; el chat con proveedores de línea de comandos leía esos archivos y las referencias abrían el lector de archivos.
- **Qué se decidió y por qué:** herramientas sobre la base, con nombres nuevos: `base.search` (los documentos, personas, organizaciones y asuntos que encuentra la búsqueda de la base, cada uno con su ruta lógica y el pasaje), `base.read` (el artículo de un objeto por su id, su ruta lógica o un enlace), `base.save_query` (una consulta guardada en la base, en lugar de escribir una página), `source.search` (pasajes del texto de los documentos) y `graph.search` (relaciones del grafo de la base). El contexto de cada turno es el resumen general de la base; no hay esquema. El proveedor de línea de comandos recibe el texto de las páginas desde la base (`docbase_knowledge_page_text`). Las referencias y los `[[enlaces]]` de una respuesta abren la pantalla de lo que nombran. «Guardar consulta» guarda pregunta, respuesta y lo que cita; la lista «Consultas guardadas» del chat las abre y las borra. La escritura de páginas de la API local responde «retirada» (sale con la API, D-400). Los textos nuevos van en español en la pantalla, como D-412.
- **Quién decidió:** agente sin consulta.

### D-414 · La investigación en internet se guarda en la base, de una persona, organización o asunto o suelta, con sus fuentes citadas

- **Fecha:** 21 oct 2026 · **Tema:** Investigación · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la investigación escribía archivos en `.llm-wiki/research/` o `wiki/queries`, leía `wiki/index.md` para enlazar páginas y exigía un proyecto abierto; D-397 pide lanzarla desde una ficha y guardarla en la base, aparte de lo que dicen los documentos.
- **Qué se decidió y por qué:** comandos sobre `knowledge_research` (guardar, listar de un objeto, listar sueltas, borrar). La página de persona u organización (pestaña Artículo) y la de asunto tienen la sección «Investigación en internet»: tema propuesto con su nombre y editable, lo que está en curso, y cada investigación con su texto y sus fuentes, para abrir y borrar. El panel general guarda sin ficha y lista las sueltas. Desde el grafo, la investigación de un hueco se guarda del objeto de su nodo cuando es una persona, organización o asunto; si no, suelta (P-236). La consigna dice de quién es y que se presenta aparte de los documentos; ya no recibe el índice del wiki. Se guardan solo las fuentes que la síntesis cita, con su número. Nada de una investigación entra en el registro, las páginas ni el grafo.
- **Quién decidió:** agente sin consulta.

### D-415 · El historial y las preferencias del chat viven en la carpeta de datos de la app, con una copia única desde el proyecto general

- **Fecha:** 21 oct 2026 · **Tema:** Chat · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** las conversaciones y las preferencias del chat se guardaban en `<proyecto>/.llm-wiki/`, que se borra con la carpeta del wiki (D-396), y el chat guardaba además las sesiones del agente en la misma carpeta.
- **Qué se decidió y por qué:** van a `chat/` en la carpeta de datos de la app, con la misma forma (`conversations.json`, `chats/<id>.json`, `chat-preferences.json`). La primera vez se copian desde el `.llm-wiki/` del proyecto general, si existe: una sola vez por base (`chat-files-copied` en `docbase_setting`), sin reemplazar lo que ya esté ni borrar nada del origen. El chat de la app ya no pide al agente guardar su sesión en la carpeta del proyecto (lleva su propio historial). Medido con la copia de la carpeta del wiki: todos sus archivos del chat quedan copiados (W5-medicion.txt, fuera del repositorio).
- **Quién decidió:** agente sin consulta.

### D-417 · «Lectura automática» reemplaza a «Activar wiki»: la base lee sola con el modelo configurado mientras está activa

- **Fecha:** 21 oct 2026 · **Tema:** Orígenes · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el interruptor «Activar wiki» de Orígenes encendía a la vez la incorporación al wiki y la lectura de la base, y le decía a la base con qué modelo preguntar. Con el wiki retirado (D-395) hacía falta un interruptor solo para la base.
- **Qué se decidió y por qué:** el panel «Lectura automática» de Orígenes enciende y pausa el ajuste `background-reading` (D-402), con el mismo aviso de costo. Al activarla, y mientras está activa cada vez que cambia el modelo, le dice a la base con qué motor preguntar, como hacía el panel del wiki; abrir la app no cambia el motor, así que no provoca preguntas nuevas. Los botones «Incorporar al wiki» del árbol de Orígenes salen. Los textos de Atención y de las pantallas que decían «mientras el wiki esté activo» hablan de la lectura automática, y los de documentos parecidos dicen qué documento «se toma» o «cuenta» en lugar de cuál alimenta el wiki. El panel de investigación puede abrirse junto a cualquier pantalla menos chat, habilidades y ajustes.
- **Quién decidió:** agente sin consulta.

### D-418 · El agente del chat y los modelos de línea de comandos trabajan en la carpeta de datos de la app; el chat se guarda solo después de leído

- **Fecha:** 21 oct 2026 · **Tema:** Chat · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el agente resolvía un proyecto para su carpeta de trabajo, sus habilidades (`.llm-wiki/skills`) y sus sesiones, los modelos de línea de comandos (Codex, Claude Code) exigían que su carpeta fuera un proyecto con `wiki/index.md`, y el guardado automático del chat esperaba un proyecto abierto y podía escribir un historial vacío antes de leer el guardado (P-239).
- **Qué se decidió y por qué:** la carpeta del agente es `<datos de la app>/agent` (comando `agent_workspace`): ahí van sus sesiones (`sessions/`), sus habilidades (`skills/`, junto con las del usuario) y lo que produce (`agent-workspace/`); los modelos de línea de comandos aceptan cualquier carpeta existente y trabajan en esa. El guardado automático arranca suspendido: escribe las preferencias cuando terminó el arranque y el historial solo si se pudo leer, así nada lo reemplaza por uno vacío. Los archivos del agente se muestran en el explorador del sistema y las imágenes locales solo se cargan desde su carpeta.
- **Quién decidió:** agente sin consulta.

### D-419 · Todos los ajustes son globales; salen los que solo servían a la incorporación del wiki

- **Fecha:** 21 oct 2026 · **Tema:** Ajustes · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** había ajustes por proyecto (modelo e idioma de salida) y secciones que solo usaba la incorporación original: vigilancia de fuentes, importación programada, mantenimiento, embeddings de páginas, descripción de imágenes (multimodal) y MinerU; sin proyectos ni incorporación no tienen efecto.
- **Qué se decidió y por qué:** quedan General, Modelo, Búsqueda web, Red, Salida, Interfaz, Novedades y Acerca de, todos globales; salen las otras secciones y el modelo por proyecto (el modelo por tarea sigue). El idioma de salida se guarda global; la base solo recibe el idioma de sus artículos cuando una persona lo cambia en Ajustes, porque cambiarlo vuelve a preguntar las síntesis. Salen también la API local, MCP y Clip (D-400).
- **Quién decidió:** agente sin consulta.

### D-420 · El motor que reconoce escaneos se conserva sin uso hasta que la base lo use por su cuenta

- **Fecha:** 21 oct 2026 · **Tema:** Reconocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la base nunca reconoció por su cuenta el texto de escaneos e imágenes: lo pedía la incorporación del wiki y se lo devolvía. Al retirarla, esos documentos quedan en «tienen el texto en imagen» sin que nada los lea.
- **Qué se decidió y por qué:** no se construye el reconocimiento en la base en este paso (es una capacidad nueva, con costo de cupo); se conserva el motor (`source_volume/recognition*`, `recognized_store`, `store_recognized_text`) marcado como sin uso, y Atención dice que por ahora quedan sin leer. Queda pendiente con prioridad alta (P-242).
- **Quién decidió:** agente sin consulta.

### D-421 · Desktop y Core se comunican por escrito, con un buzón de cada lado

- **Fecha:** 21 oct 2026 · **Tema:** Proceso de trabajo · **Estado:** vigente
- **El problema:** Micelya Core (`extrimian/mi-platform`) y Micelya Desktop los desarrolla el mismo equipo en dos tracks, cada uno con su agente. Desktop va a depender de servicios de Core (primero el reconocimiento de texto) y hacía falta un canal ordenado para pedir, preguntar y acordar, que quede escrito y que cada agente encuentre al empezar una sesión.
- **Qué se decidió:** Desktop escribe sus mensajes a Core en `docs/dropbox/core/` de este repositorio (solo escribe el agente de Desktop); Core responde en `docs/dropbox/desktop/` del repositorio de Core (solo escribe el agente de Core). Cada lado solo lee el buzón del otro. Un archivo por mensaje, numerado, con pedidos y preguntas codificados para responder punto por punto; los pendientes se nombran "Desktop P-xxx" o "Core P-xxx". Queda escrito en `CLAUDE.md`. Primer mensaje: 001 (reconocimiento de texto, conexión desde la app de escritorio).
- **Quién decidió:** dueño del producto. **Fuente:** conversación del 21 oct.

### D-422 · Cada servicio de IA se puede usar con distintos motores, configurables; Core es uno más

- **Fecha:** 21 oct 2026 · **Tema:** IA · **Estado:** vigente
- **El problema:** al retirar el wiki la base dejó de reconocer el texto de los escaneos (P-242), y Core ofrece reconocimiento con Textract, que lee bien lo impreso pero no tablas, casillas ni manuscrito en castellano. Elegir un solo motor para todo deja afuera casos y ata Desktop a un proveedor.
- **Qué se decidió:** independientemente del motor que se use, la plataforma permite configurar y usar distintos motores para cada servicio (preguntar, reconocer texto, describir imágenes, investigar, transcribir, vectores), como ya pasa con las preguntas al modelo. Core es un proveedor más, elegible por servicio; Desktop sigue funcionando con motores locales o propios. Especificación y plan en `docs/design/servicios-de-ia-propuesta.md` (propuesta para revisar). Amplía D-291.
- **Quién decidió:** dueño del producto. **Fuente:** conversación del 21 oct.

### D-423 · Documento, persona u organización y caso con la misma pantalla de dos pestañas; las notas al pie de las dos

- **Fecha:** 21 oct 2026 · **Tema:** Interfaz · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el dueño pidió (21 oct, paso 0a de `docs/design/servicios-de-ia-propuesta.md`, 2.6) que documento y caso tengan las pestañas «Artículo» y «Datos y correcciones» de persona u organización. Al repartir lo que ya mostraba cada pantalla quedaron detalles sin especificar.
- **Qué se decidió y por qué:** las tres pantallas usan un solo componente (`detail-tabs.tsx`), así se ven igual y un cambio vale para las tres. Las notas de una persona van al pie de las dos pestañas, con el mismo editor (lo que se está escribiendo no se pierde al cambiar de pestaña); en persona u organización antes estaban solo en el Artículo. Sin artículo, la pantalla abre en «Datos y correcciones» y la pestaña «Artículo» lo dice con una frase y un botón que lleva a los datos. Las acciones que corrigen (unir, confirmar o descartar el caso, cambiarle el nombre) se ven solo en «Datos y correcciones»; «Abrir el original» y «Ver en su carpeta» del documento, en las dos. En el documento, el texto completo va último en «Datos y correcciones» porque es lo más largo. En el caso, la sección suelta «Lo que dicen los documentos» sale (está dentro del artículo) y «Partes y personas» queda en los datos aunque el artículo también las nombra: sacar repeticiones es el paso siguiente (P-245). La pestaña de datos del caso cuenta lo que espera decisión: el caso propuesto y cada documento que propone el modelo.
- **Quién decidió:** agente sin consulta.

### D-424 · Atención en tres pestañas; «En curso» muestra todas las etapas del procesamiento, hechas y por hacer, cada una desplegable con sus documentos

- **Fecha:** 21 oct 2026 · **Tema:** Atención · **Estado:** vigente
- **El problema:** al retirar el wiki se perdió la cola de procesamiento. El dueño agregó una subcarpeta con 68 documentos y no podía ver cómo avanzaba: Atención tenía tres bloques uno debajo del otro, en «En curso» había líneas sueltas que desaparecían al terminar cada etapa, fuera de Atención no había ninguna señal de que la app trabajaba y los errores del modelo solo iban a la consola.
- **Qué se decidió:** Atención se divide en tres pestañas, «Problemas», «Decisiones» y «En curso», cada una con su cantidad al lado del rótulo. «En curso» muestra todas las etapas del procesamiento, las hechas y las que faltan, en el orden en que ocurren; una etapa que termina no desaparece: queda como hecha, con su cantidad. Cada etapa es un panel desplegable con la lista de sus documentos y un botón para abrir cada uno, como el que ya tenía «documentos con el texto en imagen». Se suman, como propuesta aceptada: el estado de cada etapa con «hechos de total» y lo que se está procesando ahora; un filtro por origen o carpeta; el estado de la lectura automática con su botón para pausarla o activarla; el último error del modelo en palabras, con la hora y cuándo vuelve a intentar; una línea fija que avisa que hay trabajo y lleva a «En curso»; y el número «En curso» de Inicio corregido para contar lo que de verdad está en proceso (P-249).
- **Quién decidió:** el dueño del producto, 21 oct.

### D-425 · Las nueve etapas de «En curso» salen del código y se cuentan de lo guardado; la pestaña cuenta las etapas sin terminar e Inicio, los documentos en proceso

- **Fecha:** 21 oct 2026 · **Tema:** Atención · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la decisión del dueño (D-424) pedía todas las etapas, con su estado y sus cantidades, pero no decía cuáles son, cómo se cuenta cada una ni qué número va en la pestaña y en Inicio. Sin cambiar el esquema de la base.
- **Qué se decidió y por qué:** las etapas son las que corren en el código (`worker.rs`), en su orden: recorrido de las carpetas; texto de cada documento (cuenta archivos: identificarlo y sacar su texto); texto de los escaneos (espera reconocimiento, nunca es error, hasta que la base reconozca imágenes, P-242); copias, versiones y plantillas; tipo y datos; lectura con el modelo (a quiénes nombran); quién es quién; asuntos (con el nombre de la vertical: «Casos»); y páginas (artículos). Las etapas que siguen cuentan documentos, no archivos: las copias se procesan una vez. Cada etapa se cuenta de nuevo en cada consulta, de lo que la base ya guarda y sin escribir nada (`docbase/progress.rs`). Una etapa está hecha, en curso, esperando (su turno, porque el motor pregunta las etapas en orden), en pausa (lectura automática apagada o sin modelo) o con errores. Lo que no entra en una etapa (demasiado grande, formato sin texto, acceso directo de la nube) se muestra como «queda afuera» y no se cuenta en el total; en asuntos se dice cuántos quedaron en un caso y cuántos sueltos. La pestaña «En curso» cuenta las etapas que no están hechas (sus elementos son las etapas). El número de Inicio cuenta los documentos distintos con algo pendiente en una etapa que avanza o espera su turno (un archivo sin leer cuenta como uno); lo que espera por la lectura en pausa se muestra aparte («N en pausa»), y los escaneos que esperan un reconocimiento que todavía no existe no cuentan. Las líneas sueltas de «En curso» que levantaban la base y el registro de personas salen: las etapas las reemplazan. Lo que falló en la lectura se puede volver a leer desde su etapa con la misma acción de «Problemas».
- **Quién decidió:** agente sin consulta.

### D-426 · La línea del avance va fija al pie de la ventana, debajo del menú lateral, y dice «Todo al día» cuando no hay trabajo

- **Fecha:** 21 oct 2026 · **Tema:** Interfaz · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la propuesta aceptada pedía una línea fija al pie del menú lateral («Procesando: etapa, n de N», con un ícono girando) que lleve a Atención → «En curso». El menú lateral de hoy es una columna de íconos de 48 píxeles: una frase no entra.
- **Qué se decidió y por qué:** la línea va fija al pie de la ventana, de lado a lado, empezando debajo del menú, visible desde cualquier pantalla. Dice, en este orden: el último error del modelo, si una etapa está detenida por él, con cuándo vuelve a intentar; o «Procesando: etapa, n de N» con el ícono girando; o que la lectura automática está en pausa (o que falta elegir un modelo) con cuántos documentos esperan; o «Todo al día». No desaparece cuando no hay trabajo, para que no salte el contenido; no se muestra mientras no hay ninguna carpeta incorporada. Un clic lleva a Atención → «En curso».
- **Quién decidió:** agente sin consulta.

### D-427 · Lo que hace el motor y el último error del modelo se guardan en memoria; el error se dice en palabras en la etapa que preguntó

- **Fecha:** 21 oct 2026 · **Tema:** Atención · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el motor no exponía qué estaba haciendo ni sus errores: los del modelo (límite de uso, clave, conexión) solo iban a la consola y la lectura esperaba 10 minutos en silencio.
- **Qué se decidió y por qué:** el motor guarda en memoria, mientras la app está abierta, la carpeta que recorre, el archivo que lee, la etapa en la que está preguntando, el documento de la pregunta en curso, cuántas preguntas le quedaban a cada etapa y el último error del modelo; un comando de solo lectura lo devuelve junto con las cantidades. El error se clasifica por las palabras de su mensaje en límite de uso, clave o sesión rechazada, sin conexión (o sin respuesta a tiempo) u otro, y guarda la hora y cuándo vuelve a intentar (la espera de 10 minutos que ya existía). Se muestra en la etapa que preguntó; si el modelo ni siquiera arrancó, en la etapa que iba a preguntar. Se borra con la próxima respuesta del modelo. No se guarda en la base: no hace falta después de cerrar la app y no cambia el esquema.
- **Quién decidió:** agente sin consulta.

### D-428 · El reconocimiento de escaneos en la base va antes del registro de motores, sobre el contrato `TextRecognizer`

- **Fecha:** 21 oct 2026 · **Tema:** Reconocimiento · **Estado:** en revisión
- **El problema:** al retirar el wiki, los documentos que son imagen quedaron sin leer (D-420). El dueño agregó un caso real con muchos escaneos y pidió recuperar el reconocimiento ya. El plan de `docs/design/servicios-de-ia-propuesta.md` ponía antes el registro de servicios y motores (paso 1, P-141).
- **Qué se decidió y por qué:** se invierte el orden: la base reconoce los escaneos con el motor y la configuración que existen hoy (Codex), siempre a través del contrato `TextRecognizer`, de modo que el registro de motores se sume después sin rehacer nada. El documento de diseño no se edita en este paso: el cambio se le propone al dueño aparte.
- **Quién decidió:** el dueño pidió recuperar el reconocimiento (21 oct); el orden lo propuso el agente principal.

### D-429 · El motor que reconoce escaneos es el modelo elegido en Ajustes cuando es Codex; la base lo guarda aparte del de las preguntas

- **Fecha:** 21 oct 2026 · **Tema:** Reconocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la incorporación retirada tomaba el motor del ajuste «descripción de imágenes» (multimodal) cuando usaba el modelo principal y ese modelo era Codex. Ese ajuste salió de Ajustes con el retiro del wiki (D-419), así que no quedaba de dónde tomarlo.
- **Qué se decidió y por qué:** el motor que reconoce escaneos es el modelo principal de Ajustes → LLM Models cuando es «Codex CLI (local)», con su modelo; con otro proveedor no hay motor (hoy solo Codex lee imágenes desde la base). La app se lo pasa a la base cada vez que cambia el modelo, esté o no activa la lectura automática (una persona puede pedir un reconocimiento en cualquier momento), y la base lo guarda en su propio ajuste (`recognition-engine`), separado del motor de las preguntas: así el registro de motores (P-141) podrá elegir uno distinto por servicio sin tocar la base. Sin motor, la etapa y el documento lo dicen en palabras, con el camino a Ajustes, y los botones quedan apagados.
- **Quién decidió:** agente sin consulta.

### D-430 · Los escaneos se reconocen antes que cualquier otra pregunta, de a un documento; lo pedido va primero; una falla del proveedor espera y una del documento lo marca

- **Fecha:** 21 oct 2026 · **Tema:** Reconocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464); reemplazada en parte por D-452 (lo que una persona pide reconocer ya no vive en memoria: se guarda en la base; lo demás sigue valiendo)
- **El problema:** la especificación (2.2) dice qué documentos y cuándo, pero no en qué orden respecto de las demás preguntas al modelo, qué pasa cuando falla, ni dónde se guarda lo ya leído de un escaneo largo. Sin cambiar el esquema de la base.
- **Qué se decidió y por qué:** el reconocimiento corre en el mismo hilo que las preguntas al modelo y va primero, de a un documento, porque sin texto el documento no puede seguir el camino normal; después siguen tipo, lectura, personas y casos. Lo que una persona pide (un documento, todos o los que fallaron) va antes que lo demás y corre aunque la lectura automática esté apagada; el pedido vive en memoria mientras la app está abierta (si se cierra antes, el documento sigue esperando). Las páginas ya leídas se guardan junto a la base, en `recognized/<huella del archivo>/`, con el motor que las leyó: si un escaneo largo falla a mitad, el reintento lee solo lo que faltaba, y otro motor no usa lo de otro. Una falla del proveedor (límite de uso, sin conexión, no responde) deja el documento esperando y la etapa muestra el error con la hora en que vuelve a intentar (la espera de 10 minutos de siempre); si se repite tres veces con el mismo documento, o si la falla es del documento (no se pudo convertir una página, no se encuentra el archivo), el documento queda marcado con el motivo, que se dice en palabras, hasta que alguien toque «Reintentar». Un documento ya reconocido por un motor no se vuelve a leer con otro salvo que una persona lo pida. Las páginas que esperan se cuentan en segundo plano y se guardan en memoria, para mostrar el tamaño del trabajo antes de activarlo. Como anticipaba D-425, los escaneos que esperan cuentan ahora como los demás documentos: «en curso» si la etapa avanza, «en pausa» si la lectura automática está apagada o falta el motor.
- **Quién decidió:** agente sin consulta.

### D-431 · El documento dice qué motor reconoció su texto y cuándo, y las páginas con partes «[ilegible]»; las marcas de cada página no se muestran

- **Fecha:** 21 oct 2026 · **Tema:** Reconocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la especificación pide mostrar «Texto reconocido por <motor> el <fecha>» y las páginas de baja confianza si el motor las informa. Codex no informa confianza; sí escribe «[ilegible]» en cada carácter que no distingue. El texto guardado lleva además una marca técnica antes de cada página reconocida.
- **Qué se decidió y por qué:** en «Datos y correcciones», primero va un recuadro: para un escaneo sin reconocer, qué le pasa, lo que cuesta, el motivo del último intento fallido y el botón «Reconocer el texto» (o «Reintentar»); para uno reconocido, «Texto reconocido por Codex (modelo) el <fecha>», las páginas donde el motor escribió «[ilegible]» con el consejo de compararlas con el original, y «Reconocer de nuevo con <motor>» solo si hoy está elegido otro motor. La ficha de la derecha repite el motor y la fecha. Las marcas técnicas de cada página no se muestran en el texto: el recuadro ya dice quién lo leyó. Cuando un motor informe confianza de verdad (Core, Textract), se suma sin cambiar la pantalla.
- **Quién decidió:** agente sin consulta.

### D-432 · Un cambio de instrucciones no relee nada por su cuenta; se relee a pedido

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** en revisión; ampliada en D-465 (siempre hay una opción para releer todo)
- **El problema:** cambiar el catálogo de predicados o la pregunta de lectura cambiaba la huella de la lectura, y el trabajo en segundo plano daba por vieja toda lectura y le volvía a preguntar al modelo por todos los documentos del archivo. Con los parentescos de esta corrida habrían sido cientos de preguntas que nadie pidió.
- **Qué se decidió y por qué:** una lectura que ya existe sigue valiendo. El trabajo en segundo plano le pregunta al modelo solo por los documentos sin lectura, con el texto cambiado o con una lectura fallida con instrucciones anteriores. Las lecturas hechas con una versión anterior de la pregunta o del catálogo quedan marcadas («leído con una versión anterior», en el documento y en «En curso»). Se separan dos cosas: volver a controlar una lectura con las respuestas guardadas (automático, sin modelo: cuando cambia lo que el programa controla o sus palabras) y volver a preguntarle al modelo (solo a pedido). El pedido está en «Datos y correcciones» de cada documento, en Orígenes para cada origen y en Atención → «En curso», etapa de lectura, para la carpeta elegida o para todo; siempre con un aviso de cuántos documentos y cuántas preguntas son. Se guarda en la base (sobrevive al cierre), pide respuestas nuevas (no usa las guardadas de esa pregunta), va antes que lo demás y corre aunque la lectura automática esté en pausa; cuando no queda ninguno, las identidades se vuelven a calcular sin preguntar. Cubre en parte P-090. Medido sobre la copia de la base: con el catálogo nuevo, 0 documentos para preguntarle al modelo y 134 lecturas controladas de nuevo en 12 segundos.
- **Quién decidió:** agente principal, por pedido del dueño de releer solo la sucesión (21 oct).

### D-433 · La cita de una relación entre personas nombra a las dos; parentescos y herencia en el catálogo, guardados una sola vez

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** al anotar dos actas de matrimonio el modelo corrió las personas un lugar y salieron cónyuges que no lo son (un hijo con su padre, una madre con su hijo). El programa solo controlaba que la cita nombrara a la segunda persona de la relación. No había parentescos ni herencia.
- **Qué se decidió y por qué:** un indicador nuevo del catálogo (`subjectInQuote`) pide que la cita nombre también a quien se le atribuye la relación (cónyuge, conviviente, parentescos, causante); si no, se descarta. La primera persona («mi», «el suscripto») nombra a quien firma el documento. Para el cónyuge, además (`notBetween`), si entre los dos nombres hay «hijos», «nacieron»… y ninguna palabra de matrimonio, no son cónyuges: en «de la unión de A y B nacieron sus hijos, a saber: C», C es hijo. Entran al catálogo y a las etiquetas (castellano e inglés): conviviente, padre o madre, hijo, hermano, abuelo, nieto, tío, sobrino, causante (de la persona al expediente sucesorio), heredero y heredero por representación. «Hijo de», «nieto de» y «sobrino de» se guardan como la relación del otro lado (`inverseOf`) y se muestran en los dos sentidos; las etiquetas no dicen el género («Padre o madre de») porque la base no lo sabe. El costo: algunas relaciones verdaderas cuya cita nombra al sujeto solo por su papel («la causante», «estado: casado con») se pierden hasta releer con la pregunta nueva, que pide citas con los dos nombres. Las relaciones nuevas aparecen al releer con el modelo.
- **Quién decidió:** agente sin consulta.

### D-434 · Una relación queda dudosa si sus citas nombran a otros y no a quien se le atribuye, o si el mismo documento dice un estado civil que no corresponde

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la duda «la cita nombra a otra persona u organización» (D-320) solo se aplicaba a los datos con valor, no a las relaciones. Una persona soltera según el mismo documento podía quedar cónyuge de alguien sin aviso.
- **Qué se decidió y por qué:** en las relaciones marcadas en el catálogo (`attribution`: representación, sociedad, beneficiario final, empleo, deuda, parte en, herencia), si todas sus citas nombran a otras personas u organizaciones y no al sujeto (sin contar al otro lado de la relación), la relación queda «sin confirmar». Las palabras de quien firma (`firstPerson`: «mi», «el suscripto», «la suscripta») nombran a quien tiene en el documento un papel de firmante, apoderado, remitente o emisor (`signerRoles`). El cónyuge (`contradictedBy`) queda dudoso, con un motivo nuevo («el mismo documento dice un estado civil que no corresponde»), cuando en cada documento que lo dice ese mismo documento da a una de las dos personas un estado civil de la lista, por idioma (castellano «solter…»; inglés «single», «unmarried»). Se pide que lo digan todas sus citas para no dudar de un matrimonio que otro documento confirma; un acta de matrimonio que dice «soltero» de los contrayentes también lo marca, y una persona lo confirma con un clic. La resolución sube de versión.
- **Quién decidió:** agente sin consulta.

### D-435 · «Representa a» necesita sus palabras donde lo dicen; un juzgado no representa ni es parte; heredar «en representación» no es representar

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** «representa a» salió para un juzgado que le pide un informe a otro, para un abogado que cita la matrícula de su colegio y para nietos que heredan «en representación de su padre prefallecido»; un juzgado salió como parte de un expediente.
- **Qué se decidió y por qué:** señales generales en el catálogo, nunca reglas por caso. Las palabras del vínculo (`leadCues`: «apoderado», «en representación de», «presidente»…) van antes del nombre del representado en la cita, o (`trailCues`: «representada por») después; una cita que nombra al otro lado solo lejos de ellas (el colegio de la matrícula antes de «en representación de») no lo dice. Un nombre con palabras de un tribunal (`subjectNot`/`objectNot`: «juzgado», «tribunal», «corte suprema», «fiscalía»…) no representa ni es representado, ni es parte en un asunto. «Prefallecido», «premuerto», «fallecido con anterioridad», «padre fallecido», «derecho de representación» dicen herencia por representación: la lectura lo conserva, pero no es una representación. Una palabra del catálogo de varias seguidas vale como frase. Las descripciones que lee el modelo dicen lo mismo.
- **Quién decidió:** agente sin consulta.

### D-436 · Un identificador fuerte escrito con más o menos caracteres no separa dos fichas

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** un abogado quedó partido en tres fichas y el causante en dos. La causa, en la copia de la base: la regla «otro identificador fuerte del mismo tipo separa para siempre» comparaba los valores tal cual, y eran el mismo escrito de otra manera: un CUIT sin los dos dígitos de su tipo (cortado por un salto de línea), una matrícula con la sigla del colegio al final, un documento con la letra de su clase delante, y una matrícula con «º» en lugar de «°».
- **Qué se decidió y por qué:** dos valores del mismo tipo de los que uno contiene entero al otro, con seis dígitos o más el más corto, no separan: no unen por sí solos (une el nombre, como siempre). La forma normalizada de un identificador no tiene los signos ordinales («º», «ª»). Con la copia, el abogado y el causante quedan en una ficha cada uno, y otro abogado partido en dos por el «º» también. Lo que sigue partido son organismos nombrados con una abreviatura o con su nombre corto y largo (P-255).
- **Quién decidió:** agente sin consulta.

### D-437 · Un ítem de una lista se encuentra donde la lista lo tiene, con su nombre en singular o plural

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** Atención mostraba «1 documento perdió muchas citas al leerlo». No era un escaneo: es un documento de texto que rechaza una lista de facturas. El modelo copió cada factura con la palabra de la lista delante («facturas N2 con fecha…», que el texto no tiene así) y la nombró en singular («factura N2»), y el control descartó seis de siete cosas.
- **Qué se decidió y por qué:** la cita de una mención cuya primera palabra encabeza una lista de números, y cuyo resto (que empieza con un número) está tal cual en el texto a menos de `labelWindow` palabras de esa palabra, se encuentra ahí. Un nombre de dos palabras o más está en su cita también si alguna de sus palabras está en plural («factura» y «facturas»). Con la copia, el aviso desaparece y el documento conserva sus facturas. Las citas de afirmaciones siguen pidiendo el texto tal cual.
- **Quién decidió:** agente sin consulta.

### D-438 · Cada familia de la misma plantilla tiene su plantilla guardada en la base (versión 21), también las propuestas

- **Fecha:** 22 oct 2026 · **Tema:** Parecidos · **Estado:** vigente
- **El problema:** la base detectaba que dos documentos comparten la plantilla («Misma plantilla, otros datos») y los agrupaba en familias, pero no armaba ni guardaba la plantilla: el texto común, los campos que cambian y el valor de cada documento (P-247). Ninguna familia de la copia está confirmada por una persona.
- **Qué se decidió y por qué:** la base pasa a la versión 21 con cuatro tablas nuevas: la plantilla de cada familia (por su documento principal, con su texto y una huella de lo que la armó), sus documentos, sus campos y el valor de cada documento en cada campo. Se arma también para las familias que nadie confirmó, y la pantalla la muestra como «Propuesta» hasta que se confirme la familia. Se vuelve a armar sola cuando cambia la familia (otro documento, otro texto, otro motor); si no cambió nada, no se toca.
- **Quién decidió:** el dueño del producto, 21 oct.

### D-439 · La plantilla se arma alineando cada documento con el principal, sin modelo; sus campos se nombran por su clase y se ve desde el documento, la familia y Documentos

- **Fecha:** 22 oct 2026 · **Tema:** Parecidos · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464); ampliada en D-466 (sección «Plantillas» y generar documentos)
- **El problema:** hacía falta decidir cómo se arma la plantilla sin preguntarle a un modelo, qué documentos de la familia entran, cómo se nombran los campos y desde dónde se llega a verla.
- **Qué se decidió y por qué:** cada miembro se alinea palabra por palabra con el documento principal de su familia (la misma comparación de parecidos). Una palabra del principal es de la plantilla cuando todos los miembros la tienen entera y en el mismo lugar; entre dos palabras de la plantilla, lo que dice cada documento es el valor de un campo si no todos dicen lo mismo (un tramo que solo uno tiene es un campo vacío en los demás). Para que un campo sea un dato y no pedazos: un nombre con una palabra corta en el medio («de») es un solo campo; los signos que todos los valores tienen en el borde («$», el punto final) quedan en la plantilla; el mismo valor dicho otra vez (el número de una boleta en cada talón) es el mismo campo. Los campos llenados de un formulario son campos de la plantilla, con su nombre en el formulario al lado. Entran el principal y los miembros cuya relación con él es «Misma plantilla, otros datos»: una versión o un ejemplar firmado del principal es el mismo documento, no otros datos. Cada campo se nombra por la clase de dato que es, como ya la distingue la comparación (identificador, importe, fecha, número, nombre, campo de formulario o texto), con su orden: «Fecha 2»; las palabras salen del vocabulario de la vertical. Se ve en tres lugares: en «Datos y correcciones» del documento, arriba de sus documentos parecidos («Comparte la plantilla con N documentos más · Ver la plantilla»); en la pantalla de documentos parecidos, por familia (adonde lleva Atención); y en Documentos, al filtrar por una familia. La plantilla muestra el texto común con los campos resaltados (con sus nombres o con los datos del documento que se elija) y una tabla campo por documento. Medido sobre la copia de la base: 12 de las 19 familias comparten plantilla, con 188 campos en total (de 2 a 30 por plantilla); armarlas todas lleva medio segundo y revisarlas sin cambios, 2 milésimas.
- **Quién decidió:** agente sin consulta.

### D-440 · Dos documentos de una plantilla en los que no cambia ningún dato son posiblemente el mismo documento

- **Fecha:** 22 oct 2026 · **Tema:** Parecidos · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** dos documentos sin ningún campo que cambie, o con cambios mínimos de redacción, no son «la misma plantilla con otros datos» (P-218): son posiblemente el mismo documento y una persona tiene que revisarlo.
- **Qué se decidió y por qué:** dentro de cada plantilla se comparan los documentos de a dos. Si no difieren en ningún campo, o difieren en tres palabras o menos y ninguna es un dato (una palabra con mayúscula cuenta como nombre y un campo de formulario que cambia, como dato), el par queda marcado «posiblemente el mismo documento». Se avisa en la plantilla, en la familia y en Atención, junto con los pares que podrían ser el mismo documento; el aviso se va cuando una persona decide ese par (lo une o lo descarta). En la copia de la base no aparece ninguno: el par de boletas de P-218 cambia en número, fecha y código de barras, así que son dos boletas distintas.
- **Quién decidió:** agente sin consulta.

### D-441 · Las citas del artículo son notas numeradas que se tocan; «Referencias» y «Otros datos» salen del artículo y los documentos se listan una vez

- **Fecha:** 22 oct 2026 · **Tema:** Páginas · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el artículo de una persona u organización repetía tres veces lo mismo: la síntesis con sus números «[1, 2]» en texto plano, «Referencias» con cada dato, su cita y su documento, y «Otros datos de los documentos» con lo que la síntesis no usó; «Datos y correcciones» ya muestra cada dato con su cita y sus acciones, y «Documentos» lleva a los mismos documentos que las referencias (P-245). El caso repetía cada documento en «Por qué estos documentos van juntos»; el documento ponía la cita entera en cada punto de «Lo esencial».
- **Qué se decidió y por qué:** cada cita en que se apoya el texto es una nota numerada, como en una enciclopedia: el número va donde el texto lo dice y la nota (el dato, si está sin confirmar, cada cita con su documento) va una sola vez al final de la página, con la forma de las notas de markdown (`[^1]`); la misma nota lleva el mismo número y se numeran en el orden en que el texto las nombra. En el artículo el número se toca y muestra la nota al lado, y el documento se abre desde su nombre; el número es un elemento con su nota, para la vista previa flotante que viene después (P-246). Vale para la síntesis y los datos en conflicto de persona u organización, sus «Vínculos», «Lo que dicen los documentos del asunto» del caso y «Lo esencial» del documento. «Referencias» y «Otros datos» salen del artículo: en su lugar una línea dice cuántos datos más hay en «Datos y correcciones», con su cita (reemplaza en parte D-236: nada se pierde, pero no se repite); sin síntesis escrita, la frase de espera dice lo mismo. «Documentos» queda una vez, por caso; el caso dice en una línea por qué van juntos sus documentos (cuántos por cada razón) y cada documento con sus razones queda en «Datos y correcciones», que ya lo mostraba con la forma escrita de lo que comparten (reemplaza en parte D-268). Sin la frase sobre el modelo ni sobre el cálculo de parecidos (P-208): lo propuesto dice «sin confirmar». La pregunta de la síntesis no cambia: rearmar las páginas sale de las respuestas guardadas. Medido sobre la copia de la base, sin modelo: las preguntas de síntesis sin respuesta guardada son las mismas 19 antes y después (las dejan los pasos anteriores, por los datos re-controlados, P-260); con esas respondidas, escribir de nuevo todas las páginas (134 de personas, organizaciones y casos, y 109 de documentos) no hace ninguna pregunta; y ninguna página dice ya «Referencias», «Otros datos» ni la frase sobre el programa.
- **Quién decidió:** agente sin consulta.

### D-442 · La página de un documento se titula por su tipo, número, partes y fecha; sus fechas, en castellano; el nombre del archivo queda en los datos

- **Fecha:** 22 oct 2026 · **Tema:** Páginas · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la página de un documento se titulaba con el nombre de su archivo sin la extensión (D-340), que a veces no dice qué es («Summary», «Screenshot…», un código de expediente) (P-182); y sus fechas quedaban como las escribió el documento o la lectura, en inglés o con números y separadores (P-208).
- **Qué se decidió y por qué:** el título sale de lo que la base ya sabe del documento, sin el modelo: el nombre de su tipo (el propuesto con un tipo nuevo, o el del catálogo), su número cuando el catálogo de su tipo le da uno corto, de quién es y a quién va (o entre quiénes es, si no dice ninguna de las dos), con los papeles del vocabulario (`titleRoles`) y dos nombres por lado como mucho, y su fecha cuando empieza por el día o el mes: «Factura 0001-00000123 de Ejemplo Uno S.A. a Persona Dos · 16 de abril de 2026». Sin tipo, el tema de su resumen; sin nada, el nombre del archivo. Dos páginas que se leerían igual dicen además el nombre de su archivo entre paréntesis. El título va en la página, en los enlaces de un documento a otro y arriba de su pantalla; el nombre del archivo queda en su cabecera (`file:`), en el recuadro de datos y en las listas de documentos de las demás páginas, que siguen nombrando cada archivo como se lo conoce en su carpeta. La ruta de la página no cambia (sigue saliendo del nombre del archivo): sus enlaces no se rompen. Las fechas de la página del documento se escriben como el resto de las páginas («16 de abril de 2026»): la del documento, la de sus datos de fecha, los puntos de su resumen y su tema; las citas quedan como las escribe el documento. Medido sobre la copia: de 109 páginas de documentos, ninguna queda titulada solo por su archivo, 69 dicen su fecha y 13 que se leerían igual llevan además el nombre del archivo (P-259).
- **Quién decidió:** agente sin consulta.

### D-443 · Un registro de servicios de IA en la base dice qué motor usa cada servicio; la base lo lee en cada pedido y la configuración del chat y de la búsqueda son copias que el registro escribe

- **Fecha:** 22 oct 2026 · **Tema:** IA · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la especificación (2.1, paso 1 del plan, P-141) pide un lugar único en Ajustes con un renglón por servicio (preguntar al modelo, reconocer escaneos, investigar en internet), con su motor, su configuración y un motor alternativo, y un registro único en la base o en los ajustes de la app, sin cambiar lo que hoy funciona. Hasta ahora la base guardaba dos motores sueltos (`ask-engine`, `recognition-engine`) que la pantalla le mandaba derivados del modelo del chat: el de las preguntas solo con la lectura automática encendida y el de los escaneos cada vez que cambiaba el modelo. El chat y la investigación corren en la pantalla con los ajustes de la app (la conexión activa, el buscador activo).
- **Qué se decidió y por qué:** (1) El registro vive en la base (`docbase_setting`, `ai-services`), porque quien más lo usa es la base, que pregunta y reconoce sin la pantalla, y porque así un cambio se usa en el pedido siguiente sin que la pantalla tenga que mandar nada. Tiene tres servicios; cada uno, un motor y un alternativo, cada uno con su conexión (los datos del proveedor, que siguen en Conexiones), su modelo y su razonamiento. (2) La base pregunta y reconoce solo con un motor que puede correr (hoy Codex); otro motor elegido es, para la base, un servicio sin motor, como pasaba antes con un modelo que no era Codex. Los contratos (`DocumentAsker`, `TextRecognizer`) y el nombre con que se guarda cada respuesta no cambian. (3) El chat y la investigación siguen leyendo la conexión activa y el buscador activo: son copias que el registro escribe cuando cambia y, al abrir, si no coinciden. Activar una conexión en «Conexiones con modelos» o un buscador en «Búsqueda en internet» es elegirlo para ese servicio, y cambiar el modelo de la conexión en uso cambia el del servicio, así nunca hay dos verdades. Los selectores del modelo del chat y de la incorporación salen de Conexiones; si el chat estaba en otra conexión que la activa, la conserva y Servicios de IA lo dice, con «Que use el mismo». (4) Al abrir la primera vez, el registro se arma con los ajustes como están: los mismos motores y modelos (con la configuración del dueño, Codex con su modelo y razonamiento alto para preguntar y para los escaneos, y ningún buscador). Hasta entonces la base sigue con sus dos ajustes viejos, que quedan como estaban. Un recorrido que pide una persona ya no reescribe el motor de la base.
- **Quién decidió:** agente sin consulta.

### D-444 · Las claves de API van al llavero del sistema; se mueven una vez al abrir, cada una recién después de confirmar que el llavero la guardó, y sin llavero quedan en el archivo

- **Fecha:** 22 oct 2026 · **Tema:** IA · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** las claves de los proveedores estaban en texto plano en `app-state.json` (`llmConfig.apiKey`, `providerConfigs[*].apiKey`, `searchApiConfig`). La especificación pide pasarlas al llavero del sistema una vez, sin perder ninguna, y que el archivo deje de tenerlas.
- **Qué se decidió y por qué:** (1) Se usa el llavero del sistema (Credential Manager en Windows, con el paquete `keyring`), con una entrada por lugar: la del modelo principal, una por conexión, la de la búsqueda y una por buscador. (2) Al abrir, antes de que la pantalla lea los ajustes, cada clave que siga en el archivo se guarda en el llavero, se vuelve a leer y solo si es la misma se vacía en el archivo; el archivo se guarda solo si se movió alguna. Una que el llavero no guardó queda donde estaba, y el registro dice cuántas, nunca su valor. (3) La pantalla lee las claves al abrir y guarda cada cambio en el llavero; una clave que el llavero no guarda queda en el archivo, así que sin llavero todo sigue como antes. Una clave escrita en el archivo vale más que la del llavero: está ahí porque el llavero no pudo guardarla. (4) El chat lee la clave de la búsqueda desde el llavero. (5) Los campos con claves de las funciones retiradas del wiki (lectura de PDF con MinerU, descripción de imágenes, vectores, API local) no los lee nada y quedan como están (P-262). Las pruebas usan un llavero falso; el real no se toca.
- **Quién decidió:** agente sin consulta.

### D-445 · El catálogo de motores dice qué sabe hacer cada uno, dónde procesa y qué guarda su proveedor; el motor alternativo se guarda pero la base todavía no lo usa sola

- **Fecha:** 22 oct 2026 · **Tema:** IA · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la especificación pide capacidades declaradas por motor (texto, tablas, casillas, manuscrito, idiomas) aunque hoy solo exista Codex, y mostrar dónde se procesa el documento y qué guarda el proveedor.
- **Qué se decidió y por qué:** el catálogo es un dato, no código: `src-tauri/resources/catalog/ai-engines.json`, compilado en la app y servido a la pantalla con el registro. Una entrada es un motor para un servicio: Codex para preguntar y para reconocer escaneos (el único que la base corre sola), Claude Code y las API de proveedores para preguntar (las usa el chat), y los buscadores para investigar. Las capacidades se declaran donde importan, al reconocer escaneos: Codex, texto impreso y tablas sí, casillas y manuscrito a veces; se confirman con la comparación con escaneos reales (paso 6 del plan). Los textos de dónde se procesa y qué guarda cada proveedor son informativos y generales («según las condiciones de…») cuando dependen del plan de cada cuenta; conviene que el dueño los revise. El motor alternativo se elige y se guarda, pero la base todavía no lo usa sola: con un solo motor no hay con qué cubrir lo que el principal no hace (P-261).
- **Quién decidió:** agente sin consulta.

### D-446 · «Portales» en el menú con un navegador embebido que maneja la app; la ventana se habilita solo para el desafío o el inicio de sesión; novedades a pedido; cada expediente es un caso confirmado

- **Fecha:** 21 oct 2026 · **Tema:** Portales · **Estado:** vigente
- **El problema:** la propuesta de traer expedientes de portales judiciales (P-248, `docs/design/portales-judiciales-propuesta.md`) dejaba abierto cómo se usa: dónde está, quién maneja el navegador, cuándo se actualiza y qué queda en la base.
- **Qué se decidió y por qué:** una opción nueva «Portales» en el menú izquierdo, con una página que tiene un navegador embebido. La app maneja el portal; la ventana se habilita para una persona solo cuando hace falta (el desafío anti-robots o el inicio de sesión). Fuera de eso el navegador solo sirve para recorrer las causas, detectar lo nuevo y bajarlo. Las novedades se traen a pedido («Actualizar»). Cada expediente traído crea un caso confirmado. Se guarda en `<carpeta de trabajo>\Expedientes\PJN\<jurisdicción> <número>-<año>\`. La primera prueba real es con el dueño (el portal pide un desafío: no se puede probar sin una persona).
- **Quién decidió:** el dueño del producto, 21 oct.

### D-447 · La ventana del portal es una vista hija sin ningún permiso de la app; la app la lee con guiones propios evaluados con retorno, y la bloquea con una capa que solo frena lo que hace una persona

- **Fecha:** 22 oct 2026 · **Tema:** Portales · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** D-446 pide un navegador dentro de la página que la app maneje sola, que no reciba clics ni teclado mientras trabaja y que nunca pueda llamar a la app, sin guardar contraseñas.
- **Qué se decidió y por qué:** (1) la ventana es una vista web hija de la ventana principal (`add_child`, con la característica `unstable` de Tauri), puesta sobre el área que la página le deja, con su propio perfil en `<datos locales de la app>\portales\pjn\perfil`: la sesión del portal queda ahí, como en cualquier navegador; la app no pide ni guarda contraseñas. (2) Sin permisos: los permisos de la app siguen siendo solo para la ventana `main` y nunca para una página remota; Tauri 2.11 rechaza los comandos de la app a una página remota que ningún permiso nombra, y además todo comando de la app se rechaza si no viene de la vista de la app (`lib.rs`); la vista solo puede ir a direcciones `https` de `pjn.gov.ar`, no abre ventanas nuevas y solo deja pasar la descarga que la app pidió. (3) Para leer la página, guiones propios y chicos evaluados con `eval_with_callback` (en Windows, `ExecuteScript` de WebView2, que devuelve el último valor como JSON): la página nunca le manda nada a la app. El guion devuelve el HTML; la lectura (rótulos, tabla, paginador) se hace en Rust con `dom_query`, que Tauri ya compila. Antes de actuar, el guion marca en la página lo que la app puede tocar (`data-micelya`): lo que se lee y lo que se toca es el mismo elemento; mientras una persona resuelve el desafío, la app solo mira (no marca nada). (4) El bloqueo es una capa sobre la página y un filtro que frena los clics y teclas de una persona (los eventos `isTrusted`), instalados en cada página antes que sus propios guiones; cada página nueva empieza bloqueada y la app la desbloquea cuando detecta el desafío o el inicio de sesión por sus rótulos («Ver desafío», un campo de contraseña visible), con un aviso en la página de la app. (5) Con la vista adentro, la ventana principal deja de ser «de una sola vista»: la bandeja y la reapertura la buscan como ventana (`get_window`). (6) «Usar la ventana yo» deja a la persona usarla cuando la app no está trabajando, por si el portal muestra algo que la app no reconoce.
- **Quién decidió:** agente sin consulta.

### D-448 · Cada documento se guarda con el nombre de la convención y sus datos al lado en `.micelya.json`; no se baja lo que la app ya bajó ni lo que la base ya tiene con ese nombre

- **Fecha:** 22 oct 2026 · **Tema:** Portales · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** hay que nombrar cada documento, guardar sus datos sin que la base los tome por documentos, reanudar un recorrido cortado y no bajar dos veces lo mismo, y la carpeta de trabajo todavía no existe (la agrega el paso 8).
- **Qué se decidió y por qué:** hasta el paso 8 la carpeta es `<Documentos>\Micelya\Expedientes`, resuelta en una sola función (`expedientes_folder`) para que el paso 8 la conecte. El nombre es `<JUR><número>-<año>_fs<fojas>_<aaaa-mm-dd>_<TIPO>_<detalle>.pdf`: las fojas como sus números unidos por guion («146 / 148» es `fs146-148`), tipo y detalle sin acentos ni signos, el detalle hasta 60 letras, y `_2` si el nombre ya existe. Al lado, `<mismo nombre>.micelya.json` con fecha, fojas, tipo, detalle, oficina, dirección de origen, fecha de descarga y tamaño; en la carpeta, `expediente.micelya.json` con carátula, jurisdicción, dependencia, situación, vinculados y cada actuación con su estado (sin documento, pendiente, bajado, ya estaba, falló): es lo que hace el recorrido incremental y reanudable, y se escribe después de cada documento. La base no inventaría lo que termina en `.micelya.json`. Cada descarga cae primero en `PJN\.descargas` (fuera del inventario) y solo pasa a la carpeta si es un PDF no vacío; si no (por ejemplo, una página de sesión vencida), queda «falló» y se intenta en la próxima actualización. No se baja lo que la app ya bajó y sigue en la carpeta con su tamaño, ni lo que la base ya tiene en cualquier origen con el nombre de la convención del mismo expediente, misma fecha, tipo y primera foja (también el detalle si otras actuaciones comparten esos datos) y tamaño mayor que cero. Lo de «Notas» no se lee ni se guarda nunca.
- **Quién decidió:** agente sin consulta.

### D-449 · El recorrido pasa por todas las páginas y baja solo lo nuevo; se detiene y retoma ante el desafío; el expediente entra a la base como origen «Expedientes» y caso confirmado

- **Fecha:** 22 oct 2026 · **Tema:** Portales · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** falta decir cómo recorre la app un expediente de muchas páginas, qué hace si el portal la interrumpe y cómo queda el caso en la base.
- **Qué se decidió y por qué:** si la vista ya muestra el expediente pedido se queda; si no, completa el formulario (jurisdicción por su código en la lista del portal, número y año) y toca «Consultar»; si el formulario pide el desafío, o no avanza después de «Consultar», la ventana pasa a la persona hasta que la página cambia o la persona toca «Ya lo resolví, seguir». Recorre siempre todas las páginas, de a un pedido con 0,7 s entre dos, y baja solo lo nuevo: lo nuevo puede estar en cualquier página y pasar de página es barato. El «Siguiente» se busca siempre desde la tabla de actuaciones (la de «A FS.») hasta la tabla que sigue, nunca por número; termina cuando no hay «Siguiente», cuando la página no cambia en 30 segundos o cuando vuelve una ya vista; una página que no es la última y no tiene 15 actuaciones se avisa. Si a mitad aparece el desafío o el inicio de sesión, se detiene, avisa, y al cambiar la página sigue desde donde quedó la vista sin repetir lo bajado; cada documento se intenta dos veces por recorrido; «Detener» deja todo guardado. Al terminar (aunque sea en parte), la carpeta de expedientes se suma como origen «Expedientes» si ningún origen la contiene (si la carpeta de trabajo ya es un origen, se usa ese), se barre, y se crea un caso confirmado, hecho por una persona, llamado «<carátula> (<JUR> <número>/<año>)»; sus documentos entran al caso a medida que la base los inventaría (al terminar y cada vez que se abre la lista de seguidos). Los vinculados se leen de la tabla rotulada «vinculados» antes de «Notas», solo sus referencias, y se ofrecen para traer. «Actualizar todos» los pone en fila, uno por vez; cada uno puede pedir el desafío.
- **Quién decidió:** agente sin consulta.

### D-450 · La carpeta de trabajo de Micelya está en Ajustes, es un origen de la base y tiene «Recortes web» y «Expedientes»; los recortes web se guardan ahí como archivos markdown

- **Fecha:** 21 oct 2026 · **Tema:** Carpeta de trabajo · **Estado:** vigente
- **El problema:** con el retiro del wiki (D-400) se fueron la carpeta de trabajo de los proyectos y el recortador web, que guardaba en los proyectos. Los portales (D-446) necesitan dónde guardar los expedientes, y el recortador, dónde dejar lo que se trae de la web (P-227).
- **Qué se decidió y por qué:** la carpeta de trabajo de Micelya vuelve a Ajustes. Por defecto es la que ya figuraba en los ajustes de la app, o una en Documentos. Se crea si no existe, con las subcarpetas «Recortes web» y «Expedientes», y se agrega como origen de la base una sola vez; si el dueño la cambia, se actualiza el origen. Una sola función resuelve estas carpetas para toda la app. El recortador web (la extensión, ahora «Micelya Clipper», y su servidor local) guarda cada recorte como archivo markdown en «Recortes web», con la dirección de la página y la fecha al principio; la base lo inventaría como cualquier documento y Atención avisa de los recortes nuevos. El servidor solo atiende lo necesario (estado y recorte), sin proyectos. La API local y el servidor MCP quedan para otra noche.
- **Quién decidió:** el dueño del producto, 21 oct.

### D-451 · La carpeta de trabajo es un solo origen: lo que la app guarda adentro nunca es un origen aparte, el origen «Expedientes» que hubiera se suma sin perder nada y, si la carpeta cambia, el origen se muda; el recortador escucha solo en la propia máquina

- **Fecha:** 22 oct 2026 · **Tema:** Carpeta de trabajo · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la base no acepta orígenes superpuestos, y el navegador de portales (D-449) sumaba la carpeta de expedientes como origen «Expedientes» si ningún origen la contenía: con la carpeta de trabajo como origen, uno quedaría dentro del otro. D-450 tampoco dice qué es «actualizar el origen» cuando la carpeta cambia, ni cómo se protege el servidor del recortador.
- **Qué se decidió y por qué:** (1) La carpeta se guarda en los ajustes de la app con la clave que ya tenía (`workspaceDir`), así la carpeta que ya figuraba sigue siendo la misma; sin ella, `<Documentos>\Micelya`. Una función (`work_folder.rs`) la resuelve para toda la app; los portales la usan para `Expedientes`. (2) La base recuerda cuál origen es el de la carpeta de trabajo. Si un origen ya la contiene, ese sirve y no se crea otro. Si no, se crea uno llamado «Carpeta de trabajo». Los orígenes que estén dentro de «Expedientes» o «Recortes web», o el que la carpeta tenía antes, pasan a ser parte de él: las mismas ubicaciones con su ruta debajo de la subcarpeta, con sus etiquetas, exclusiones e historial, así no se pierde el texto, la lectura ni los documentos de los casos. Cualquier otro origen adentro es del dueño: la carpeta no se toma y Ajustes dice cuál origen lo impide. Los portales ya no crean el origen «Expedientes» (reemplaza esa parte de D-449). (3) Si el dueño cambia la carpeta, el mismo origen pasa a la carpeta nueva: lo que está ahí con la misma ruta conserva todo lo que se sabía; lo que no está deja la base (los archivos no se tocan). Ajustes lo avisa antes de cambiar, y la carpeta solo se guarda si la base la acepta. (4) El servidor escucha solo en 127.0.0.1:19827, con `/status` y `/clip`. Un pedido que no venga de la propia máquina necesita el token de `MICELYA_CLIP_TOKEN`. Una página web no puede guardar recortes: se rechaza su origen y el recorte tiene que llegar como JSON, que el navegador no deja mandar desde otro sitio sin preguntar antes. La extensión ya no ofrece servidor ni token: solo habla con la propia máquina. (5) El archivo se llama `<aaaa-mm-dd> <título>.md` (sin signos que Windows no acepta, hasta 80 letras, y ` (2)` si ya existe) y empieza con «Dirección:» y «Fecha del recorte:». (6) Atención muestra «N recortes web nuevos» entre las decisiones, con cada recorte para abrir, hasta que alguien toca «Ya los vi» (el momento queda en la base).
- **Quién decidió:** agente sin consulta.

### D-452 · «Casos» y «Páginas» esperan la primera vuelta del motor en vez de figurar hechas; los pedidos de reconocimiento a mano se guardan en la base

- **Fecha:** 22 oct 2026 · **Tema:** Avance · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** al abrir la app, Atención → «En curso» mostraba «Casos» y «Páginas» como hechas aunque el motor todavía no hubiera pasado por ellas: lo que les falta (las preguntas al modelo para proponer el caso de los documentos sueltos y para escribir lo que dicen los documentos) se sabe recién cuando el motor las recorre, y esa cuenta vivía en memoria. Además, lo que una persona pedía reconocer a mano se perdía si cerraba la app antes (D-430). La especificación ofrecía mostrar «esperando la primera vuelta» o calcularlo desde la base.
- **Qué se decidió y por qué:** mostrar la espera. Hasta la primera vuelta del motor desde que se abrió la app, esas dos etapas no figuran como hechas: dicen «esperando la primera vuelta del motor para saber si queda algo por hacer», y quedan «En pausa» si la lectura automática está apagada o falta el motor, o esperando su turno o «En curso» si está activa; no suman documentos en proceso. Pasa solo cuando ya hay algo leído o la etapa tiene cosas propias: con la base vacía siguen hechas, sin nada. No se calcula desde la base porque contar esas preguntas es casi lo mismo que prepararlas (lo que escribe en la base), y la cuenta se pide cada tres segundos mientras se ve: sería lento y dejaría de ser de solo lectura. Una vuelta que el motor saltea porque falta resolver quién es quién no cuenta. La etapa de lectura no cambia: lo leído con una versión anterior sigue contado como leído y lo dice. Los pedidos de reconocimiento se guardan en la base (versión 22, una tabla con el orden en que se pidieron) hasta que el documento se reconoce o falla del todo; los intentos fallidos del proveedor siguen contándose en memoria. Esto reemplaza lo que D-430 decía del pedido en memoria.
- **Quién decidió:** agente sin consulta.

### D-453 · El chat adjunta documentos de la base con un buscador o con «@»; el agente lee su texto de la base con su nombre y su lugar, y los cita

- **Fecha:** 22 oct 2026 · **Tema:** Chat · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** el chat adjuntaba archivos del proyecto de wiki, que ya no existe, y el selector quedaba vacío (P-243). La especificación pide elegir documentos de la base con un buscador, que el agente reciba su texto desde la base (no un archivo del disco) con su nombre y su ubicación y lo cite, y «Preguntar al chat sobre este documento» desde el documento; no dice cómo se eligen ni cuánto texto va.
- **Qué se decidió y por qué:** en el cuadro del mensaje, el botón «Documento» abre un buscador (por nombre, carpeta o texto; vacío, los más recientes), y escribir «@» y parte del nombre ofrece lo mismo; solo aparecen documentos que ya tienen su texto en la base, uno por documento aunque tenga copias. Lo adjuntado queda en la conversación para las preguntas siguientes hasta que se quita, y el mensaje enviado lo muestra (tocarlo abre el documento). El agente recibe la ubicación elegida y lee de la base el nombre, «origen › carpeta» y el texto, sin las marcas del reconocimiento; hasta 8 documentos por mensaje, 20.000 caracteres de cada uno y 48.000 entre todos, y si se corta avisa que el resto se encuentra buscando en la base. Cada adjunto entra entre las referencias de la respuesta («Adjuntado al mensaje · lugar») y la instrucción pide citarlo por su nombre y su lugar. Con los proveedores de línea de comandos (Codex, Claude Code), que no corren el agente, la app arma el mismo bloque con el texto de la base. «Preguntar al chat sobre este documento» abre el chat con el documento adjunto, en una conversación nueva salvo que la abierta esté vacía, y está apagado mientras el documento no tiene su texto. Adjuntar personas, organizaciones y casos queda para más adelante (P-269).
- **Quién decidió:** agente sin consulta.

### D-454 · Las notas numeradas y los enlaces de los artículos muestran una tarjeta flotante al pasar el mouse o llegar con el teclado; tocar una nota la muestra y la deja, como antes; en pantalla táctil el primer toque de un enlace muestra su tarjeta y el segundo lo abre

- **Fecha:** 22 oct 2026 · **Tema:** Artículos · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** P-246 pide la vista previa flotante de las notas numeradas (D-441) y de los enlaces a personas, organizaciones, documentos y casos, con las sugerencias de `docs/design/hovercards-sugerencias.md` (demora al abrir, zona segura al cerrar, Escape, teclado, táctil, tamaño fijo, contenido sin preguntar al modelo). La especificación no dice qué hace un toque sobre una nota (que hoy muestra la cita) ni de dónde sale el resumen de un enlace.
- **Qué se decidió y por qué:** se usa `preview-card` de base-ui, envuelto como `tooltip` (`src/components/ui/preview-card.tsx`): abre a los 500 ms de dejar el mouse encima o de llegar con el teclado (solo con foco visible, no con un clic), cierra 300 ms después de salir y deja pasar el mouse a la tarjeta; Escape la cierra y devuelve el foco a la nota. **Nota:** la tarjeta muestra la cita y su documento, con el botón «Abrir el documento»; un toque, un clic o Enter la muestran y la dejan abierta hasta otro toque, Escape o un toque afuera, igual que antes de las vistas previas (la nota no lleva a ningún lado: lo que abre es el botón). Enter además lleva el teclado al botón de la tarjeta, y Tab al salir de ella vuelve a la nota. **Enlace:** la tarjeta muestra el tipo («Persona», el término del vocabulario para un caso, «Documento»), el nombre y un resumen corto con el botón «Abrir»; el clic y Enter abren el enlace como siempre; en pantalla táctil el primer toque muestra la tarjeta y el segundo abre. El resumen sale del artículo que la app ya escribió en la base (el primer párrafo de texto después del título: el tema de un documento, o lo que dicen los documentos de alguien), sin modelo; se pide recién cuando la tarjeta abre, se guarda en memoria y se olvida cuando la base cambia. Mientras llega, líneas grises en una tarjeta del mismo tamaño. La animación es solo un fundido, y no se hace si el sistema pide reducir el movimiento. Vale en los artículos de persona u organización, documento y caso, en Inicio y en las respuestas del chat; las notas de pie propias del markdown no tienen tarjeta. No hay imagen: los artículos no tienen una.
- **Quién decidió:** agente sin consulta.

### D-455 · Las vistas previas flotantes se apagan en Ajustes → Interfaz; vienen encendidas y el cambio se aplica al instante, sin el botón Guardar

- **Fecha:** 22 oct 2026 · **Tema:** Artículos · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la especificación pide una preferencia para desactivar las vistas previas; no dice dónde va, cómo viene ni qué queda cuando se apaga.
- **Qué se decidió y por qué:** una casilla «Vistas previas flotantes» al final de Ajustes → Interfaz, encendida de entrada, guardada en `app-state.json` (`floatingPreviews`) y leída al abrir la app. Se aplica al marcarla, sin el botón Guardar de la sección, porque no depende de nada más y se prueba enseguida en un artículo; el texto lo dice. Apagada, nada se abre solo con el mouse ni con el foco: una nota sigue mostrando su cita al tocarla o con Enter, y un enlace abre su ficha directamente, también en pantalla táctil.
- **Quién decidió:** agente sin consulta.

### D-456 · Una persona nombrada por el papel que tiene en el documento («el causante», «mis mandantes») cuenta como nombrada; «causante» toma el asunto del documento

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** después de releer la sucesión con la pregunta nueva (D-433), muchas relaciones de herencia y parentesco se descartaban porque su cita nombra a una de las dos personas por su papel en el documento («el causante», «la causante», «mis mandantes», «mis representados», «en representación de su padre»), no por su nombre, y el control pedía el nombre. Además, 8 «causante» se descartaban por objeto desconocido: el modelo dio como objeto a la persona misma o a nadie, o el asunto había quedado afuera (su cita era el número del expediente); y una vez puso el asunto como sujeto y a la persona como objeto.
- **Qué se decidió y por qué:** palabras de papel en el catálogo (`reading.roleReferences`, por idioma: «causante», «de cujus», «causantes», «mi mandante», «mis mandantes», «mis representados», «mis dos representados», «representación de su padre», «representación de su madre», «los herederos»…), cada una con la relación del mismo documento cuyo lado nombra: el sujeto de «causante», el objeto de «representa a» de quien firma, el sujeto de «heredero por representación de» o de «heredero de». Una cita con esas palabras nombra a quien tiene ese papel en ese documento; un papel de una sola persona («el causante») nombra solo cuando el documento tiene una sola. Si el documento no dice a quién representa quien firma, «mis mandantes» son las partes del escrito (`mentionRoles`), sin contar a quien firma. Vale para el control de la segunda persona (D-250), el de la primera (D-433) y la duda por atribución (D-434). La lectura se controla en dos pasadas: primero con los nombres; después, con los papeles que eso dejó, lo que se había descartado por no nombrar a un lado. Una relación con el asunto (`objectFromDocument`, «causante») sin un asunto dado toma el asunto del documento cuyo nombre lleva el de la persona (la carátula «APELLIDO, NOMBRE s/ sucesión»), si es uno solo; y una relación dada con los dos lados al revés según sus tipos se da vuelta. Cada arreglo queda contado como corrección. La base no sabe el género: con dos causantes en un documento, «la causante» no nombra a ninguna.
- **Quién decidió:** agente sin consulta.

### D-457 · El hijo de una filiación es quien el texto nombra justo antes de «hijo de»

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** en las partidas y actas («inscribo el nacimiento de X … hijo de A … y de B») la cita del modelo nombra al padre o a la madre y no al hijo, que está escrito justo antes; muchas veces el modelo dio además al padre en los dos lados de «padre o madre de». De 127 «padre o madre de» que devolvió el modelo para la sucesión quedaban 9.
- **Qué se decidió y por qué:** palabras de filiación en el catálogo (`reading.filiation`: «hijo de», «hija de», «hijos de», «hijo del»…; «y de», «e de», que siguen la lista de padres; a menos de 30 palabras). El hijo es la persona que la cita nombra última antes de esas palabras (o por su papel: «el causante de autos era hijo de»); si la cita empieza con ellas o con «y de», o no nombra a nadie antes («DNI…, hijo de»), la persona que el texto nombra última antes de ellas, sin contar a quien sigue a palabras de otro vínculo (`others`: «casado con», «cónyuge», «testigo»: el cónyuge que un acta de defunción escribe antes de los padres). Si la cita nombra al padre antes de «hijo de», es hijo de otro y no vale. El hijo que dio el modelo se acepta si es ese; si dio al padre en los dos lados, el hijo es ese cuando el texto dice uno solo, contado como corrección. La misma cita en dos partidas (dos hermanos) vale para los dos. La medición ya contaba «padre o madre de» contra padre, madre e hijo de la hoja.
- **Quién decidió:** agente sin consulta.

### D-458 · Fichas partidas: la medición cuenta la mejor coincidencia; una sigla con puntos es sigla; lo que el modelo dijo de un grupo releído sigue valiendo

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** la medición contaba 11 entidades de la hoja de la sucesión partidas en varias fichas. Revisadas una por una: dos no estaban partidas (un nombre de la hoja coincidía también con otra persona del mismo documento: el nombre del padre está dentro del nombre del hijo), y en otras dos la medición sumaba organismos distintos que comparten palabras (un registro genérico y el del automotor). Un colegio profesional quedó en tres fichas porque su sigla escrita con puntos se leía como letras sueltas y nunca se comparaba con su nombre, y porque lo que el modelo ya había dicho de él estaba atado a grupos de documentos releídos que no lo nombran igual. El resto son veredictos «dudosa» o «distinta» del modelo guardados (un organismo con nombre corto y largo, uno nombrado solo «Archivo», un registro de la capital, un registro y el colegio que lo lleva, una persona con dos fechas de nacimiento), organismos que nunca se compararon (una abreviatura, un cargo escrito como nombre) y una matrícula leída distinta en dos escaneos.
- **Qué se decidió y por qué:** (1) la medición dice además las fichas partidas por la mejor coincidencia de cada mención de la hoja: las escritas igual; si no, aquellas cuyas palabras tiene más esa entidad que cualquier otra de la hoja. (2) Una sigla escrita letra por letra («A.B.C.») es una sigla: se compara con las iniciales del nombre, sin perder letras como «e» o «y». (3) Lo que el modelo dijo de un grupo que ya no existe (su documento se releyó) vale para el grupo que lo nombra ahora: el del mismo documento cuyo nombre está dentro del viejo, o los del mismo nombre en otros documentos que el nombre ya hizo una sola entidad; y dos grupos que el modelo dijo que son el mismo son candidatos aunque cambien sus nombres, sin volver a preguntar (R-20). No se borró ni se cambió ningún veredicto. Con la copia, el colegio queda en una ficha y un registro civil escrito de dos maneras también; quedan 8 partidas por la medición nueva (10 por la de antes), todas con su causa (P-272, P-255).
- **Quién decidió:** agente sin consulta.

### D-459 · La pregunta de lectura (`conocimiento-7`) pide cada filiación por separado, de cada persona, con una cita que nombra a los dos y no saltea palabras

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** con las respuestas guardadas de la sucesión (D-456, D-457), de las 104 relaciones de padre, madre e hijo de la hoja se encuentran 35. De las 69 que faltan, el modelo no devolvió 38, devolvió 17 mal armadas (el padre o el hijo en los dos lados, o al revés) y 14 con una cita que no alcanza (con puntos suspensivos, o «de dicha unión» sin los nombres de los padres). Abuelos, tíos, sobrinos y nietos casi no los devolvió. Eso depende de lo que pide la pregunta, no del control del programa.
- **Qué se decidió y por qué:** la pregunta pasa a `conocimiento-7`. En sus reglas generales: una cita larga se acorta al tramo continuo que hace falta (en una relación, el que nombra a los dos), nunca salteando palabras con «...»; el sujeto y el objeto son dos menciones distintas, cada una en el lado que dice el predicado; una frase que une a varias personas da una afirmación por cada par, todas con la misma cita; un parentesco va una vez por par, sin repetir su inverso, y van los de cada persona del documento, no solo los de su titular. Lo propio de la abogacía va en el catálogo, en las descripciones de los predicados: «padre o madre de» pide una afirmación por cada padre y por cada madre de cada persona que un acta, una partida, una declaratoria o un escrito nombra con sus padres, con un ejemplo inventado («Ana Ríos, hija de Luis Ríos y de Eva Paz» son dos afirmaciones con esa misma cita), la cita desde los nombres de los padres cuando el texto dice «de cuya unión nacieron», y del nombre del inscripto al del padre o la madre en un acta con campos; «hijo de» pide registrarlo como «padre o madre de»; hermano, abuelo, nieto, tío y sobrino aceptan a uno de los dos nombrado por su papel en el documento («hermana del causante», D-456) y piden una afirmación por cada uno. El cambio no relee nada solo (D-432): con la copia de la base, 0 documentos que la lectura automática pediría y las 134 lecturas quedan como de una versión anterior. Releer la carpeta de la sucesión, si una persona lo pide, son 78 preguntas para 68 documentos: la pide el agente principal al cerrar la noche, con la autorización del dueño, y se mide después (P-271).
- **Quién decidió:** agente sin consulta (la relectura de la sucesión la autorizó el dueño del producto).

### D-460 · Una filiación dada al revés se da vuelta; un padre sin hijo toma el de la filiación; una cita con puntos suspensivos se descarta con ese motivo

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** aun con la pregunta nueva, el modelo puede armar mal una filiación: el hijo dado como padre («Z, hijo de X» dado como Z padre de X), que el control aceptaba porque la cita nombra a los dos; o el padre sin hijo. Y una afirmación cuya cita tiene puntos suspensivos se descartaba como «cita no encontrada», el mismo motivo que deja un escaneo mal reconocido y que cuenta para la vista de atención.
- **Qué se decidió y por qué:** (1) si la filiación de la cita (D-457) dice que quien se dio como padre es hijo del otro, los dos lados se dan vuelta, contado como corrección («filiacion-dada-vuelta»), en «padre o madre de» y en «hijo de»; (2) un padre sin hijo se trata como el padre en los dos lados: el hijo es el que la filiación del texto dice, si dice uno solo; (3) el hijo en los dos lados con dos padres, o el padre en los dos lados sin palabras de filiación («Padre: X»), se siguen descartando: no se sabe de quién; (4) una afirmación cuya cita no está en el texto y tiene «...» o «…» se descarta con el motivo «cita-con-puntos-suspensivos», que no cuenta como escaneo mal reconocido. Control de lectura `lectura-14`: todo se vuelve a controlar con las respuestas guardadas, sin preguntar. Con la copia: 10 descartes pasan a ese motivo, se recupera una filiación (un padre sin hijo) y la medición de la hoja no cambia (103 relaciones firmes y 2 dudosas; lo que no existe sigue en 0 firmes en su documento); con las respuestas de antes no hay ninguna dada vuelta.
- **Quién decidió:** agente sin consulta.

### D-461 · La ventana del portal se crea una sola vez y se reutiliza: se muestra al entrar a «Portales» y se oculta, sin cerrarla, al salir o al recargar la app; queda siempre como lo último que pidió la página

- **Fecha:** 22 oct 2026 · **Tema:** Portales · **Estado:** vigente (aprobada en bloque por el dueño el 22 oct, D-464)
- **El problema:** al abrir «Portales» aparecía «No se pudo abrir el portal: a webview with label `portal-pjn` already exists» y la ventana del portal quedaba vacía. Esa ventana vive dentro de la ventana de la app y sobrevive a la página. Cuando React monta la página dos veces (en desarrollo lo hace siempre), dos pedidos de mostrarla llegaban a la vez, los dos intentaban crearla y uno fallaba; y el pedido de ocultarla del primer montaje podía llegar después del segundo pedido de mostrarla y dejarla oculta. Si la app se recargaba con la página abierta, la ventana del portal quedaba encima de la pantalla nueva. La especificación dejaba elegir entre ocultarla o cerrarla al salir.
- **Qué se decidió y por qué:** ocultarla, no cerrarla: cerrada se perdería la página en la que está el portal y un recorrido en curso, y volver a abrirla es más lento; la sesión del portal queda en su perfil igual. La app crea la ventana una sola vez y después solo la mueve, la muestra o la oculta, de a un pedido por vez; si al crearla resulta que ya existe, usa esa. La página no manda órdenes sueltas: dice dónde quiere la ventana o que no la quiere, y los pedidos se aplican en orden, terminando siempre en el último y sin repetir lo que ya está; uno que falló se vuelve a intentar en el siguiente (la página lo pide cada 0,7 s mientras se ve). Cuando la página de la app se recarga, las ventanas de los portales se ocultan hasta que «Portales» las pida de nuevo.
- **Quién decidió:** agente sin consulta.

### D-462 · Limpieza de pendientes: se cierra lo que ya no corresponde y cada pendiente queda en una caja, con prioridad en lo de Claude y una propuesta en cada decisión; la sesión arranca con un resumen

- **Fecha:** 22 oct 2026 · **Tema:** Forma de trabajo · **Estado:** vigente
- **El problema:** había 178 pendientes abiertos y cada sesión empezaba listándolos todos, como pedía `CLAUDE.md`. El dueño no puede contestar 37 decisiones y mirar 44 cosas sueltas: tardaría meses. 120 no se tocaban desde antes del 19 de octubre, antes del retiro del wiki y del rediseño de asuntos, y muchos repetían a otro.
- **Qué se decidió y por qué:** el dueño pidió cerrar lo que ya no corresponde, separar lo que es solo de Claude de lo que lo necesita, ordenar el trabajo de Claude y traerle una decisión propuesta para cada cosa suya. Un sub-agente leyó los 178 contra el código y los registros, sin usar el modelo. Se borraron los 21 ya hechos y 58 más: 6 hechos sin marcar, 23 sin objeto y 29 que repetían a otro (el que queda absorbe lo que todavía pedían, y las dependencias apuntan a él). Quedan 121, cada uno en una caja: trabajo de Claude (`responsable: claude`, con `prioridad` de 1 a 3 y un `tema` para agrupar tandas), decisiones del dueño (`responsable: usuario`, con `propuesta`: el problema, la recomendación y la alternativa), cosas para que el dueño vea en la app (`a-verificar`, agrupadas por pantalla para recorrerlas de una vez) y lo que queda en espera a propósito. `en-espera` pasa a valer también para lo que espera algo externo, no solo el producto mínimo. Al empezar una sesión ya no se listan todos: se muestra cuántos hay en cada caja, las decisiones que esperan al dueño y lo próximo de Claude. Las reglas nuevas están en el propio `pendientes.json` y en `CLAUDE.md`.
- **Quién decidió:** dueño del producto (la limpieza y sus pasos); agente (las cajas, los campos `prioridad`, `tema` y `propuesta`, y el resumen al empezar).

### D-463 · El producto mínimo se cierra con la revisión de la experiencia de uso con la marca, la conexión con Micelya Core, un documento generado desde una plantilla, acciones pedidas por el chat y un instalador de prueba para el estudio de referencia

- **Fecha:** 22 oct 2026 · **Tema:** Producto · **Estado:** vigente
- **El problema:** El pendiente del producto mínimo (P-049) decía «la etapa 1 de la base más el puente con el wiki», y el wiki se retiró el 21 oct (D-395 a D-400). Sin un criterio vigente no hay forma de saber cuándo está terminado ni qué priorizar.
- **Qué se decidió y por qué:** El producto mínimo termina cuando están, funcionando y vistos por el dueño (D-154): (1) la revisión de la experiencia de uso, con el marco visual de la marca de Micelya (P-057); (2) la conexión con Micelya Core, que es lo que va a usar el estudio de referencia (D-468); (3) generar un documento a partir de una plantilla (P-285), con una sección para ver las plantillas (P-287); (4) ejecutar acciones desde el chat de la app, por ejemplo unir documentos en un caso (P-286); (5) un instalador de Windows para que algunos abogados del estudio de referencia la prueben (P-006, cuando el dueño lo pida, D-467). Al entregarlo se valida el vocabulario con ese estudio (D-470) y se revisan con ellos los términos del portal judicial, probando también el inicio de sesión del abogado (D-478). P-049 se reemplaza por P-284. El dueño escribió «la conexión a Micelya Desktop»; se entendió Micelya Core, a confirmar.
- **Quién decidió:** dueño del producto.

### D-464 · Las decisiones que el agente tomó sin consultar quedan aprobadas en bloque; las del wiki retirado, reemplazadas por su retiro

- **Fecha:** 22 oct 2026 · **Tema:** Forma de trabajo · **Estado:** vigente
- **El problema:** Unas 245 decisiones del registro estaban «en revisión» porque el agente las tomó sin consultar, mientras el dueño no estaba (P-066). Revisarlas una por una no es posible y muchas eran del wiki, que ya no existe.
- **Qué se decidió y por qué:** Se aprueban en bloque y pasan a vigentes. Las de tema «Wiki» quedan reemplazadas por su retiro (D-396). Si alguna otra resulta sin objeto, se marca cuando se toque esa parte. Tres se aprueban con un agregado: no releer todo al cambiar la pregunta (D-432) se completa con un botón para releer todo (D-465); los campos de las plantillas con nombres por su clase (D-439) se completan con una sección para ver las plantillas y, después, generar documentos (D-466); la capa que bloquea el portal (D-447) se ajusta con la hipótesis guardada en P-281: que el bloqueo no actúe dentro del recuadro del desafío e, idealmente, enfocar solo esa parte. P-066 se cierra.
- **Quién decidió:** dueño del producto.

### D-465 · Lo leído con otra versión de la pregunta o con otro modelo no se relee solo: Atención lo cuenta con un botón para releerlo, y siempre hay una opción para releer todo

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente
- **El problema:** Desde D-432, cambiar la pregunta o el catálogo no relee nada solo, y al cambiar de modelo lo ya leído tampoco se relee (P-090). La persona no se entera de cuántos documentos quedaron leídos con otra versión, y no tiene forma de pedir que se relea todo.
- **Qué se decidió y por qué:** Atención muestra cuántos documentos quedaron leídos con otra versión de la pregunta o con otro modelo, con el botón «Volver a leer con el modelo» y su costo. Además, siempre hay una opción para releer todo el archivo, con aviso de costo. Nada se relee solo.
- **Quién decidió:** dueño del producto (sobre la propuesta del agente).

### D-466 · Una sección «Plantillas» para ver todas las plantillas con sus campos y, como parte del producto mínimo, generar un documento a partir de una

- **Fecha:** 22 oct 2026 · **Tema:** Plantillas · **Estado:** vigente
- **El problema:** Las plantillas (D-438 a D-440) solo se ven desde un documento de su familia, y sus campos se llaman por su clase («Fecha 2», «Nombre 3»), que a un abogado no le dice nada.
- **Qué se decidió y por qué:** Se aprueba D-439 y se agrega una sección propia para ver todas las plantillas de la base, con sus campos y sus documentos (P-287), y mejores nombres para los campos (P-257). Generar un documento a partir de una plantilla es parte del producto mínimo (P-285, D-463).
- **Quién decidió:** dueño del producto.

### D-467 · El instalador de prueba se genera cuando el dueño lo pida, para la prueba con el estudio de referencia; no se publica una versión descargable antes

- **Fecha:** 22 oct 2026 · **Tema:** Distribución · **Estado:** vigente
- **El problema:** Habilitar la publicación de versiones del fork (P-006) quedó abierto; el repositorio es público, así que una versión publicada la puede bajar cualquiera.
- **Qué se decidió y por qué:** No se publica todavía. El dueño avisa cuándo generar un instalador para otra máquina, cuando esté lista la versión de prueba para el estudio de referencia (D-463). La firma de los instaladores (P-022) se ve en ese momento.
- **Quién decidió:** dueño del producto.

### D-468 · El estudio de referencia va a usar Micelya Core como proveedor de IA; qué usan los demás clientes se analiza después

- **Fecha:** 22 oct 2026 · **Tema:** Servicios de IA · **Estado:** vigente
- **El problema:** Para los clientes había que decidir con qué servicios de IA trabaja la app: su propia suscripción de ChatGPT o Claude (cuyos términos lo ponen en duda, P-029), claves de API o Micelya Core.
- **Qué se decidió y por qué:** En el primer caso, el estudio de referencia, se usa Micelya Core. La conexión con Core pasa a ser parte del producto mínimo (D-463). Qué hacen los demás clientes se decide más adelante; la suscripción propia queda para las pruebas del dueño.
- **Quién decidió:** dueño del producto.

### D-469 · El marco visual con la marca de Micelya es parte del producto mínimo: primero una maqueta de tres pantallas para aprobar y después se aplica a toda la app

- **Fecha:** 22 oct 2026 · **Tema:** Interfaz · **Estado:** vigente
- **El problema:** El marco visual y los elementos de marca (P-057) estaban sin fecha, y rediseñar pantallas que van a cambiar es trabajo perdido.
- **Qué se decidió y por qué:** Es parte del producto mínimo, junto con la revisión de la experiencia de uso (D-463). Primero una maqueta navegable de tres pantallas (Inicio, una persona y un caso) con la paleta, las tipografías y la barra superior del manual de marca; el dueño la aprueba o corrige, y recién entonces se aplica a toda la app.
- **Quién decidió:** dueño del producto (sobre la propuesta del agente).

### D-470 · El vocabulario queda fijo en estudios jurídicos de Argentina y se valida con el estudio de referencia al entregarle el producto mínimo

- **Fecha:** 22 oct 2026 · **Tema:** Vocabulario · **Estado:** vigente
- **El problema:** Las palabras de la app (caso, plantilla, los nombres de las relaciones) están fijas para estudios jurídicos de Argentina (P-069, P-079); elegir otro rubro o país, o cambiar palabras por estudio, no existe.
- **Qué se decidió y por qué:** No se construye todavía un selector. La validación de las palabras se hace con el estudio de referencia cuando se le entregue el producto mínimo. Cambiar palabras por estudio va junto con el editor de catálogos (P-279).
- **Quién decidió:** dueño del producto.

### D-471 · Dos fichas que una persona separó se vuelven a proponer como la misma solo ante evidencia fuerte y nueva

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente
- **El problema:** Cuando una persona decide que dos fichas no son la misma, quedan separadas para siempre, aunque aparezca después un documento que muestre que sí lo son (P-099).
- **Qué se decidió y por qué:** Se vuelve a preguntar solo con evidencia fuerte y nueva: el mismo DNI, CUIT o matrícula en un documento que no estaba cuando se decidió. Un nombre parecido no alcanza. Se muestra en Atención con lo nuevo resaltado.
- **Quién decidió:** dueño del producto (sobre la propuesta del agente).

### D-472 · En un caso, la app propone quién es el cliente del estudio y quién la contraparte, y una persona lo confirma una vez por caso

- **Fecha:** 22 oct 2026 · **Tema:** Asuntos · **Estado:** vigente
- **El problema:** Un caso muestra sus partes, pero no de qué lado está cada una ni quién es el cliente del estudio (P-106).
- **Qué se decidió y por qué:** La app lo propone con lo que ya lee (quién remite y quién recibe las cartas, a quién representa el abogado del estudio) y una persona lo confirma una vez por caso. Se ve en la cabecera del caso como «Cliente» y «Contraparte».
- **Quién decidió:** dueño del producto (sobre la propuesta del agente).

### D-473 · La ficha de un grupo de sociedades se crea siempre que haya una señal firme, aunque no haya nada que atribuirle, y también se puede pedir por el chat

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente
- **El problema:** La ficha del grupo de dos sociedades que parecen del mismo grupo se creaba solo si había algo que atribuirle (D-260, P-128), así que la relación no se veía.
- **Qué se decidió y por qué:** Se crea siempre que haya una señal firme (mismo nombre, otra forma societaria y otra señal que las vincule). Las marcas sin forma societaria quedan afuera. Una persona también puede pedirlo desde el chat (P-286).
- **Quién decidió:** dueño del producto (sobre la propuesta del agente).

### D-474 · Las cláusulas repetidas entre contratos se muestran en chico: dentro de los documentos de una misma plantilla

- **Fecha:** 22 oct 2026 · **Tema:** Parecidos · **Estado:** vigente
- **El problema:** La app puede detectar pasajes que se repiten entre documentos distintos (P-211), pero no estaba decidido si se construye ni con qué alcance.
- **Qué se decidió y por qué:** Sí, en chico: dentro de las familias de la misma plantilla, mostrar «esta cláusula está también en N documentos» y compararlas cláusula por cláusula, sin páginas nuevas. Antes de ir más lejos, medirlo en una carpeta con muchos contratos.
- **Quién decidió:** dueño del producto (sobre la propuesta del agente).

### D-475 · Un caso se cierra o se archiva a mano y se puede reabrir; cerrado, sale de las listas y de Atención pero sigue en la búsqueda

- **Fecha:** 22 oct 2026 · **Tema:** Asuntos · **Estado:** vigente
- **El problema:** Todos los casos quedan abiertos para siempre (P-213).
- **Qué se decidió y por qué:** Una persona lo cierra o archiva desde el caso y puede reabrirlo. Un caso cerrado sale por defecto de las listas y de Atención, sigue apareciendo en la búsqueda con la marca «cerrado» y no recibe propuestas nuevas de documentos.
- **Quién decidió:** dueño del producto (sobre la propuesta del agente).

### D-476 · Una revisión con el modelo busca contradicciones entre los documentos de una ficha o un caso, a pedido, y las lleva a Atención

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente
- **El problema:** La revisión que había antes (Lint) salió con el wiki (D-399); quedó pendiente una que agregue valor en la estructura nueva (P-226).
- **Qué se decidió y por qué:** Se hace a pedido desde la ficha o el caso, no sola: el modelo lista las contradicciones con sus citas y aparecen en Atención para resolver. Se empieza por los casos confirmados.
- **Quién decidió:** dueño del producto (sobre la propuesta del agente).

### D-477 · Entran al catálogo los vínculos de expedientes que se usan seguido, medidos antes con el expediente de prueba nuevo

- **Fecha:** 22 oct 2026 · **Tema:** Conocimiento · **Estado:** vigente
- **El problema:** El catálogo de vínculos no tiene algunos que usan los expedientes y contratos (P-256, P-093), y cada vínculo nuevo alarga la pregunta al modelo.
- **Qué se decidió y por qué:** Se suman los que un abogado busca y aparecen seguido: funcionario de, transfiere a o cesión, radicado en, patrocinante de y cliente de. Los que casi no aparecen quedan como datos del documento. Antes de adoptarlos se miden con una hoja de respuestas de la carpeta de prueba del PJN y, si hace falta, de otro expediente bajado del portal.
- **Quién decidió:** dueño del producto (sobre la propuesta del agente).

### D-478 · Los términos de uso del portal judicial se revisan con el estudio de referencia al entregar el producto mínimo, y con ellos se prueba también el inicio de sesión del abogado

- **Fecha:** 22 oct 2026 · **Tema:** Portales · **Estado:** vigente
- **El problema:** Antes de usar «Portales» con clientes hace falta que un abogado confirme que el reglamento del portal permite que la app navegue por la persona (P-265), y hasta ahora solo se probó el desafío, no el inicio de sesión.
- **Qué se decidió y por qué:** Se revisa con el estudio de referencia cuando se le entregue el producto mínimo. Con ellos se prueba también la versión en la que el abogado inicia sesión, no solo la del desafío. Las pruebas del dueño siguen mientras tanto.
- **Quién decidió:** dueño del producto.

### D-479 · La protección de main exige una sola comprobación con nombre fijo que siempre informa, y esa comprobación corre las pruebas en Linux

- **Fecha:** 22 oct 2026 · **Tema:** Proceso de trabajo · **Estado:** vigente
- **El problema:** La regla proteger-main exigía «check (windows-latest)», un nombre que sale de la matriz de máquinas Windows, que solo compilaba y no corría pruebas (P-046). El 22 oct quedó «en cola» sin máquina y el pedido de integración no se podía integrar (P-282).
- **Qué se decidió y por qué:** Una comprobación «verificacion» que espera a las demás y siempre informa; las pruebas rápidas (tipos, pruebas de la interfaz y de Rust) en Ubuntu, con tope de tiempo; la compilación de Windows aparte; un commit nuevo cancela la corrida anterior del mismo pedido. Se prueba en su propio pedido de integración y, con la aprobación del dueño, la regla pasa a exigir «verificacion».
- **Quién decidió:** dueño del producto (sobre la propuesta del agente).

### D-480 · El guion que bloquea la ventana del portal actúa solo en la página principal y, al bloquear, saca el foco

- **Fecha:** 22 oct 2026 · **Tema:** Portales · **Estado:** vigente
- **El problema:** En la primera prueba real (22 oct) el desafío del PJN quedó bloqueado: «Ver desafío» no respondía y dentro de su recuadro aparecía «Micelya está usando esta ventana». En Windows, Tauri instala el guion de inicio también dentro de los recuadros de una página (su documentación lo dice: «Windows: scripts are always added to subframes»), el guion arranca bloqueado y la app desbloquea solo la página principal. Con una página inventada con un recuadro de otro origen se reprodujo igual.
- **Qué se decidió y por qué:** El guion termina de entrada si no está en la página principal. Mientras está bloqueada, la capa de la página principal ya tapa toda la ventana, recuadros incluidos, así que no se pierde protección. Al bloquear, además, se saca el foco del elemento activo: sin eso, las teclas llegaban a un recuadro que tenía el foco. Comprobado con la página inventada: bloqueado, tocar el recuadro no hace nada; desbloqueado, responde. Falta la prueba real con el dueño; después viene el modo foco (P-281).
- **Quién decidió:** agente, sobre la hipótesis que trajo el dueño (P-281).
