# Inventario de la base de datos por significado (27 oct 2026)

**Estado:** borrador para que el dueño lo revise. Reemplaza la versión del 26 oct, corregida con la revisión del dueño y otro agente (ver «Qué se corrigió»). Acompaña a la [propuesta del modelo](modelo-de-conocimiento-propuesta.md) y a la [base de destino](base-de-datos-destino.md).

**Para qué sirve ahora.** Los datos actuales se pueden descartar (D-546): este inventario **no es un mapa de migración**. Sirve para tres cosas: recuperar qué necesidad descubrimos con cada tabla, para que la base nueva no la pierda; saber qué aprendizaje hay que extraer antes de descartar la base real; y saber qué código toca cada parte.

**Cómo se hizo.** Un guion buscó, en el código de `src-tauri/src` (sin pruebas, sin `schema.rs` ni `upgrade.rs`), qué archivos escriben cada tabla (`INSERT`, `UPDATE`, `DELETE`) y cuáles la leen (`FROM`, `JOIN`). Se leyeron a mano solo los casos dudosos. El significado sale de los comentarios del propio esquema y de esas lecturas. Lo que dice «inferencia» no se verificó leyendo el código.

## Qué se corrigió respecto del 26 oct

- **Un `content` no es un archivo.** Al guardar el texto, si otro contenido ya tiene el mismo texto, los dos pasan a ser uno (`text.rs`, `store`): un Word y su PDF quedan bajo el mismo `content`. Es una mezcla de recurso binario y documento, que la base nueva separa.
- **`knowledge_matter.state` y `status` son dos cosas:** `state` dice si el registro sigue activo; `status` (D-475) si el caso está abierto, cerrado o archivado. Las dos hacen falta.
- **La respuesta del modelo y el veredicto no se duplican:** `model_answer` guarda lo recibido; el veredicto es la conclusión que usa el programa. Se mantiene la distinción.
- **Faltaban tres tablas vivas:** `content_fts` (búsqueda en el texto), `content_search` (búsqueda en números normalizados y resúmenes) y `similar_key_data` (los datos clave con que se detecta que hay que volver a comparar).
- **Contar referencias no prueba que una tabla esté muerta.** Ahora se separan escrituras, lecturas, importaciones únicas y restos.

## Naturaleza de lo que guarda cada tabla

| Naturaleza | Qué quiere decir | Con la base descartable |
|---|---|---|
| **Observación** | Lo que la base vio en los orígenes: archivos, huellas, cambios | Se vuelve a observar al recorrer |
| **Resultado** | Lo que el programa calcula y puede recalcular | Se recalcula |
| **Respuesta del modelo** | Lo que dijo la IA, guardado para no preguntar dos veces (R-20) | Se puede recalcular, pero cuesta cupo; con las preguntas nuevas, la mayoría se vuelve a hacer |
| **Decisión humana** | Lo que decidió una persona | No se migra; **se extrae el criterio o el ejemplo que enseña** |
| **Creado por el usuario** | Lo que una persona cargó: un caso, una nota, una investigación | En pruebas se descarta; en el uso real es dato fuente |
| **Cola o configuración** | Trabajo pendiente, ajustes, registro de actividad | Se reinicia |

## Las tablas

Las columnas «escriben» y «leen» dan los archivos principales (sin `docbase/`).

### Orígenes, archivos y texto

| Tabla | Naturaleza | Escriben / leen | Necesidad que cubre | En el modelo nuevo |
|---|---|---|---|---|
| `space` | Configuración | la crea el esquema / `mod` | De qué espacio es la base | Queda (privado; la clase «entrenamiento» se suma) |
| `origin` | Configuración | `origins`, `scan`, `identity` / 17 archivos | Cada fuente: carpeta, proveedor, carpeta de trabajo | Queda como **fuente**; la procedencia se modela aparte |
| `location` | Observación | `scan`, `identity`, `work_origin` / 34 archivos | Cada archivo en un origen, con su huella | Pasa a ser la **ubicación de clase archivo** de un recurso. `label_overrides` va a las etiquetas |
| `file` | Observación | `identity`, `text` / 30 archivos | Cada huella de archivo y su contenido | Se vuelve el **recurso binario** (huella, tamaño) |
| `content` | Resultado (mezcla) | `identity`, `text`, `recognize` / 20 archivos | Hoy junta archivos con el mismo texto: es a la vez recurso y documento | **Se parte** en recurso, representación textual y documento (ver la base de destino) |
| `derivative` | Resultado | `text`, `page_shapes` / 18 archivos | El texto obtenido y otros derivados | Queda, **por recurso o por versión** según el derivado |
| `content_fts`, `content_search` | Resultado | `text`, `search` / `search`, `query` | Dos búsquedas distintas: texto y números normalizados más resúmenes | Quedan, **por documento** |
| `recognition_request` | Cola | `recognize` | Escaneos que esperan reconocimiento | Queda, por recurso |
| `change` | Observación | `scan`, `identity` / `changes`, `query` | «Qué cambió desde…» | Queda |
| `exclusion` | Decisión humana | `tree` / `mod`, `work_origin` | Qué entra al procesamiento | Queda: decidir qué entra es aparte de interpretar |

