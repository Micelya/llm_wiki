# Base de datos de destino del replanteo (27 oct 2026)

**Estado:** aprobada por el dueño el 27 oct para construirla por capas (D-554), con las correcciones del agente local (D-560) y las precisiones del revisor (D-561, asumidas en la corrida desatendida). Primero lo de abajo (secciones 2 y 4, y la lectura colgada de la versión); después las piezas, las relaciones y el catálogo. Traduce a tablas el [modelo de documentos y conocimiento](modelo-de-conocimiento-propuesta.md), teniendo en cuenta el [inventario por significado](base-de-datos-inventario.md). No autoriza cambios al programa. Alcance: dimensión individual (D-549); la base actual se descarta y se vuelve a procesar (D-548).

**Cómo leerla.** Cada grupo dice qué responsabilidad tiene, qué tablas lo forman y qué restricciones impiden datos inconsistentes. Los nombres son canónicos, en inglés, sin vocabulario de una vertical. Los campos se dan solo cuando aclaran el significado; el detalle de tipos y columnas se fija al construir.

## 1. Mapa general

```
origin ─┬─ resource_location ── resource ──┬── resource_text, resource_page, resource_marks
        │                                  ├── resource_part (entrada de un ZIP, adjunto de un correo)
        │                                  └── provenance ──(record)
        │
piece ──┬── document ── version ── appearance ──(resource + selector)
        ├── entity
        ├── matter
        ├── thing
        ├── record    (expediente, D-559)
        └── template  (plantilla, D-559)
        │
reading ── mention ── mention_identifier
relation ── evidence          decision     doubt     verdict ── model_answer
catalog_entry (app + instance)        work_item
```

## 2. Fuentes y recursos: qué bytes hay y dónde

| Tabla | Responsabilidad | Restricciones |
|---|---|---|
| `space` | El espacio de esta base: `private` o `training` (D-545) | Una sola fila |
| `origin` | Cada fuente configurada: una carpeta, un proveedor, la carpeta de trabajo | — |
| `resource` | Una secuencia de bytes: huella SHA-256, tamaño, tipo de medio | Huella única. **Nunca cambia:** un archivo editado es otro recurso |
| `resource_location` | Dónde está hoy cada recurso en un origen (la ruta). Es la contención en carpetas | `(origin_id, rel_path)` único; `resource_id` apunta a `resource` (nulo mientras se calcula la huella) |
| `resource_part` | Un recurso que sale de otro: una entrada de un ZIP, un adjunto de un correo | Padre e hijo existen; `selector` dice cuál parte; padre ≠ hijo |
| `resource_text` | El texto de un recurso **por página** y por motor: la capa de texto (pdfium), lo leído por posición, lo impreso leído con RapidOCR en la PC y lo que devuelva un proveedor (D-558) | Por `(resource_id, page_no, engine)`; un recurso sin páginas tiene una sola. Así cada documento incluido arma su texto con sus páginas (D-560). Es lo que hoy hace `content` + `derivative` para el texto, sin juntar recursos |
| `resource_page` | Por página: huella exacta, huellas de imagen, huella del texto, forma de la página (D-522) y si tiene manuscrito o firmas (por página y, más adelante, por región; D-518) | Solo recursos con páginas; `(resource_id, page_no)` único |
| `resource_marks` | Lo verificado sin modelo: firmas digitales, marcas | Por recurso |
| `provenance` | Cómo se obtuvo un recurso: canal (`portal`, `email`, `upload`, `generated`, `scan`), cuándo, la referencia en la fuente, los datos que dio (fecha, fojas, tipo de actuación) y, si es de un portal, el expediente | Apunta a un `resource` existente; varias por recurso. **No depende de la ubicación:** mover el archivo no la toca |
| `exclusion`, `change`, `recognition_request` | Lo excluido del procesamiento; qué cambió; la cola de reconocimiento | Como hoy, por recurso |

**Por qué así.** El recurso es inmutable y se identifica por su huella, así una ubicación de evidencia nunca pasa a señalar otro contenido (D-551). La contención en carpetas es la ubicación; la contención dentro de un recurso (un compuesto) la da la aparición (sección 4); la contención entre recursos (ZIP, correo), `resource_part`. La procedencia es un registro aparte que sobrevive a los movimientos.

## 3. El registro común de piezas

