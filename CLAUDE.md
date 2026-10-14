# Instrucciones para sesiones de Claude en este repositorio

Este repositorio es el fork `Micelya/llm_wiki` de `nashsu/llm_wiki` (remoto `upstream`).

## Al empezar cada sesión

1. Leer el documento más reciente de [`docs/sessions/`](docs/sessions/) (contexto, decisiones y hallazgos de la sesión anterior), el diseño vigente en [`docs/design/base-documental.md`](docs/design/base-documental.md) y [`pendientes.json`](pendientes.json).
2. Antes de cualquier otra cosa, listarle al usuario los pendientes que no estén en estado `hecho`, agrupados por `tipo`, indicando `id`, título, estado y responsable. Marcar cuáles están bloqueados por otro pendiente (`depende_de`).

## Durante el trabajo

- Mantener `pendientes.json` al día: agregar un pendiente cuando surja una tarea que no se resuelve en el momento, y actualizar `estado` y `actualizado` cuando cambie. Seguir las reglas escritas en el propio archivo.
- Todo cambio que este fork haga respecto de upstream se anota en [`FORK_CHANGES.md`](FORK_CHANGES.md), siguiendo las reglas escritas ahí.
- Toda decisión de producto, de diseño o de forma de trabajo se anota en [`docs/decisiones.md`](docs/decisiones.md), siguiendo las reglas escritas ahí: primero el problema, después la decisión, sin dar nada por sabido. Una decisión que cambia no se borra: se agrega una entrada nueva y la vieja queda marcada como reemplazada.
- Todo análisis de documentos (pasadas, cálculos, usos del modelo) respeta las reglas de [`docs/design/reglas-de-procesamiento.md`](docs/design/reglas-de-procesamiento.md). Si una prueba muestra que una regla no funciona, se propone el cambio al dueño del producto; no se ignora la regla.
- Las pruebas se corren a mano antes de cada PR: `npm run test:mocks` y, en `src-tauri`, `cargo test --lib`. No se espera ningún fallo.
- Los PR se abren contra este fork: `gh pr create --repo Micelya/llm_wiki --base main`. Sin esos parámetros, `gh` apunta al proyecto original.