### Lo heredado del wiki

| Tabla | Naturaleza | Escriben / leen | En el modelo nuevo |
|---|---|---|---|
| `project`, `project_origin` | Configuración | `origins`, `work_origin`, `chat_files`, `pages/check_retire` | **Sale.** Hoy asocian los orígenes al «proyecto general» y sirven a la copia única de archivos de chat del wiki; con una base nueva no hay nada que copiar. `origins.rs` y `work_origin.rs` dejan de usarlas |
| `wiki_page`, `wiki_page_source` | Resto | solo las lee `pages/import` (importación única del wiki) | **Sale**, con `pages/import` y `chat_files` |
| `wiki_link`, `knowledge_source_block` | Resto | ningún archivo | **Sale** |

### Etiquetas

| Tabla | Naturaleza | En el modelo nuevo |
|---|---|---|
| `label_group`, `label`, `location_label` | Creado por el usuario / resultado | Quedan para clasificar |
| `label_rule` | Decisión humana | Queda; su maquinaria se reutiliza para los criterios por carpeta (que crean relaciones) |
| `label_manual` | Decisión humana | Queda |

### Lectura y conocimiento

| Tabla | Naturaleza | Escriben / leen | Necesidad que cubre | En el modelo nuevo |
|---|---|---|---|---|
| `knowledge_extraction` | Resultado + respuesta | `extract` / 12 archivos | Una lectura por contenido, con su base (versión de instrucciones, vocabulario, texto) | Una lectura **por versión de documento**, con su base |
| `knowledge_mention` | Resultado | `extract`, `resolve` / 22 archivos | Cómo un documento nombra algo, con rol y cita | Queda, con **ubicación extensible** en vez de `page`, y apuntando a la versión |
| `knowledge_identifier` | Resultado | `extract` / 11 archivos | DNI, CUIT… de una mención | Queda, idem |
| `knowledge_evidence` | Resultado | `extract`, `claims` / 17 archivos | Lo que un documento dice de una mención | Queda como **evidencia**, atada a la versión y con postura (apoya o cuestiona) |
| `knowledge_entity`, `knowledge_alias` | Resultado | `resolve` / 15 y 2 archivos | Personas y organizaciones y sus nombres | Quedan, con id del registro común; sale el tipo `asunto` |
| `knowledge_claim` | Resultado | `claims` / 13 archivos | Lo que se sostiene, calculado de la evidencia. `context_entity` (el cargo **dentro de** una empresa) y `qualifier` (un domicilio constituido no es el real) evitan contradicciones falsas (D-210, D-516) | Pasa a ser la **relación**, con sus extremos en el registro común. **El contexto y la calificación se conservan** (ver la propuesta, sección 5) |
| `knowledge_group` | Resultado | `resolve` / `graph`, `pages` | Grupos de organizaciones vinculadas | Se vuelve relación entre entidades |
| `knowledge_doubt` | Resultado | `resolve` / `claims`, `entities` | Pares que la resolución no pudo distinguir | Queda como **duda**, entre piezas cualesquiera |
| `knowledge_verdict` | Respuesta del modelo | `resolve` | El veredicto del modelo sobre dos grupos de menciones | Queda como **veredicto**, separado de la respuesta cruda |
| `knowledge_decision` | Decisión humana | `decide`, `anchors` / `matters`, `screens` | Unir, separar, marcar como errónea, confirmar, nombrar. **Se ancla a claves estables de las menciones, no a ids de filas**, así sobrevive a una nueva lectura (`anchors.rs`) | Queda como el lugar de las decisiones sobre identidad y relaciones, **con el mismo anclaje estable** |
| `knowledge_summary` | Resultado | `extract` / `pages`, `search` | Los puntos del resumen con su cita | Queda, por documento, citando la versión |
| `knowledge_page`, `knowledge_overview` | Resultado | `pages` / 12 archivos | Las páginas que la app escribe por entidad y el panorama | Quedan como derivados de una pieza |
| `knowledge_note`, `knowledge_query`, `knowledge_research` | Creado por el usuario | `articles`, `research` | Notas, preguntas guardadas, investigaciones | Quedan, apuntando al registro común (hoy `object_id` usa prefijos `ent-`, `mat-`, `doc-`) |
| `knowledge_gone`, `knowledge_gone_file`, `knowledge_gone_mention` | Resultado (por disparadores) | disparadores del esquema / `changes`, `identity` | Que un documento reemplazado conserve las identidades de sus nombres | Quedan, por documento |
| `knowledge_dirty`, `similar_dirty` | Cola | `resolve`, `pages` / `similar/store` | Qué hay que recalcular | Quedan; la granularidad es la del reproceso (documento y etapa) |

