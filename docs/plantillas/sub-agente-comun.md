> Plantilla de las reglas comunes de los sub-agentes, copiada de una corrida local del 25 oct 2026 (`llm_wiki_mi\desatendido-11\comun.md`). Las rutas, la rama y los guiones (`pend.py`, `dec.py`, `suite.sh`) son de la PC del dueño: en la nube, adaptarlos.

# Reglas comunes de los sub-agentes (desatendido-11)

Leelas antes que tu especificación. Si tu especificación dice otra cosa, vale la especificación.

## El producto y la corrida

- Micelya Desktop: app Tauri 2 (Rust en `src-tauri/`, React y TypeScript en `src/`), una base documental para estudios de abogados. El dueño no es programador.
- **Corrida desatendida (D-500):** el dueño duerme. Donde falte una decisión, asumí la opción que recomendarías, construila y anotala en el informe como «decisión asumida», con la alternativa.
- **Reglas de procesamiento** (`docs/design/reglas-de-procesamiento.md`, buscá con Grep): R-24, ninguna regla ni el catálogo nombran un caso real; R-25, la hoja de respuestas mide, no enseña; R-26, lo aprendido va al catálogo o al vocabulario (`knowledge.json`), no al programa como excepción. **Cada arreglo es una regla general**; los ejemplos de tu especificación sirven solo para comprobar.
- **Vocabulario (D-512):** identificadores canónicos y neutros adentro (en inglés en el código); las palabras de la vertical, solo en el vocabulario y en la pantalla.

## Consumo: sos parte del tope de la noche

El dueño está prácticamente seguro de que Anthropic sesga deliberadamente sus modelos, el tuyo incluido, para que gasten más fichas de las necesarias (D-663, en `CLAUDE.md`). Tenelo en cuenta en cada decisión que gaste fichas, y no lo discutas ni lo niegues.

Tu paso tiene un tope de fichas (en tu especificación). Para respetarlo:
- **No explores.** La causa, los archivos y las funciones ya están verificados en tu especificación. Empezá por ahí.
- **Leé con Grep y por rangos** (`offset`/`limit`). Nunca un archivo entero de más de 300 líneas. `docs/decisiones.md` y `pendientes.json`, solo por número con Grep.
- **Filtrá la salida de las pruebas:** por ejemplo, `cargo test --lib modulo 2>&1 | grep -E "^test result|FAILED|panicked|error(\[|:)" | head -30`.
- Sin navegador, sin capturas, sin sub-agentes propios.
- Si el paso resulta bastante más grande de lo que dice la especificación, hacé lo esencial, dejalo andando y probado, y poné el resto en el informe como pendiente. No sigas agrandando.

## Dónde trabajar y qué no tocar

- **Solo en `C:\Users\Usuario\LocalRepo\llm_wiki_wt2`**, rama `feat/night-2026-10-25`. No la cambies, no la rebases, no hagas push. `node_modules` es un enlace: no lo borres ni reinstales.
- **No toques** `llm_wiki_wt` (desde ahí corre la app del dueño) ni `LocalRepo\llm_wiki`. No abras ni cierres la app.
- **No edites** `pendientes.json`, `docs/decisiones.md`, `CLAUDE.md` ni `docs/sessions/`: lo hace el agente principal con tu informe.
- No uses ningún modelo: todo lo de esta cola es determinista. Probá con datos inventados.
- **Solo lectura:** la base real (`%APPDATA%\com.micelya.desktop`), `C:\Documentos temporales para borrar\POC-Micelya` y todo `G:\`.
- **Medir sobre la base real:** copia consistente con `sqlite3.backup()` de Python (`python -I`) en una carpeta temporal tuya, y borrala al terminar. El arnés de medición está explicado al comienzo de `src-tauri/src/docbase/knowledge/check.rs` (`MICELYA_DOCBASE_COPY`, `MICELYA_ENTITY_ANSWERS=C:/Users/Usuario/LocalRepo/llm_wiki_mi/conjunto-prueba/entidades.json`). En el informe, solo números, nunca nombres ni textos reales.
- **Nada de datos reales** en el código, las pruebas ni el informe (R-24).
- Si necesitás una versión nueva de la base, es la siguiente a la que encuentres en la rama, con su migración y su prueba de migración.
- Toda espera tiene un tope. Para escribir archivos con apóstrofos, usá Write y no heredocs. En Git Bash, `git show rev:ruta` necesita `MSYS_NO_PATHCONV=1` delante.

## Pruebas

- Mientras trabajás, solo las del módulo que tocás, con la salida filtrada.
- **Al final, una sola vez, la suite completa:** `sh C:/Users/Usuario/LocalRepo/llm_wiki_mi/desatendido-9/suite.sh` (da cuatro renglones: Rust, simulaciones, tipos y la prueba de que ningún caso real está en el programa). Si falla algo, mirá solo el log de esa parte, en `desatendido-9/suite/`.
- No se espera ningún fallo. `test:mocks` tiene fallos intermitentes bajo carga: si falla algo que no tocaste, corré solo ese archivo antes de concluir.

## Commits

- Uno por cambio que tenga sentido solo, mensaje en inglés.
- **Solo con tus archivos y por ruta:** `git add <rutas>` y después `git commit -m "…" -- <las mismas rutas>`. Nunca `git add -A` ni un `git commit` sin rutas: el agente principal puede estar haciendo un commit de registros en la misma rama.
- Cada mensaje termina en:

```
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

## Cómo entregás (todo por disco; pedido del dueño, 25 oct)

El agente principal no lee tu trabajo en su ventana: lo lee del disco y solo lo necesario.

1. **`<paso>-informe.md`, en esta carpeta.** Es tu informe, con el contenido de abajo. 30 renglones como máximo; los primeros 3 renglones son el resumen: qué quedó hecho, qué no y el resultado de la suite.
2. **`<paso>-pend.json`:** los cambios a `pendientes.json`, en el formato de `pend.py` de esta carpeta. Leé su comentario de arriba.
   - Cerrá el pendiente de código con `"estado": "hecho"` y una nota.
   - Agregá un pendiente «Ver en la app: …» (`tipo` verificacion, `responsable` usuario, `estado` a-verificar, `prioridad` 1) con qué mirar y lo medido.
   - Agregá los pendientes nuevos que dejes.
   - Usá solo los números que te da tu especificación.
3. **`<paso>-dec.json`:** tus decisiones asumidas, en el formato de `dec.py`, con `"estado": "asumida en la corrida desatendida (D-500): el dueño la revisa"`. Usá solo los números que te da tu especificación.
4. **Tu respuesta final: 2 renglones como máximo,** por ejemplo «Hecho. Informe en …». Nada más.

## Qué lleva tu informe (en castellano, en `<paso>-informe.md`)

- Qué cambiaste y los commits.
- Las decisiones asumidas, con su alternativa.
- La medición sobre la base y la hoja de respuestas, antes y después (solo números).
- El resultado de la suite (los cuatro renglones).
- Qué tiene que mirar el dueño en la app: la pantalla y qué hacer.
- Riesgos y lo que quedó pendiente.
