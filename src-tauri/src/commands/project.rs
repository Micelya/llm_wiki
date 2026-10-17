use std::fs;
use std::path::Path;

use chrono::Local;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::panic_guard::run_guarded;
use crate::types::wiki::WikiProject;

#[tauri::command]
pub fn create_project(name: String, path: String) -> Result<WikiProject, String> {
    run_guarded("create_project", || create_project_impl(name, path))
}

pub(crate) fn create_project_impl(name: String, path: String) -> Result<WikiProject, String> {
    let root = Path::new(&path).join(&name);

    if root.exists() {
        return Err(format!("Directory already exists: '{}'", root.display()));
    }

    // Create all required subdirectories
    let dirs = [
        "raw/sources",
        "raw/assets",
        "wiki/entities",
        "wiki/concepts",
        "wiki/sources",
        "wiki/queries",
        "wiki/comparisons",
        "wiki/synthesis",
    ];
    for dir in &dirs {
        fs::create_dir_all(root.join(dir))
            .map_err(|e| format!("Failed to create directory '{}': {}", dir, e))?;
    }

    let today = Local::now().format("%Y-%m-%d").to_string();

    // schema.md
    let schema_content = format!(
        r#"# Esquema del wiki

Wiki de un estudio jurídico: lo escribe la app desde los documentos del archivo. Cada página dice lo que dicen los documentos, con sus fuentes; nunca conocimiento general ni opiniones que los documentos no contienen.

## Page Types

| Type | Directory | Purpose |
|------|-----------|---------|
| entity | wiki/entities/ | Personas, organizaciones y asuntos (las escribe la app desde su registro) |
| concept | wiki/concepts/ | Figuras jurídicas, cláusulas, obligaciones y trámites que usan los documentos |
| source | wiki/sources/ | Una página por documento del archivo: qué es, fecha, partes y hechos clave |
| query | wiki/queries/ | Preguntas abiertas del estudio |
| comparison | wiki/comparisons/ | Comparaciones entre documentos o versiones |
| synthesis | wiki/synthesis/ | Resúmenes transversales |

## Nombres de archivo

- Archivos: `kebab-case.md`, siempre con `.md`.
- Conceptos: la figura jurídica como frase (por ejemplo, `intereses-por-mora.md`).
- Documentos: la app decide la ruta de la página de cada documento.

## Encabezado

Todas las páginas empiezan con un encabezado YAML:

```yaml
---
type: entity | concept | source | query | comparison | synthesis | overview
title: El mismo título que el primer encabezado de la página
tags: []
related: []
sources: []
created: AAAA-MM-DD
updated: AAAA-MM-DD
---
```

Sin campos de referencia académica (`authors`, `year`, `url`, `venue`).

## Índice y registro

`wiki/index.md` y `wiki/overview.md` los arma la app. `wiki/log.md` registra lo que se incorporó, del más reciente al más viejo.

## Enlaces

- `[[nombre-de-pagina]]` solo hacia páginas que existen; un nombre sin página queda como texto.
- Las páginas hablan de los documentos, nunca del propio wiki ni del material que recibió.

## Datos que difieren

Cuando dos documentos dicen cosas distintas del mismo dato, la página muestra los dos valores con su documento; no elige uno.
"#
    );
    write_file_inner(root.join("schema.md"), &schema_content)?;

    // purpose.md
    let purpose_content = r#"# Project Purpose

## Goal

<!-- What are you trying to understand or build? -->

## Key Questions

<!-- List the primary questions driving this research -->

1.
2.
3.

## Scope

<!-- What is in scope? What is explicitly out of scope? -->

**In scope:**
-

**Out of scope:**
-

## Thesis

<!-- Your current working hypothesis or conclusion (update as research progresses) -->

> TBD
"#;
    write_file_inner(root.join("purpose.md"), purpose_content)?;

    // wiki/index.md
    let index_content = r#"# Wiki Index

## Entities

## Concepts

## Sources

## Queries

## Comparisons

## Synthesis
"#;
    write_file_inner(root.join("wiki/index.md"), index_content)?;

    // wiki/log.md
    let log_content = format!(
        r#"# Research Log

## {today}

- Project created
"#
    );
    write_file_inner(root.join("wiki/log.md"), &log_content)?;

    // wiki/overview.md
    let overview_content = r#"---
type: overview
title: Project Overview
tags: []
related: []
---

# Overview

<!-- Provide a high-level summary of what this wiki covers and its current state. Update regularly as understanding deepens. -->
"#;
    write_file_inner(root.join("wiki/overview.md"), overview_content)?;

    // .obsidian config for Obsidian compatibility
    fs::create_dir_all(root.join(".obsidian"))
        .map_err(|e| format!("Failed to create .obsidian: {}", e))?;

    // Obsidian app config: set attachment folder, exclude hidden dirs
    let obsidian_app_config = r#"{
  "attachmentFolderPath": "raw/assets",
  "userIgnoreFilters": [
    ".cache",
    ".llm-wiki",
    ".superpowers"
  ],
  "useMarkdownLinks": false,
  "newLinkFormat": "shortest",
  "showUnsupportedFiles": false
}"#;
    write_file_inner(root.join(".obsidian/app.json"), obsidian_app_config)?;

    // Obsidian appearance: dark mode
    let obsidian_appearance = r#"{
  "baseFontSize": 16,
  "theme": "obsidian"
}"#;
    write_file_inner(root.join(".obsidian/appearance.json"), obsidian_appearance)?;

    // Enable graph view and backlinks core plugins
    let obsidian_core_plugins = r#"{
  "file-explorer": true,
  "global-search": true,
  "graph": true,
  "backlink": true,
  "tag-pane": true,
  "page-preview": true,
  "outgoing-link": true,
  "starred": true
}"#;
    write_file_inner(
        root.join(".obsidian/core-plugins.json"),
        obsidian_core_plugins,
    )?;

    Ok(WikiProject {
        name,
        // Forward slashes for cross-platform consistency in the TS layer.
        path: root.to_string_lossy().replace('\\', "/"),
    })
}