### Asuntos

| Tabla | Naturaleza | Escriben / leen | Necesidad que cubre | En el modelo nuevo |
|---|---|---|---|---|
| `knowledge_matter` | Resultado + creado por el usuario | `matter_edit`, `matters`, `matter_records` / 13 archivos | El asunto: puede crearse **sin documentos** (`matter_edit`); `state` (activo) y `status` (abierto, cerrado, archivado); quién lo creó | **Pieza de clase asunto.** Se conservan los dos estados; `confirmation` y `name_by_person` van a decisiones |
| `knowledge_matter_document` | Resultado | `matters`, `matter_edit` / 12 archivos | Qué documentos están en el asunto y por qué | Relación documento–asunto |
| `knowledge_matter_choice` | Decisión humana | `matter_edit` | Documento puesto o sacado a mano | Decisión sobre esa relación |
| `knowledge_matter_record` | Observación (del portal) | `matter_records` / `portals/check_records` | El expediente: portal, jurisdicción, número, año, título, de dónde vino | **Pieza de clase expediente** (propuesta) y procedencia de sus actuaciones |
| `knowledge_matter_question` | Resultado + decisión | `matter_records` | Si un asunto parecido al de un expediente es el mismo | Duda entre asuntos, con su decisión |
| `knowledge_matter_verdict` | Respuesta del modelo | `matters` | Si un documento pertenece a un asunto | Veredicto |

### Tipos, datos y parecidos

| Tabla | Naturaleza | Escriben / leen | Necesidad que cubre | En el modelo nuevo |
|---|---|---|---|---|
| `document_type`, `document_data` | Resultado + respuesta | `similar/classify`, `similar/store` / 8 archivos | Qué es cada documento y sus datos, con de dónde salió (marcas, modelo o familia) | Por documento (tipo) y por versión (datos) |
| `type_decision` | Decisión humana | `similar/classify` | Un tipo propuesto aceptado o rechazado | **Aprendizaje del catálogo**: es exactamente una aprobación de vocabulario |
| `document_marks` | Resultado | `similar/store` | Lo verificado sin modelo (firmas, marcas) | Por recurso |
| `similar_signature`, `similar_band`, `similar_common`, `similar_key_data` | Resultado | `similar/store` | El motor de parecidos: huellas de texto, vecinos, texto rutinario, datos clave | Quedan; se suman las **huellas de página** |
| `similar_pair` | Resultado + decisión | `similar/store`, `by_type`, `generate` / 13 archivos | La relación entre dos documentos, sus medidas y la decisión de una persona | La relación va a **relaciones**, las medidas a **evidencia**, la decisión a **decisiones** |
| `similar_group`, `similar_group_member` | Resultado | `similar/store` | Familias de documentos | Se vuelven **versiones de un documento** o **documentos de una plantilla** |
| `similar_template*`, `template_naming` | Resultado | `similar/template`, `similar/store` | Plantillas, sus campos y valores, con una identidad que dura | **Pieza de clase plantilla** (propuesta) |
| `template_field_choice`, `template_field_answer` | Decisión humana / respuesta | `similar/template`, `similar/naming` | Qué es cada campo de una plantilla | Decisión y veredicto sobre la plantilla |
| `generated_document` | Observación | `similar/generate` | Lo que la app generó desde una plantilla, para qué asunto | Documento con procedencia «generado por la aplicación» |
| `model_answer` | Respuesta del modelo | `questions`, `similar/classify` / 8 archivos | Cada respuesta, una vez por pregunta (R-20) | Queda |

### Fuera del conocimiento

`local_program`, `local_program_use`, `operation_log` (el servidor local y el registro de operaciones) y `docbase_setting` (ajustes) quedan.

## Qué aprendizaje extraer de la base real antes de descartarla

Lo hace el agente local, o este cuando la base esté en Drive (no se ve desde la nube). Sale como entradas del catálogo, ejemplos y hoja de respuestas, nunca como registros a migrar.

| De dónde | Qué se extrae |
|---|---|
| `knowledge_decision` (unir, separar, errónea) | Los pares que una persona separó o unió: casos difíciles de identidad, y si alguno revela un criterio general |
| `type_decision` | Los tipos aprobados con su nombre y descripción: candidatos al catálogo |
| `similar_pair` con decisión | Las relaciones entre documentos que una persona corrigió: ejemplos para versiones, copias y plantillas |
| `template_field_choice` | Qué es cada campo de las plantillas reconocidas |
| `knowledge_matter_choice` y asuntos confirmados o descartados | Criterios de pertenencia a un asunto |
| Las hojas de respuesta (fuera de la base, `llm_wiki_mi/conjunto-prueba`) | Se conservan enteras; se revisan contra el criterio nuevo de documento (D-527, D-534) |
