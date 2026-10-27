# Inventario de la base de datos frente al modelo del replanteo (26 oct 2026)

**Estado:** borrador para que el dueño lo revise. Acompaña a la [propuesta del modelo de documentos y conocimiento](modelo-de-conocimiento-propuesta.md). No describe la base de destino tabla por tabla: dice qué hay hoy, qué problema tiene frente al modelo y qué pasaría con cada tabla.

**Pedido del dueño (D-535):** el modelo de la base se fue degradando con parches. Hay que reencauzarlo, prolijo y alineado con el modelo conceptual, con nombres de tablas y campos canónicos y sin vocabulario de una vertical. No es un parche más: lo que haya que cambiar se cambia.

**Cómo se hizo:** un guion leyó las definiciones de `src-tauri/src/docbase/schema.rs` (las `CREATE TABLE` y los `ALTER TABLE … ADD COLUMN`) y los comentarios que el propio programa escribe sobre cada tabla; para las tablas sospechadas de restos se contaron las referencias en el código. No se leyó el código que usa cada tabla: lo que dice «inferencia» falta verificarlo.

## 1. Lo que muestra el inventario

Son unas 60 tablas. Las tablas `knowledge_decision_v15`, `knowledge_decision_v29` y `model_answer_by_engine` son pasos intermedios de migraciones, no tablas vivas.

**Un mismo concepto en varios lugares (verificado):**

| Concepto | Dónde está hoy |
|---|---|
| Asunto | `knowledge_matter` y, a la vez, entidades de tipo `asunto` en `knowledge_entity` (el catálogo lo define como tipo de entidad; D-183 decidió extraer los asuntos como entidades) |
| Relaciones | `knowledge_claim` (entre entidades); `similar_pair` (entre documentos: copia, versión, uno dentro de otro); `knowledge_matter_document` (documento y asunto); `knowledge_group` (grupos de organizaciones); `generated_document.matter_id` y `matter_name` |
| Decisiones del humano | `knowledge_decision`; `type_decision`; `knowledge_matter_choice`; `knowledge_matter_question.answer`; columnas de decisión en `similar_pair` (`state`, `chosen_relation`, `decided_at`…); `knowledge_matter.confirmation` y `name_by_person`; `template_field_choice`; `label_manual` |
| Lo que dijo el modelo | `model_answer` (la respuesta guardada) y, encima, `knowledge_verdict`, `knowledge_matter_verdict`, `template_field_answer`, campos de pregunta en `document_type` y `document_data` |
| Identificador común | Ya existe uno hecho a mano: `knowledge_note.object_id` usa prefijos `ent-…`, `mat-…`, `doc-…` |

**Lo que no existe (verificado):**
- **El documento como unidad.** La pieza es el contenido (`content`), identificado por la huella del archivo entero. Un documento incluido en un compuesto no tiene dónde vivir; tampoco las versiones ni la que prevalece.
- **Otra forma de ubicar que no sea la página.** `knowledge_mention`, `knowledge_identifier` y `knowledge_evidence` guardan solo `page`.
- **Las cosas** (inmuebles, vehículos) como piezas.
- **Relaciones cuyo sujeto no sea una entidad.** `knowledge_claim.subject` apunta solo a `knowledge_entity`.

**Rastros de parches (verificado):**
- `knowledge_matter` tiene `state` y `status`, más cinco columnas agregadas después.
- `knowledge_claim` tiene cinco columnas agregadas después (`certainty`, `amount`, `detail`, `context_entity`, `qualifier`).
- `wiki_link` y `knowledge_source_block` no tienen ninguna referencia en el código fuera de la definición de la base. `wiki_page` y `wiki_page_source` solo las usa la importación del wiki retirado (y `wiki_page`, además, `agent/runtime.rs`).

**Nombres:** las tablas y los campos ya están casi todos en inglés canónico (inferencia sobre la lista; no se revisaron uno por uno). El vocabulario de una vertical está sobre todo **en los datos del catálogo**: los tipos de entidad `persona`, `organizacion` y `asunto`, y relaciones solo jurídicas (`causante`, `heredero_de`) mezcladas con las generales. Excepciones en la base: `knowledge_matter_record.portal` y `jurisdiction` (propias del expediente judicial).

## 2. Qué pasa con cada tabla

Leyenda: **queda** (con cambios menores); **cambia** (de forma o de nombre); **se funde** en otra; **sale**.

### Espacio, orígenes y archivos

| Tabla | Qué guarda hoy | Qué pasa |
|---|---|---|
| `space` | El espacio de la base | Queda. Se suma la clase «entrenamiento» |
| `origin` | Cada origen (carpeta, proveedor) | Queda. Es la fuente de la procedencia |
| `location` | Cada archivo en un origen | Cambia: es una aparición con ubicación de clase «archivo». `label_overrides` pasa a decisiones |
| `file` | La huella de cada archivo | Queda |
| `content` | El contenido de un archivo entero, hoy usado como «el documento» | Cambia: sigue siendo el recurso binario; el **documento** pasa a ser una pieza propia, con versiones y apariciones (una aparición = recurso + ubicación dentro de él) |
| `derivative` | Lo obtenido de un contenido (texto, y más adelante resumen) | Cambia: lo generado es por documento, de la versión que prevalece; el texto sigue siendo por recurso |
| `change`, `exclusion`, `recognition_request` | Registro de cambios; lo excluido del procesamiento; la cola de reconocimiento de escaneos | Quedan |
| `project`, `project_origin` | Proyectos del wiki | A revisar: el wiki se retiró, pero el código aún los usa (`origins.rs`, `work_origin.rs`, `chat_files.rs`) |
| `wiki_page`, `wiki_page_source`, `wiki_link`, `knowledge_source_block` | Restos del wiki | Salen, después de verificar la importación y `agent/runtime.rs` |

