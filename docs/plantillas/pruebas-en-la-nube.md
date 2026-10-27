# Pruebas en la nube: bajar los datos y la base, usarlos y entregar

Para un agente de Claude Code que corre en la nube sobre este repositorio. Las reglas cortas están en `CLAUDE.md` («Pruebas en la nube: traer y entregar archivos»); acá está el paso a paso. Las decisiones son la D-529 (dos entornos), la D-546 (traer y entregar archivos) y la D-547 (la base en Drive).

## 1. Antes de empezar

- **Leé los guardrails de consumo de `CLAUDE.md`** y escribí tu plan con un tope de fichas por parte, antes de bajar nada.
- **El dueño te pasa en el prompt el enlace de la carpeta de Drive `POC-Micelya`.** Es de solo lectura y da acceso a datos reales sin iniciar sesión: **nunca va al repositorio**, ni a un commit, ni a un mensaje a Core.
- **Todo lo que bajes tiene datos reales.** Va a una carpeta fuera del repositorio, por ejemplo `~/prueba/`, y no se sube a ningún servicio externo.
- **Qué es cada cosa** de la carpeta, y cómo se llaman las carpetas de cliente, lo dice `pruebas/LEEME.md`. Si no sabés qué necesitás, bajá primero solo ese archivo: el guion acepta también un archivo suelto en lugar de una subcarpeta.

## 2. Bajar solo lo que usa la prueba

El guion `docs/plantillas/bajar-de-drive.py` lista la carpeta y baja cada archivo directo a disco, sin pasar por tu contexto. **No uses el conector de Drive** para bajar, listar ni contar: una prueba gastó 133 mil fichas solo en contar la carpeta.

Bajá en dos destinos separados: los documentos de los clientes por un lado y `pruebas/` por otro. Así, si la prueba vuelve a recorrer los documentos, no mezcla las hojas de respuestas con ellos.

```bash
E="<enlace que pasó el dueño>"
python docs/plantillas/bajar-de-drive.py "$E" ~/prueba/poc "<carpeta de cliente>" "<otra carpeta de cliente>"
python docs/plantillas/bajar-de-drive.py "$E" ~/prueba/pruebas pruebas/base pruebas/hojas-de-respuestas pruebas/revision-de-datos
```

| Subcarpeta | Para qué | Tamaño aproximado |
|---|---|---|
| `pruebas/base` | La base de la app y su ficha | 12 MB |
| `pruebas/hojas-de-respuestas` | Las verdades contra las que se mide | 2 MB |
| `pruebas/revision-de-datos` | La lista de palabras sensibles, para el punto 5 | menos de 1 MB |
| `pruebas/expedientes-pjn` | Un expediente bajado del portal del PJN | 18 MB |
| `pruebas/documentos-de-prueba` | Los seis PDF de la hoja de partes | 20 MB |
| `pruebas/estado-de-la-app` | Lo que sabe la app, en planillas | menos de 1 MB |
| las carpetas de cliente (todo lo que no es `pruebas/`) | Los documentos originales | 85 MB |

El guion deja los archivos en `<destino>/datos/` y su lista en `<destino>/lista.tsv`. Necesita que el entorno permita `drive.google.com` y `drive.usercontent.google.com`.

## 3. Comprobar la base

Si bajaste `pruebas/base`, el guion termina con una línea así:

```
base del 2026-10-26 22:10, versión 29: huella ok
```

**Si dice «NO COINCIDE», no la uses:** Drive todavía la estaba subiendo o la descarga se cortó. Volvé a bajarla en unos minutos y, si sigue igual, avisale al dueño.

La ficha (`datos/pruebas/base/ficha.json`) dice de cuándo es la copia, la versión de la base, el commit del código de la app (y si ya está en `main`), cuántas filas tiene cada tabla y dónde está en Drive cada origen.

## 4. Usar la base

**Trabajá siempre sobre una copia:** descomprimí `micelya-base.zip` en una carpeta de trabajo y dejá el zip intacto.