| Tabla | Responsabilidad | Restricciones |
|---|---|---|
| `piece` | Cada pieza con identidad: `id` (UUID v7), `class`, quién la creó (`system` o `person`), su estado de vida (`active`, `merged`, `retired`) y, si se fusionó, en cuál | `class` es una de las clases definidas; `merged_into` apunta a una pieza de la misma clase |
| `document` | Datos del documento: su tipo, la versión que prevalece y por qué | `id` = `piece.id` de clase `document`; la versión que prevalece es una de sus versiones |
| `entity` | Persona u organización: `kind` (`person`, `organization`), nombre principal | `id` = `piece.id` de clase `entity` |
| `matter` | El asunto: nombre, `lifecycle` (`open`, `closed`, `archived`, D-475) y quién lo originó (`portal`, `system`, `person`). Puede no tener documentos | `id` = `piece.id` de clase `matter`. Activo o retirado va en `piece.state`: **las dos dimensiones de estado se conservan** |
| `thing` | La cosa: su tipo (del catálogo) y un nombre | `id` = `piece.id` de clase `thing` |
| `record` *(D-559)* | El expediente: organismo, número, año, título oficial | `id` = `piece.id` de clase `record`; `(organismo, número, año)` único |
| `template` *(D-559)* | La plantilla: su texto con los campos, su nombre, su certeza y si está propuesta o confirmada | `id` = `piece.id` de clase `template`. Nace propuesta; queda firme cuando una persona la confirma o la certeza es alta |
| `piece_name` | Los nombres con que aparece una pieza (alias) | Por pieza y nombre normalizado |
| `piece_identifier` | Identificadores de una pieza: DNI, CUIT, matrícula, dominio | `kind` es un identificador del catálogo; un mismo identificador en dos piezas de la misma clase se marca como duda, no se rechaza |

**Integridad entre `piece` y la tabla de su clase:** clave foránea de la tabla de clase a `piece`, y un disparador que rechaza una fila cuya `piece.class` no corresponde. Una clase nueva es una tabla nueva más su valor en `class` (D-552).

**El dueño de la instancia** es una entidad; un ajuste (`owner_piece_id`) la señala.

## 4. Documentos, versiones y apariciones

| Tabla | Responsabilidad | Restricciones |
|---|---|---|
| `version` | Un estado del documento que cambia algo que importa: si está firmada, presentada o registrada, completa, su fecha | Pertenece a un `document` |
| `appearance` | Una presencia de una versión en un recurso: `version_id`, `resource_id`, `selector` (archivo entero; páginas o regiones; posición), su orden dentro del recurso; **quién hizo el corte** (`rule`, `model`, `person`) y **si está `proposed` o `confirmed`** (D-556) | `(resource_id, selector)` único; el `selector` es válido para el tipo del recurso (una página solo en recursos con páginas) |
| `version_diff` | En qué difieren dos versiones del mismo documento | Las dos versiones son del mismo documento |

- **Representación:** dos apariciones de la **misma versión** en recursos distintos (el Word y su PDF). No hace falta una tabla.
- **Documento dentro de un compuesto:** una aparición con `selector` de páginas sobre el recurso del compuesto; el orden da la contención ordenada.
- **El compuesto es un documento propio** (D-560): su versión aparece con selector «archivo entero» sobre el mismo recurso, y la relación `includes`, con su orden, dice qué incluye. Así tiene su resumen, entra a un asunto con sus incluidos y se analiza como un conjunto.
- **Lo que se apoya en un corte propuesto** (D-556): la lectura del incluido no espera; las propuestas nuevas que salen de él (copias y versiones, asuntos, plantillas) esperan a que el corte se confirme.
- **Copias:** varias apariciones de la misma versión en recursos idénticos o con varias ubicaciones del mismo recurso.

## 5. Lectura

| Tabla | Responsabilidad | Restricciones |
|---|---|---|
| `reading` | Una lectura de una **versión**, con su base: revisión del catálogo, versión de instrucciones, motor, huella del texto leído, estado | Por versión; la base completa permite saber si está vigente |
| `mention` | Cómo nombra la versión a algo: tipo, texto escrito y normalizado, rol, cita, **ubicación** (`resource_id` + selector + posición en el texto) y la pieza a la que se resolvió | La ubicación apunta a un recurso que es aparición de esa versión. Se resuelve a una pieza o queda sin resolver |
| `mention_identifier` | Los identificadores que trae una mención, con su cita y ubicación | Por mención |