#[tauri::command]
pub fn open_project(path: String) -> Result<WikiProject, String> {
    run_guarded("open_project", || {
        let root = Path::new(&path);

        validate_wiki_project_root(root)?;

        // Derive project name from the directory name
        let name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        Ok(WikiProject {
            name,
            // Forward slashes for cross-platform consistency in the TS layer.
            path: path.replace('\\', "/"),
        })
    })
}

#[tauri::command]
pub fn open_project_folder(app: AppHandle, path: String) -> Result<(), String> {
    run_guarded("open_project_folder", || {
        let root = Path::new(&path);
        validate_wiki_project_root(root)?;

        let canonical = root
            .canonicalize()
            .map_err(|e| format!("Failed to resolve project path '{}': {}", path, e))?;
        let canonical = canonical.to_string_lossy().to_string();

        match app.opener().open_path(canonical.clone(), None::<&str>) {
            Ok(()) => Ok(()),
            Err(open_err) => app
                .opener()
                .reveal_item_in_dir(canonical)
                .map_err(|reveal_err| {
                    format!(
                        "Failed to open project folder: {}; reveal fallback also failed: {}",
                        open_err, reveal_err
                    )
                }),
        }
    })
}

#[tauri::command]
pub fn open_path_in_project(
    app: AppHandle,
    project_path: String,
    target_path: String,
) -> Result<(), String> {
    run_guarded("open_path_in_project", || {
        let root = Path::new(&project_path);
        validate_wiki_project_root(root)?;

        let root_canonical = root
            .canonicalize()
            .map_err(|e| format!("Failed to resolve project path '{}': {}", project_path, e))?;
        let target = Path::new(&target_path);
        let target = if target.is_absolute() {
            target.to_path_buf()
        } else {
            root_canonical.join(target)
        };
        let target_canonical = target.canonicalize().map_err(|e| {
            format!(
                "Failed to resolve target path '{}': {}",
                target.display(),
                e
            )
        })?;

        if !target_canonical.starts_with(&root_canonical) {
            return Err(format!(
                "Refusing to open a path outside the project: '{}'",
                target_canonical.display()
            ));
        }

        let target = target_canonical.to_string_lossy().to_string();
        match app.opener().open_path(target.clone(), None::<&str>) {
            Ok(()) => Ok(()),
            Err(open_err) => app
                .opener()
                .reveal_item_in_dir(target)
                .map_err(|reveal_err| {
                    format!(
                        "Failed to open project path: {}; reveal fallback also failed: {}",
                        open_err, reveal_err
                    )
                }),
        }
    })
}

fn validate_wiki_project_root(root: &Path) -> Result<(), String> {
    if !root.exists() {
        return Err(format!("Path does not exist: '{}'", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("Path is not a directory: '{}'", root.display()));
    }

    if !root.join("schema.md").exists() {
        return Err(format!(
            "Not a valid wiki project (missing schema.md): '{}'",
            root.display()
        ));
    }
    if !root.join("wiki").is_dir() {
        return Err(format!(
            "Not a valid wiki project (missing wiki/ directory): '{}'",
            root.display()
        ));
    }

    Ok(())
}

fn write_file_inner(path: std::path::PathBuf, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create parent dirs for '{}': {}",
                path.display(),
                e
            )
        })?;
    }
    fs::write(&path, contents)
        .map_err(|e| format!("Failed to write file '{}': {}", path.display(), e))
}