```bash
mkdir -p ~/prueba/trabajo && cd ~/prueba/trabajo
python -m zipfile -e ~/prueba/pruebas/datos/pruebas/base/micelya-base.zip .     # deja micelya-base.sqlite3
python -c "import sqlite3; d=sqlite3.connect('micelya-base.sqlite3'); print(d.execute('pragma user_version').fetchone())"
```

- **Qué tablas hay y qué guarda cada una:** `docs/design/base-de-datos-inventario.md`. Las más usadas: `origin` y `location` (dónde está cada archivo), `file` y `content` (el documento por su huella), `document_type`, `knowledge_entity` y `knowledge_matter` (personas, organizaciones y casos), `similar_pair` («Copias y versiones») y `model_answer` (las preguntas que la app le hizo al modelo, con su respuesta).
- **Las rutas:** cada ubicación guarda su ruta relativa a su origen (`location.rel_path`); el origen guarda la carpeta de la PC del dueño (`origin.location`), que en la nube no existe. Para que el código de la app encuentre los archivos, apuntá los orígenes de tu copia a lo que bajaste, como dice `rutas` en la ficha. Con los dos orígenes de hoy:

  ```bash
  mkdir -p ~/prueba/trabajo/llm-wiki-workspace
  cp -r ~/prueba/pruebas/datos/pruebas/expedientes-pjn ~/prueba/trabajo/llm-wiki-workspace/Expedientes   # hace falta haberla bajado
  python - <<'EOF'
  import os, sqlite3
  d = sqlite3.connect(os.path.expanduser("~/prueba/trabajo/micelya-base.sqlite3"))
  d.execute("update origin set location=? where name='POC-Micelya'", (os.path.expanduser("~/prueba/poc/datos"),))
  d.execute("update origin set location=? where name='Carpeta de trabajo'", (os.path.expanduser("~/prueba/trabajo/llm-wiki-workspace"),))
  d.commit()
  EOF
  ```

  Si la ficha trae otro origen, se arma igual: la carpeta donde quedó lo que dice `en_drive`, menos la `parte`.
- **El código de la app con la copia:** las pruebas de Rust que miden sobre datos reales reciben la copia en `MICELYA_DOCBASE_COPY` (el comentario al principio de cada `check.rs` dice cómo se corren). Usá el commit de la ficha si la prueba depende de la versión de la base.
- **Las respuestas guardadas del modelo:** si la app hace una pregunta idéntica a una de `model_answer`, usa la respuesta guardada sin preguntar. **En la nube no hay Codex:** si un paso necesita una pregunta nueva, no puede seguir. Decilo en el informe; no lo reemplaces por una respuesta tuya sin decirlo.
- **La base mide, no enseña:** lo que dice la app no es una verdad. Las verdades son las hojas de respuestas, y tampoco enseñan (R-25): no ajustes umbrales mirando sus casos.

## 5. Antes de subir la rama: buscar datos reales

El repositorio es público. Con la rama commiteada y **antes del push**:

```bash
python docs/plantillas/revisar-datos-reales.py ~/prueba/pruebas/datos
```

- Compara las líneas que tu rama agrega sobre `main` con los nombres de personas, organizaciones y casos de la base, las palabras sensibles de los PDF y las hojas de respuestas. También busca números con forma de DNI o CUIT, correos, teléfonos y enlaces o ids de Drive.
- **Mirá cada coincidencia:** si es una palabra común («mundo», «inmueble»), sigue; si es un dato real, sacalo o cambialo por un ejemplo inventado, y volvé a correrlo.
- **Los ejemplos de los documentos de diseño también cuentan:** un nombre real que te dieron como ejemplo en la conversación es un dato real.
- En el informe, decí que lo corriste y con cuántas coincidencias.

## 6. Entregar

- **En el repositorio:** el informe, las métricas y los guiones, sin datos reales, en la rama que pide el prompt. Sin pedido de integración: lo revisa el agente local.
- **Lo que tiene datos reales** (textos, resultados por documento): en un .zip dentro de la carpeta del proyecto, en una carpeta que git ignore solo en ese entorno (`.git/info/exclude`), y mandado al dueño con la herramienta de envío de archivos de la sesión. Nunca en un commit. El detalle está en `CLAUDE.md`.
- **Las fichas:** informá las que usaste, por parte y en total.