## 6. Relaciones, evidencia y decisiones

| Tabla | Responsabilidad | Restricciones |
|---|---|---|
| `relation` | Lo que se sostiene. Sujeto (pieza), tipo (catálogo), objeto (**pieza o valor**), **contexto** (pieza dentro de la cual vale), rol, calificación, datos del valor (monto, moneda, concepto), **desde** y **hasta** con su precisión, certeza, estado (`proposed`, `confirmed`, `rejected`) | Exactamente uno de objeto-pieza u objeto-valor. El tipo admite esas clases de sujeto, objeto y contexto (lo verifica el código con el catálogo). **No hay unicidad por sujeto-tipo-objeto:** varios períodos son varias filas |
| `evidence` | Cada respaldo: postura (`supports`, `contradicts`), qué respalda (la relación, su desde o su hasta), clase (`text`, `portal`, `container_order`, `folder_rule`), y según la clase la versión, menciones, cita y ubicación, o la procedencia, o la regla | Según la clase, sus campos obligatorios (una evidencia de texto tiene versión y ubicación; una del portal, procedencia) |
| `decision` | Lo que hizo una persona: `join`, `separate`, `wrong`, `confirm`, `name`, `include`, `exclude`, `set_prevailing`, `approve_term`, `reject_term`, `split_pages`, `join_pages`, `confirm_cut`; sobre qué (anclas), con qué valor, motivo, lo que se le mostró, estado (`active`, `undone`, `to_review`) | **Las anclas son claves estables, no ids de filas** (D-561): una pieza que creó una persona, por su `piece.id`; lo que encontró la aplicación, por sus menciones; una interpretación de un documento, además por su versión y su evidencia; un corte, por la huella del recurso y sus páginas. Las menciones y las versiones se identifican por la huella del recurso, las páginas y el nombre o la cita normalizados. Si un ancla no encuentra nada **o encuentra más de un destino**, la decisión pasa a `to_review` y no se aplica; nunca se borra en silencio |
| `doubt` | Lo que la aplicación no pudo resolver: dos piezas que podrían ser una, dos relaciones que se contradicen, una versión que prevalece sin reglas suficientes | Se recalcula; desaparece cuando una decisión la resuelve |
| `verdict` | La conclusión que el programa sacó de una respuesta del modelo, con la clase de pregunta | Apunta a la respuesta en `model_answer` |
| `model_answer` | Cada respuesta del modelo tal como llegó, una vez por pregunta (R-20) | Por huella de la pregunta |

**Cómo se calcula una relación.** La resolución junta la evidencia que habla del mismo hecho **según lo que declara su tipo en el catálogo** (D-561): los datos que lo identifican (sujeto, objeto, contexto, calificación, **rol y datos del valor**, los que correspondan a ese tipo) y si admite uno o varios valores. Un valor distinto es otro hecho si el tipo admite varios, y una contradicción sobre el mismo hecho si admite uno solo. Después la ordena en períodos: un fin completa el período que encaja; un inicio posterior a un fin conocido abre otro. Las decisiones activas se aplican encima y nunca se pisan.

**Lo que creó el usuario** (el dueño de la instancia, un caso sin documentos, un cliente cargado a mano) es una pieza con `created_by = person`; sus relaciones llevan como evidencia la decisión que las creó.

## 7. Catálogo y aprendizaje

| Tabla | Responsabilidad | Restricciones |
|---|---|---|
| `catalog_entry` | Las entradas de **la instancia**: clase de entrada (tipo de documento, tipo de cosa, identificador, rol, tipo de relación, regla), clave, revisión, contenido, **cómo se aplica** (`code`, `model`, `both`), estado (`proposed`, `approved`, `corrected`, `rejected`), procedencia (de qué documentos o decisiones salió), quién la aprobó y cuándo | `(clave, revisión)` única. Lo que viene con la aplicación sigue en los archivos de `resources/catalog/`; el catálogo efectivo es la unión, y la instancia puede ajustar una entrada de la aplicación por su clave |
| `work_item` | Lo que hay que recalcular: etapa (`text`, `pages`, `documents`, `reading`, `identity`, `relations`, `matters`, `summaries`) y documento **o pieza** (la página de una persona) | Una fila por etapa y objeto |

