# Instrucciones para sesiones de Claude en este repositorio

Este repositorio es el fork `Micelya/llm_wiki` de `nashsu/llm_wiki` (remoto `upstream`).

## Al empezar cada sesión

1. Leer el documento más reciente de [`docs/sessions/`](docs/sessions/) (contexto, decisiones y hallazgos de la sesión anterior), el diseño vigente en [`docs/design/base-documental.md`](docs/design/base-documental.md) y [`pendientes.json`](pendientes.json).
2. Antes de cualquier otra cosa, listarle al usuario los pendientes que no estén en estado `hecho`, agrupados por `tipo`, indicando `id`, título, estado y responsable. Marcar cuáles están bloqueados por otro pendiente (`depende_de`).

## Durante el trabajo

- Mantener `pendientes.json` al día: agregar un pendiente cuando surja una tarea que no se resuelve en el momento, y actualizar `estado` y `actualizado` cuando cambie. Seguir las reglas escritas en el propio archivo.
- Toda decisión de producto, de diseño o de forma de trabajo se anota en [`docs/decisiones.md`](docs/decisiones.md), siguiendo las reglas escritas ahí: primero el problema, después la decisión, sin dar nada por sabido. Una decisión que cambia no se borra: se agrega una entrada nueva y la vieja queda marcada como reemplazada.
- Todo análisis de documentos (pasadas, cálculos, usos del modelo) respeta las reglas de [`docs/design/reglas-de-procesamiento.md`](docs/design/reglas-de-procesamiento.md). Si una prueba muestra que una regla no funciona, se propone el cambio al dueño del producto; no se ignora la regla.
- Las pruebas se corren a mano antes de cada PR: `npm run test:mocks` y, en `src-tauri`, `cargo test --lib`. No se espera ningún fallo.
- Los PR se abren contra este fork: `gh pr create --repo Micelya/llm_wiki --base main`. Sin esos parámetros, `gh` apunta al proyecto original.

## Uso de la inteligencia artificial

- **La IA se usa donde agrega valor o donde el proceso la necesita.** No hay ninguna regla que pida evitarla, limitarla ni minimizarla.
- **Lo determinista se prefiere solo cuando logra la misma eficacia o una mejor, con la misma eficiencia o una mejor.** Si una herramienta determinista da un resultado peor que la IA, se usa la IA.
- **Si el proceso requiere la IA, o la IA logra un resultado superior, se usa.** Ahorrar cupo, el costo del modelo o "agregar otro uso del modelo" no son argumentos para elegir un camino peor.
- Antes de una prueba que consuma cupo, se le avisa al dueño del producto. Avisar no es lo mismo que evitar.

## Upstream

- **No vamos a sincronizar con upstream** (`nashsu/llm_wiki`). El proyecto diverge de forma definitiva: lo que haga upstream ya no nos interesa.
- **La compatibilidad con upstream no es un criterio.** El código se cambia según nuestros requerimientos, en cualquier archivo y en la medida que haga falta. Cuánto nos alejemos de upstream no es algo que importe ni preocupe.
- No se proponen contribuciones a upstream.

## Estimaciones de esfuerzo

- **Nunca estimar en horas, días ni ninguna otra unidad de tiempo.** Esas estimaciones salen de cómo trabajan las personas, no de cómo trabaja un agente, y por eso son siempre erróneas.
- **La única estimación válida es relativa:** cuánto esfuerzo lleva una tarea comparada con otra (por ejemplo, "más chica que", "parecida a", "bastante más grande que"). Sirve para ordenar y elegir, no para planificar en el calendario.
- **El esfuerzo no es motivo para postergar ni para descartar una opción.** Si algo hay que hacerlo, se planifica cuándo y cómo; no se usa "es trabajo adicional" como argumento en contra.
