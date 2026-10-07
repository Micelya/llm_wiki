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

### F-003 · Workflows de GitHub endurecidos
- **Propósito:** que una acción de terceros o una dependencia comprometida no pueda robar secretos ni alterar releases, y que las etiquetas traídas de upstream no publiquen releases acá (se habilita con la variable `ENABLE_TAG_RELEASE`).
- **Toca upstream:** `.github/workflows/ci.yml`, `.github/workflows/build.yml`
- **Upstream:** candidato (salvo la variable `ENABLE_TAG_RELEASE`)
- **PR:** #3

### F-004 · Identidad y versionado propios de la app
- **Propósito:** que esta app no comparta datos ni avisos de actualización con LLM Wiki: nombre «Micelya Desktop», identificador `com.micelya.desktop`, versión propia (desde `0.1.0`), releases con etiquetas `mi-desktop-v*` y chequeo de actualizaciones contra este repositorio.
- **Toca upstream:** `src-tauri/tauri.conf.json`, `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `.github/workflows/build.yml`, `.github/scripts/package-windows-portable.ps1`, `index.html`, `src-tauri/src/lib.rs`, `src-tauri/src/tray.rs`, `src/lib/update-check.ts`, `src/lib/changelog.ts`, `src/App.tsx`, `src/components/settings/sections/about-section.tsx`
- **Upstream:** solo-fork
- **PR:** #5

### F-005 · Actualización mensual de las acciones fijadas
- **Propósito:** que las acciones de GitHub fijadas a versión exacta no queden congeladas: Dependabot propone un PR mensual agrupado, solo para acciones.
- **Toca upstream:** ninguno (agrega `.github/dependabot.yml`)
- **Upstream:** candidato
- **PR:** #6

## Reabsorbidos

Cambios que upstream ya incorporó (ID · versión de upstream que lo trae).

_Ninguno todavía._