Cada resultado (`reading`, `relation`, `document`) guarda la **huella del catálogo efectivo** y de las instrucciones con que se produjo. Cuando cambia una entrada se crean `work_item` para la etapa que la usa en los documentos donde aplica (D-552). **Y cuando una etapa cambia un resultado, para las etapas que dependen de él**, en el orden fijo de la sección 9 y con alcance conservador: si cambia quién es una persona, todos los documentos y piezas donde aparece (D-561). Con la caché de respuestas (D-555), eso le vuelve a preguntar al modelo solo cuando la pregunta cambió.

**En la base de entrenamiento** (D-545) se suma `training_example`: una versión de documento, la entrada del catálogo o la respuesta correcta que ilustra, y su **propósito** (`teach` o `measure`), para que lo que enseña nunca se use para medir.

## 8. Lo derivado y lo que queda igual

- **Por documento, de la versión que prevalece:** el resumen (`document_summary`, con citas a la versión), el tipo y los datos mostrados, los índices de búsqueda (`content_fts` y `content_search` pasan a ser por documento).
- **Vectores, diferidos** (D-557): `version_vector`, el vector de cada versión (y de cada página), con el modelo y su versión. Lo calcula un proveedor (Core, en AWS; mensaje 004); Desktop no lo calcula en la PC. Sin proveedor no hay vectores y la búsqueda es por palabras. Si el modelo cambia, se recalculan todos.
- **Por pieza:** las páginas que escribe la aplicación (`piece_page`), notas (`piece_note`), investigaciones (`piece_research`), apuntando a `piece`. Las preguntas guardadas (`saved_query`) y el panorama quedan.
- **El motor de parecidos** sigue midiendo (firmas de texto, vecinos, texto rutinario, datos clave) sobre **el texto de cada versión** (el de sus páginas: cada documento incluido por separado), no sobre el recurso entero (D-560). Las huellas de imagen de `resource_page` solo desempatan entre mismo documento y versión, y proponen candidatos entre escaneos sin texto; nunca son prueba sola. Lo que concluye no va a una tabla de pares: arma versiones y apariciones (mismo documento), relaciones `made_with_template` o de parecido (documentos distintos) y su evidencia.
- **Igual que hoy:** etiquetas (`label_group`, `label`, `label_rule`, `label_manual`, `location_label` sobre `resource_location`), `operation_log`, `local_program`, `local_program_use`, ajustes.
- **Salen:** `content` (se parte), `derivative` (va a `resource_text` y a los derivados por documento), `similar_pair` y `similar_group` (sección anterior), `knowledge_matter_*` (pasan a relaciones, decisiones, dudas y veredictos), `type_decision` y `template_field_choice` (a `decision`), `project`, `project_origin` y todo lo del wiki.

## 9. Reinicio y reproceso

1. **Respaldo** de la base actual, como siempre (D-496). Queda solo para leer, y al final se compara el reproceso contra ella (D-553). Todo el procesamiento se hace en la PC del dueño con el agente local.
2. **Extraer el aprendizaje** de la base real (la lista está en el inventario): decisiones de identidad, tipos aprobados, relaciones entre documentos corregidas, campos de plantillas, criterios de asunto. Sale como propuestas de entradas del catálogo y como casos para la hoja de respuestas, revisadas por el dueño. Lo hace el agente local en la PC del dueño (D-553). En la base real (versión 29) hay poco: ninguna decisión de identidad ni de tipos, ningún par de copias decidido y ningún campo de plantilla elegido; sí 65 documentos puestos o sacados de un caso a mano, 2 casos creados por una persona, 3 etiquetas manuales y 1 regla de etiqueta. El aprendizaje está sobre todo en las decisiones, las hojas de respuestas y el catálogo del repositorio.
3. **Revisar las hojas de respuesta** contra los criterios nuevos de documento y versión (D-527, D-534, D-551).
4. **La base nueva es un archivo nuevo,** con su propia numeración de versiones. La aplicación nueva no lee la base vieja ni trae código para migrarla; la vieja queda en su lugar hasta que el dueño la borre. Se retiran también las importaciones únicas del wiki (`pages/import`, `chat_files`). **Desde su primera versión vale lo de siempre:** cada actualización posterior respeta los datos, con su migración y un respaldo antes de tocarlos (D-496, D-560).
   - **La caché** (D-555): un guion aparte, fuera de la app, copia de la base vieja las respuestas del modelo, por la huella de la pregunta; el texto reconocido por página sigue en `recognized/`, al lado de la base, atado a la huella de los bytes. Solo se usa para preguntas idénticas: las que se hicieron sobre compuestos enteros no se vuelven a usar, porque el texto de cada incluido es otro.
