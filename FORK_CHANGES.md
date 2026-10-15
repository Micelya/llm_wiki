# Diferencias con upstream

Registro de lo que este fork hace distinto de [nashsu/llm_wiki](https://github.com/nashsu/llm_wiki) y **para qué**.
El detalle de cada cambio (qué líneas, qué pruebas) está en su PR; acá no se repite.

## Reglas

- Una entrada por **propósito**, no por commit ni por PR. Si un PR posterior sirve al mismo propósito, se agrega a la entrada existente.
- Cada campo ocupa una línea. Si hace falta más, va en el PR.
- El archivo lista solo diferencias **vigentes**. Cuando upstream incorpora un cambio, la entrada se borra y se anota en "Reabsorbidos".
- IDs correlativos (`F-001`, `F-002`…); no se reutilizan.

## Campos

| Campo | Contenido |
|---|---|
| Propósito | Qué problema resuelve o qué capacidad agrega, en una frase |
| Toca upstream | Archivos del proyecto original que se modifican (los archivos nuevos no se listan) |
| Upstream | `candidato` (conviene proponerlo) · `propuesto` (PR abierto allá) · `solo-fork` (no aplica a upstream) |
| PR | PR de este repositorio; el de upstream si existe |

## Vigentes

### F-001 · Encabezados de página con dos puntos sin comillas
- **Propósito:** que una página cuyo título contiene `:` no pierda su tipo, relacionados y fuentes (aparecía como "Other" y casi sin relaciones en el grafo).
- **Toca upstream:** `src/lib/frontmatter.ts`, `src/lib/ingest-sanitize.ts`
- **Upstream:** candidato
- **PR:** #1

### F-002 · Volumen de fuentes (fuentes sin copiar)
- **Propósito:** que un proyecto use carpetas externas como fuentes sin copiarlas, con un único componente (`src-tauri/src/source_volume/`) que decide dónde está cada fuente, detecta sus cambios y distingue un archivo borrado de un origen que no se puede leer.
- **Toca upstream:** `src-tauri/src/lib.rs`, `src-tauri/src/commands/fs.rs`, `src-tauri/src/commands/extract_images.rs`, `src-tauri/src/commands/file_sync.rs`, `src/lib/source-lifecycle.ts`, `src/components/sources/sources-view.tsx`, `src/i18n/*.json`, `src-tauri/tauri.windows.conf.json`
- **Upstream:** solo-fork
- **PR:** #7, #12

### F-003 · Workflows de GitHub endurecidos
- **Propósito:** que una acción de terceros o una dependencia comprometida no pueda robar secretos ni alterar releases, y que las etiquetas traídas de upstream no publiquen releases acá (se habilita con la variable `ENABLE_TAG_RELEASE`).
- **Toca upstream:** `.github/workflows/ci.yml`, `.github/workflows/build.yml`
- **Upstream:** candidato (salvo la variable `ENABLE_TAG_RELEASE`)
- **PR:** #3. Desde el PR #49, la comprobación y los instaladores corren solo en Windows (P-082); esa parte es solo-fork

### F-004 · Identidad y versionado propios de la app
- **Propósito:** que esta app no comparta datos ni avisos de actualización con LLM Wiki: nombre «Micelya Desktop» (también en los textos de la interfaz, con un único reemplazo en `src/i18n/product-name.ts`), identificador `com.micelya.desktop`, versión propia (desde `0.1.0`), releases con etiquetas `mi-desktop-v*` y chequeo de actualizaciones contra este repositorio.
- **Toca upstream:** `src-tauri/tauri.conf.json`, `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `.github/workflows/build.yml`, `.github/scripts/package-windows-portable.ps1`, `index.html`, `src-tauri/src/lib.rs`, `src-tauri/src/tray.rs`, `src/lib/update-check.ts`, `src/lib/changelog.ts`, `src/App.tsx`, `src/components/settings/sections/about-section.tsx`, `src/i18n/index.ts`, `src/components/layout/icon-sidebar.tsx`, `src/main.tsx`
- **Upstream:** solo-fork
- **PR:** #5, #11

### F-005 · Actualización mensual de las acciones fijadas
- **Propósito:** que las acciones de GitHub fijadas a versión exacta no queden congeladas: Dependabot propone un PR mensual agrupado, solo para acciones.
- **Toca upstream:** ninguno (agrega `.github/dependabot.yml`)
- **Upstream:** candidato
- **PR:** #6

### F-006 · Reconocimiento de texto en escaneos e imágenes
- **Propósito:** que las páginas de PDF sin texto y las fotos de documentos aporten su texto completo, leído por un motor intercambiable (`src-tauri/src/source_volume/recognition*.rs`) dentro de la tarea de ingesta de cada fuente y con avance por página; el texto leído se guarda en el proyecto por hash del archivo (`raw/recognized/`), así ningún archivo se lee dos veces. Primer motor: Codex CLI.
- **Toca upstream:** `src-tauri/src/commands/fs.rs`, `src/commands/fs.ts`, `src/lib/ingest.ts`
- **Upstream:** solo-fork
- **PR:** #7, #14, #17

### F-007 · Chat con proveedores CLI: busca en el wiki y lee las páginas
- **Propósito:** que el chat con Codex CLI o Claude Code responda con el contenido del wiki: no envía las skills del modo automático (desactivaban la búsqueda), pide hasta 10 páginas y agrega al pedido su texto (`src/lib/cli-chat-context.ts`).
- **Toca upstream:** `src/components/chat/chat-panel.tsx`
- **Upstream:** candidato
- **PR:** #8, #13

### F-008 · Etiquetas legibles en el grafo
- **Propósito:** que las etiquetas del grafo no se superpongan: solo los nodos más conectados llevan etiqueta sin acercar el zoom, los títulos largos se acortan y un clic deja resaltado el nodo con sus vecinos (`src/lib/graph-labels.ts`).
- **Toca upstream:** `src/components/graph/graph-view.tsx`
- **Upstream:** candidato
- **PR:** #9

### F-009 · Buscador en español
- **Propósito:** que una pregunta en español no arrastre palabras como «por», «qué» o «de», que coinciden con casi todas las páginas y desordenan los resultados (`src-tauri/src/commands/search_spanish.rs`).
- **Toca upstream:** `src-tauri/src/commands/search.rs`, `src-tauri/src/commands/mod.rs`
- **Upstream:** candidato
- **PR:** #13

### F-010 · Aviso de importación informativo
- **Propósito:** que «Imported 31, skipped 2» no se muestre como error cuando los archivos omitidos lo fueron por las reglas de exclusión.
- **Toca upstream:** `src/components/sources/sources-view.tsx`
- **Upstream:** candidato
- **PR:** #10

### F-011 · Catálogo de fuentes y deduplicación exacta
- **Propósito:** que el mismo documento en varias rutas o formatos se ingiera una sola vez y conserve todas sus ubicaciones: el catálogo (`src-tauri/src/source_volume/catalog.rs`) separa contenido de ubicación; un archivo idéntico se detecta por su SHA-256 antes de encolarlo y el mismo texto en otro formato dentro de la tarea de ingesta, antes de llamar al modelo (`src/lib/source-catalog.ts`).
- **Toca upstream:** `src-tauri/src/lib.rs`, `src/lib/ingest.ts`, `src/lib/source-lifecycle.ts`, `src/lib/ingest-cache.ts`
- **Upstream:** solo-fork
- **PR:** #15, #17

### F-012 · Historial de versiones en Windows
- **Propósito:** que el historial de versiones de un archivo se encuentre en Windows: la clave bajo la que se guarda ya no depende de si la ruta llegó con `/` o con `\` (antes se guardaba y el panel lo mostraba vacío); además deja en cero los fallos de `cargo test --lib` en Windows.
- **Toca upstream:** `src-tauri/src/commands/file_history.rs`, `src-tauri/src/commands/fs.rs`
- **Upstream:** candidato
- **PR:** #21

### F-013 · Base documental
- **Propósito:** que la app sepa qué archivos hay en un espacio, dónde está cada copia y cuáles son el mismo documento, y que lo derivado de cada documento se obtenga una sola vez: un componente propio (`src-tauri/src/docbase/`) con una base SQLite por espacio, sus pantallas (`src/components/docbase/`: Inicio, Documentos, Orígenes), el proyecto general que la app crea y abre sola con su wiki activable, etiquetas por grupos con reglas de carpeta, exclusiones, un índice de las páginas y enlaces del wiki, el vocabulario por vertical (`src/vocabulary/`, con variantes por país y los nombres de las relaciones), y los documentos parecidos: el motor (`src-tauri/src/docbase/similar/`) y sus pantallas (sección en el detalle de un documento, línea en Inicio, filtros en Documentos), y el tipo, la etapa y los datos de cada documento con el modelo detrás de un contrato intercambiable (`src-tauri/src/source_volume/asking.rs`), con su panel en Orígenes, su columna y filtro en Documentos y su lugar en el detalle.
- **Toca upstream:** `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/src/lib.rs`, `src/stores/wiki-store.ts`, `src/components/layout/content-area.tsx`, `src/components/layout/icon-sidebar.tsx`, `src/components/layout/research-panel-nav.ts`, `src-tauri/src/commands/fs.rs`, `src-tauri/src/commands/project.rs`, `src/App.tsx`, `src/lib/project-store.ts`, `src/lib/source-lifecycle.ts`
- **Upstream:** solo-fork
- **PR:** #24, #25, #26, #27, #28, #29, #31, #33, #42, #43, #47

### F-014 · Detección de idioma entre lenguas romances
- **Propósito:** que un texto en español, italiano o portugués no se tome por francés solo por traer «la» y una letra acentuada (un wiki entero salió en francés): entre esas cuatro lenguas gana la que junta más palabras y letras propias.
- **Toca upstream:** `src/lib/detect-language.ts`
- **Upstream:** candidato
- **PR:** #33

### F-015 · Una sola instancia de la app
- **Propósito:** que abrir la app cuando ya está corriendo (cerrar la ventana solo la oculta en la bandeja) traiga su ventana al frente en vez de arrancar otra copia, que competía con la primera por los puertos locales y quedaba acumulada en segundo plano.
- **Toca upstream:** `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/src/lib.rs`, `src-tauri/src/tray.rs`
- **Upstream:** candidato
- **PR:** #34

### F-016 · Lo completado en un formulario PDF es parte de su texto
- **Propósito:** que dos copias del mismo formulario PDF completadas para personas distintas no se lean igual: al texto de cada página se le agrega lo escrito en sus campos, que antes se perdía (la base las tomaba por el mismo documento y el wiki recibía el formulario vacío).
- **Toca upstream:** `src-tauri/src/commands/extract_images.rs`
- **Upstream:** candidato
- **PR:** #38

## Reabsorbidos

Cambios que upstream ya incorporó (ID · versión de upstream que lo trae).

_Ninguno todavía._
