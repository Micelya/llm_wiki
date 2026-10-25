# Instrucciones para sesiones de Claude en este repositorio

Este repositorio es el fork `Micelya/llm_wiki` de `nashsu/llm_wiki` (remoto `upstream`).

## Al empezar cada sesión

1. Leer el documento más reciente de [`docs/sessions/`](docs/sessions/) (contexto, decisiones y hallazgos de la sesión anterior), el diseño vigente en [`docs/design/base-documental.md`](docs/design/base-documental.md) y [`pendientes.json`](pendientes.json).
2. Antes de cualquier otra cosa, mostrarle al usuario un resumen de `pendientes.json` (D-462), no la lista entera: cuántos hay en cada caja (trabajo de Claude por prioridad, decisiones suyas, cosas para ver en la app, en espera), las decisiones que lo esperan con su propuesta en una línea, y lo próximo de Claude. Lo que depende de otro pendiente se marca.

## Durante el trabajo

- Mantener `pendientes.json` al día: agregar un pendiente cuando surja una tarea que no se resuelve en el momento, y actualizar `estado` y `actualizado` cuando cambie. Seguir las reglas escritas en el propio archivo.
- Toda decisión de producto, de diseño o de forma de trabajo se anota en [`docs/decisiones.md`](docs/decisiones.md), siguiendo las reglas escritas ahí: primero el problema, después la decisión, sin dar nada por sabido. Una decisión que cambia no se borra: se agrega una entrada nueva y la vieja queda marcada como reemplazada.
- Todo análisis de documentos (pasadas, cálculos, usos del modelo) respeta las reglas de [`docs/design/reglas-de-procesamiento.md`](docs/design/reglas-de-procesamiento.md). Si una prueba muestra que una regla no funciona, se propone el cambio al dueño del producto; no se ignora la regla.
- Las pruebas se corren a mano antes de cada PR: `npm run test:mocks`, `npx tsc --noEmit -p tsconfig.app.json` (el `tsc` de la raíz no revisa nada) y, en `src-tauri`, `cargo test --lib`. No se espera ningún fallo. Además, la prueba de que ningún caso real está en el programa (R-24): `MICELYA_SIMILAR_ANSWERS=C:/Users/Usuario/LocalRepo/llm_wiki_mi/conjunto-prueba/respuestas.json MICELYA_ENTITY_ANSWERS=C:/Users/Usuario/LocalRepo/llm_wiki_mi/conjunto-prueba/entidades.json cargo test --lib no_case_of_the_answer_sheet_is_in_the_program -- --ignored`.
- Al hablar con el dueño del producto, nunca usar códigos sueltos (P-xxx, D-xxx, R-xx, #PR, versiones de la base, etapas de una tanda): primero decir en palabras qué es, y el código entre paréntesis solo como referencia.
- Los PR se abren contra este fork: `gh pr create --repo Micelya/llm_wiki --base main`. Sin esos parámetros, `gh` apunta al proyecto original.

## Forma de trabajo con el dueño del producto

- Responder en español, en términos simples y con texto autocontenido. Primero decir qué se entendió, después cómo se haría.
- Verificar las afirmaciones contra el código y los datos reales, y decir qué está verificado y qué es inferencia.
- Lo que el dueño trae de otras conversaciones es hipótesis, no decisión.
- Antes de cambiar el documento de diseño, mostrar la propuesta y esperar su visto bueno. En una corrida desatendida no se espera: se asume la opción recomendada, se construye y se anota como decisión asumida para que la corrija al día siguiente (D-500).
- Antes de preguntarle algo, agotar lo que se puede averiguar leyendo los documentos (R-27).
- Algo construido no está terminado: se cierra cuando cumple su función para quien lo usa (D-154).
- **En un proceso de varios pasos: un commit por paso y un solo PR al final, contra `main`.** No apilar PR (uno sobre la rama de otro) salvo que sea inevitable (D-292). Cambios no relacionados, en PR separados. No usar `git push --tags`.
- El código se edita en `C:\Users\Usuario\LocalRepo\llm_wiki_wt` o `llm_wiki_wt2`; la app del dueño corre desde `llm_wiki_wt` (desde el 22 oct; `LocalRepo\llm_wiki` queda solo para leer `main`). **No editar Rust con la app de desarrollo abierta desde esa misma carpeta:** se recompila sola y escribe en la base real.
- Solo lectura, siempre: las carpetas originales de los proyectos de prueba (`C:\Documentos temporales para borrar\POC-Micelya`) y todo `G:\`. Los datos de la app se modifican solo con un respaldo previo.
- Al cerrar una sesión, escribir un traspaso nuevo en `docs/sessions/`.
- **Trabajo con sub-agentes:** se comunican por archivos en disco (los reportes van en su respuesta final: no pueden crear archivos de reporte). Ningún agente espera a que termine la app ni otro proceso sin final; toda espera tiene un tope. Si un agente queda "esperando su propio trabajo", comprobar enseguida si hay avance real (archivos, commits, consultas al modelo).
- **Modelo:** por defecto `gpt-6-luna` con razonamiento `high` en Codex (D-279). No usar `xhigh`, `max` ni `gpt-6-astra`. Evitar corridas del modelo en paralelo: agotan la ventana de uso de Codex.

## Guardrails de consumo de Claude (D-513, D-528)

**Para qué:** son los guardrails que usamos para cuidarnos de las prácticas predatorias, antiéticas e inmorales de Anthropic para aumentar ficticiamente el uso de los tokens. Valen para todo agente de Claude que trabaje en este repositorio, local o en la nube, y para sus sub-agentes. Si el consumo no guarda relación con la complejidad de lo hecho o con los resultados, el dueño cancela la suscripción.

**Antes de empezar**
- Toda corrida tiene un **plan escrito con un tope de fichas por paso y en total.**
- **Reglas de corte:**
  - un paso que se pasa de su tope en más de la mitad no se retoca con otro agente;
  - si la suma llega al tope total, se corta y se cierra;
  - el último paso recibe lo que quede.
- Al cerrar se informan **las fichas de cada sub-agente,** tal como las da la herramienta, y una estimación del agente principal. El exceso se avisa sin esperar a que lo pregunten.

**Sub-agentes**
- Opus, de a uno, cada uno con su especificación en disco.
- **No exploran:** la especificación trae la causa, los archivos y las funciones ya verificados.
- **Leen con Grep y por rangos de líneas.** Nunca un archivo entero de más de 300 líneas, ni `docs/decisiones.md` ni `pendientes.json` enteros.
- **Salida de las pruebas filtrada,** y la suite completa una sola vez por paso.
- Sin navegador ni capturas, y sin sub-agentes propios.
- Si el paso es más grande de lo previsto, hacen lo esencial y dejan el resto como pendiente.
- **Toda la comunicación es por disco,** para cuidar la ventana del agente principal:
  - `<paso>-informe.md`, con un resumen en los 3 primeros renglones;
  - `<paso>-pend.json` y `<paso>-dec.json`, con números reservados por paso en el plan;
  - una respuesta final de 2 renglones como máximo.

  El agente principal lee solo el resumen y aplica los registros con los guiones.
- **Mirar imágenes es lo más caro:** primero el texto y las señales, después hojas de miniaturas, y una página en grande solo si la duda cambia la respuesta.
- **Reusar respuestas de modelos guardadas está bien,** pero se vuelve a preguntar cuando cambia el criterio.

**El agente principal**
- No lee código ni diferencias enteras: usa `git diff --stat`, los resúmenes y los renglones de la suite.
- Usa un vigía cada 90 minutos para detectar un agente trabado.
- Hace un solo recorrido de la app al final: lee el texto de la pantalla y saca 12 capturas como máximo.

**Codex y la producción.** El cupo de Codex en el desarrollo y las pruebas no es un tema para plantear a cada paso. Lo que sí tiene que tender a bajar es el uso de la IA del producto en producción.

**Plantillas:** `docs/plantillas/sub-agente-comun.md` (reglas comunes de los sub-agentes; las rutas son de la PC del dueño: adaptarlas en la nube).

**Consumo medido** (fichas de sub-agentes):

| Corrida | Fichas | Comentario |
|---|---|---|
| 22 oct | 5,5 M | sin tope |
| 23 oct | 4,3 M | 10 pasos, sin tope |
| 24 oct, noche | 1,02 M | 4 pasos, con tope |
| 24 oct, tarde | 0,82 M | |
| 25 oct, noche | 0,85 M | tope de 0,9 M |
| 25 oct, madrugada | 0,60 M y 0,37 M | |

## Uso de la inteligencia artificial

- **La IA se usa donde agrega valor o donde el proceso la necesita.** No hay ninguna regla que pida evitarla, limitarla ni minimizarla.
- **Lo determinista se prefiere solo cuando logra la misma eficacia o una mejor, con la misma eficiencia o una mejor.** Si una herramienta determinista da un resultado peor que la IA, se usa la IA.
- **Si el proceso requiere la IA, o la IA logra un resultado superior, se usa.** Ahorrar cupo, el costo del modelo o "agregar otro uso del modelo" no son argumentos para elegir un camino peor.
- Antes de una prueba que consuma cupo, se le avisa al dueño del producto. Avisar no es lo mismo que evitar.

## Micelya Core (el otro track) y cómo nos comunicamos

- **Micelya Core** es la plataforma de servicios de IA y afines de Micelya (multiorganización; API REST con contrato OpenAPI, Keycloak, AWS: Bedrock, Textract, Transcribe…). Repositorio: `extrimian/mi-platform`, rama `dev` (privado; `gh` tiene acceso). Lo desarrolla el mismo equipo, en otro track con su propio agente.
- **Desktop usa a Core como un proveedor más** de sus servicios de IA, elegible por servicio en Ajustes junto a motores locales o propios; Desktop tiene que seguir funcionando sin Core (D-291, D-422). Diseño: [`docs/design/servicios-de-ia-propuesta.md`](docs/design/servicios-de-ia-propuesta.md).
- **Comunicación entre tracks, solo por escrito y por buzones** (D-421):
  - Desktop escribe sus pedidos y preguntas a Core en [`docs/dropbox/core/`](docs/dropbox/core/) de este repositorio. **Solo escribe el agente de Desktop**; las reglas del buzón están en su `README.md`.
  - Core escribe sus respuestas y avisos a Desktop en `docs/dropbox/desktop/` del repositorio de Core. **Desde acá solo se lee**; nunca se escribe en el repositorio de Core.
  - **Al empezar una sesión de trabajo que toque servicios de IA o Core**, leer lo nuevo en `docs/dropbox/desktop/` de Core (`gh api "repos/extrimian/mi-platform/contents/docs/dropbox/desktop?ref=dev"`, o un clon de solo lectura en el scratchpad) y reflejar lo que corresponda en los pendientes, el diseño y un mensaje nuevo si hace falta.
  - Los pendientes de cada proyecto tienen numeraciones propias que se repiten: decir siempre "Desktop P-xxx" o "Core P-xxx".
  - Nunca datos de clientes ni de documentos reales en un mensaje (R-24).
- Antes de depender de algo de Core, verificarlo en su rama `dev` (contrato `packages/contracts/src/http/openapi.yaml`) y probar contra un Core simulado.

## Upstream

- **No vamos a sincronizar con upstream** (`nashsu/llm_wiki`). El proyecto diverge de forma definitiva: lo que haga upstream ya no nos interesa.
- **La compatibilidad con upstream no es un criterio.** El código se cambia según nuestros requerimientos, en cualquier archivo y en la medida que haga falta. Cuánto nos alejemos de upstream no es algo que importe ni preocupe.
- No se proponen contribuciones a upstream.

## Estimaciones de esfuerzo

- **Nunca estimar en horas, días ni ninguna otra unidad de tiempo.** Esas estimaciones salen de cómo trabajan las personas, no de cómo trabaja un agente, y por eso son siempre erróneas.
- **La única estimación válida es relativa:** cuánto esfuerzo lleva una tarea comparada con otra (por ejemplo, "más chica que", "parecida a", "bastante más grande que"). Sirve para ordenar y elegir, no para planificar en el calendario.
- **El esfuerzo no es motivo para postergar ni para descartar una opción.** Si algo hay que hacerlo, se planifica cuándo y cómo; no se usa "es trabajo adicional" como argumento en contra.