5. **Procesar por etapas, midiendo cada una contra su hoja** (R-17): recorrido de orígenes → recursos y texto → páginas → documentos, versiones y apariciones → lectura → identidad → relaciones → asuntos → resúmenes. La lectura automática sigue apagada hasta que el dueño la encienda; **la lectura con el modelo usa cupo de Codex y se avisa antes.**

## 10. Los escenarios, recorridos sobre este diseño

| Escenario | Cómo lo representa |
|---|---|
| Un Word y su PDF con el mismo texto | Dos `resource`; un `document` con una `version` y **dos `appearance`** (una por recurso). No se confunden bytes, versión y representación |
| Una escritura suelta y dentro de un PDF compuesto | Un `document`; la versión tiene una aparición con selector «archivo entero» sobre el recurso suelto y otra con selector «páginas 4 a 9» sobre el recurso del compuesto. Cada ubicación y cada `provenance` se conservan |
| Un borrador y un firmado con un dato distinto | Un `document`, dos `version`; prevalece la firmada (regla, o `decision` `set_prevailing`). Cada `reading` es por versión, y la `evidence` de cada dato apunta a su versión: la ficha muestra la firmada y se puede rastrear el dato del borrador |
| Dos formularios de la misma plantilla con distintos titulares | Dos `document` distintos (los datos clave difieren); los dos con relación `made_with_template` a la misma `template`; las menciones de cada titular se resuelven a entidades distintas |
| Una persona con cargos en dos organizaciones | Dos `relation` de tipo cargo, cada una con su **contexto** (la organización): distinto contexto no es contradicción |
| Una persona que sale y vuelve a entrar | Dos `relation` de empleo con el mismo sujeto y objeto, cada una con su desde y hasta y su evidencia. El inicio posterior al fin conocido abre la segunda |
| Un documento del portal que se mueve de carpeta | Cambia la `resource_location`; el `resource` y su `provenance` (portal, expediente, actuación) no cambian, ni la relación con el `record` |
| El operador crea un caso sin documentos | Una `piece` de clase `matter` con `created_by = person` y su `decision` de origen; sin relaciones de documentos, igual es válida y visible |
| El operador rechaza una unión y se reprocesa | La `decision` `separate` se ancla a claves estables de las menciones; al releer, la resolución la vuelve a encontrar y no une. Si el ancla ya no encuentra nada, pasa a `to_review` |
| Cambia una regla del catálogo | Nueva revisión en `catalog_entry`; cambia la huella del catálogo efectivo; se crean `work_item` para la etapa que la usa en los documentos donde aplica; las `decision` activas se respetan; cada resultado dice con qué revisión salió |
| Una persona participa en un mismo asunto con dos roles | Dos `relation`: el tipo «participa en un asunto» declara que el rol identifica el hecho, así que la evidencia de cada rol no se junta (D-561) |
| Una persona marca como errónea una lectura del borrador | La `decision` `wrong` se ancla a la versión del borrador y a su evidencia: la versión firmada no la hereda (D-561) |
| Un compuesto con siete documentos, en un asunto | Un `document` compuesto con la relación `includes` hacia los siete, en orden; el asunto apunta al compuesto y los incluidos van con él; uno puede entrar solo a otro asunto (D-556, D-560) |
| Una persona corrige un corte | La `decision` `split_pages` o `join_pages` se ancla a la huella del recurso y a sus páginas; al reprocesar, el corte vuelve igual (D-560) |

## 11. Abierto

- Las regiones de una página (dos documentos en una hoja), después del corte por páginas.
- Los tipos de cosa iniciales.
- Lo que propone el compuesto como conjunto (relaciones entre personas y el control contra lo que anuncia el escrito), con la etapa de relaciones.
- El detalle de columnas, índices y disparadores se fija al construir.
