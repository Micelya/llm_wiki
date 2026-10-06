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
- **Propósito:** que un proyecto use carpetas externas como fuentes sin copiarlas, con un único componente (`src-tauri/src/source_volume/`) que decide dónde está cada fuente.
- **Toca upstream:** `src-tauri/src/lib.rs`, `src-tauri/src/commands/fs.rs`
- **Upstream:** solo-fork
- **PR:** pendiente (rama `feat/source-volume`)

## Reabsorbidos

Cambios que upstream ya incorporó (ID · versión de upstream que lo trae).

_Ninguno todavía._