### Etiquetas

| Tabla | Qué pasa |
|---|---|
| `label_group`, `label`, `location_label`, `label_manual` | Quedan para clasificar y filtrar. Ninguna pieza es una etiqueta |
| `label_rule` | Queda, y su maquinaria se reutiliza para los criterios por carpeta, que crean relaciones con origen «regla de carpeta» |

### Piezas y relaciones

| Tabla | Qué pasa |
|---|---|
| *(nueva)* registro común de piezas | Identificador opaco (UUID v7) y clase de cada pieza. Reemplaza los prefijos hechos a mano de `knowledge_note.object_id` |
| `knowledge_entity` | Cambia: solo personas y organizaciones, con su id en el registro común. Sale el tipo `asunto` |
| `knowledge_matter` | Cambia: el asunto, con su id en el registro común. Un solo campo de estado; `origin` dice quién lo creó (portal, aplicación o persona); `confirmation` y `name_by_person` pasan a decisiones |
| `knowledge_matter_record` | Cambia: el expediente. Si es una pieza propia está abierto (propuesta, sección 9); `portal` y `jurisdiction` se generalizan como organismo y procedencia |
| *(nuevas)* cosas | Tabla de la clase «cosa», con su tipo e identificadores del catálogo |
| *(nueva)* documento, versión, aparición | Ver la sección 3 de la propuesta |
| `knowledge_alias` | Cambia: nombres de cualquier pieza, no solo de entidades |
| `knowledge_claim` | Cambia: **la tabla única de relaciones.** Sujeto y objeto apuntan al registro común; desde y hasta con su precisión; origen; certeza; estado. Absorbe a `similar_pair` (la relación; las medidas quedan como evidencia), `knowledge_matter_document`, `knowledge_group` y el asunto de `generated_document` |
| `knowledge_evidence`, `knowledge_mention`, `knowledge_identifier` | Cambian: la `page` se reemplaza por una ubicación extensible según la clase de fuente |
| `knowledge_extraction` | Queda: la lectura de cada documento, con la versión del catálogo |
| `knowledge_summary` | Se funde con lo generado por documento |
| `knowledge_page`, `knowledge_overview`, `knowledge_note`, `knowledge_query`, `knowledge_research` | Quedan; apuntan al registro común |
| `knowledge_gone`, `knowledge_gone_file`, `knowledge_gone_mention` | Quedan: la memoria de lo que se fue |
| `knowledge_dirty`, `similar_dirty` | Quedan como colas de trabajo; a evaluar si se unen |

### Decisiones, dudas y respuestas del modelo

| Tabla | Qué pasa |
|---|---|
| `knowledge_decision` | Cambia: **el único lugar de las decisiones humanas.** Absorbe `type_decision`, `knowledge_matter_choice`, la respuesta de `knowledge_matter_question`, las columnas de decisión de `similar_pair` y de `knowledge_matter`, y `template_field_choice` |
| `knowledge_doubt` | Cambia: dudas entre dos piezas cualesquiera, más las contradicciones abiertas. Absorbe la pregunta de `knowledge_matter_question` |
| `model_answer` | Queda: cada respuesta del modelo, una sola vez por pregunta (R-20) |
| `knowledge_verdict`, `knowledge_matter_verdict`, `template_field_answer` | Se funden en una tabla de veredictos, con la clase de pregunta |
| `document_type`, `document_data` | Cambian: por documento, no por contenido. Los tipos propuestos alimentan el ciclo de aprendizaje |

### Parecidos y plantillas

| Tabla | Qué pasa |
|---|---|
| `similar_signature`, `similar_band`, `similar_common`, `document_marks` | Quedan: son el motor que mide. Se suman las huellas de página |
| `similar_pair` | Se funde: la relación va a la tabla única; las medidas quedan como evidencia |
| `similar_group`, `similar_group_member` | Cambian: las familias son versiones de un documento o documentos de una plantilla |
| `similar_template*`, `template_naming` | Cambian: la plantilla como pieza, si se confirma (propuesta, sección 9) |
| `generated_document` | Cambia: un documento con procedencia «generado por la aplicación»; el asunto pasa a una relación |

### Fuera del modelo de conocimiento

| Tabla | Qué pasa |
|---|---|
| `local_program`, `local_program_use`, `operation_log`, `docbase_setting` | Quedan |

## 3. Cómo se haría

1. Diseñar la base de destino tabla por tabla a partir del modelo aprobado.
2. Revisar el código que usa cada tabla que cambia (lo que acá es inferencia).
3. Migrar a una versión nueva de la base con el mecanismo existente: respaldo previo y ningún cambio sin respaldo (D-496).
4. Los ids del catálogo, canónicos; lo jurídico en la capa de la vertical.
