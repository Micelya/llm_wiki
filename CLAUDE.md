# Instrucciones para sesiones de Claude en este repositorio

Este repositorio es el fork `Micelya/llm_wiki` de `nashsu/llm_wiki` (remoto `upstream`).

## Al empezar cada sesión

1. Leer [`pendientes.json`](pendientes.json).
2. Antes de cualquier otra cosa, listarle al usuario los pendientes que no estén en estado `hecho`, agrupados por `tipo`, indicando `id`, título, estado y responsable. Marcar cuáles están bloqueados por otro pendiente (`depende_de`).

## Durante el trabajo

- Mantener `pendientes.json` al día: agregar un pendiente cuando surja una tarea que no se resuelve en el momento, y actualizar `estado` y `actualizado` cuando cambie. Seguir las reglas escritas en el propio archivo.
- Todo cambio que este fork haga respecto de upstream se anota en [`FORK_CHANGES.md`](FORK_CHANGES.md), siguiendo las reglas escritas ahí.
- Los PR se abren contra este fork: `gh pr create --repo Micelya/llm_wiki --base main`. Sin esos parámetros, `gh` apunta al proyecto original.
