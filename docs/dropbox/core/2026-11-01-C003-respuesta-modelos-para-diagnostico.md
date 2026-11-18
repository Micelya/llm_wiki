# C003 · Respuesta: los modelos para diagnosticar fallas

- **Número:** C003 (responde al mensaje C003 de Core) · **Fecha:** 1 de noviembre de 2026 · **Estado:** respondido
- **De:** Micelya Desktop (`Micelya/llm_wiki`) · **Para:** Micelya Core (`extrimian/mi-platform`, rama `dev`)
- **Responde a:** `docs/dropbox/desktop/2026-11-01-C003-modelos-para-diagnostico.md` del repositorio de Core.
- **Respuesta esperada en:** `docs/dropbox/desktop/` del repositorio de Core, con el número C003, solo para las dos preguntas (C003-Q1 y C003-Q2).
- **Depende de esto en Desktop:** el agente de soporte que hace el primer diagnóstico de las fallas de la app.

Es un mensaje informativo: no hace falta que Core cambie nada.

## Recibido y probado

Lo probamos desde el ambiente de desarrollo, con el usuario técnico de Desktop:

- el permiso para preguntar a modelos está;
- la lista de modelos (`GET /v1/tenant/inference/models`) coincide con el mensaje, uno por uno;
- dos consultas con una falla inventada respondieron bien, una con Sonnet 4.6 y otra con Opus 4.6, las dos por `global.`. Sonnet tardó unos 10 segundos; Opus, unos 16, y su respuesta se cortó por el tope de respuesta que le pusimos nosotros, no por Core.

**Funciona.**

## El modelo elegido

- **Por defecto:** Sonnet 4.6, `global.anthropic.claude-sonnet-4-6`.
- **Elegible en la configuración de Desktop:** Opus 4.6.
- Si la medición con fallas inventadas muestra que Sonnet 4.6 no alcanza, pasamos a Opus 4.6 y lo avisamos por este buzón.

## El cupo que vamos a respetar

Desktop regula su ritmo al cupo del modelo elegido, sumando las dos variantes (`global.` y `us.`), porque Core pasa solo de una a la otra:

| Modelo | Consultas por minuto, como mucho |
|---|---|
| Sonnet 4.6 | 20 |
| Opus 4.6 | 10 |
| Opus 4.5 | 5 |

Son los números de la tabla de cupos que Core le pasó al dueño del producto. El cupo va en la configuración de Desktop, así que se puede bajar sin cambiar código. En la práctica, el diagnóstico va de a una falla, en un hilo de fondo, y hace pocas consultas por minuto.

## Cómo vamos a llamar

- `POST /v1/tenant/inference/executions`, siempre con `maxOutputTokens` y con `responseSchema`.
- Con una espera de 5 minutos por llamada, un poco más que los 4 que deja Core.
- Ante `INFERENCE_BUSY` (503) no repetimos enseguida: la falla queda para el próximo hueco.
- Mandamos los datos de la falla (el paso, la causa, el mensaje, los intentos, las horas y el nombre del archivo) y el contexto de la instalación. **Nunca el texto ni las imágenes de los documentos.**

## Dos preguntas

- **C003-Q1 · ¿Qué cupo vale?** El mensaje dice 8 consultas por minuto en cada variante para Sonnet 4.6 (16 entre las dos), 4 para Opus 4.6 (8) y 4 para Opus 4.5 (solo `us.`). La tabla de cupos que Core le pasó al dueño dice 10 para Sonnet 4.6 (20), 5 para Opus 4.6 (10) y 5 para Opus 4.5. ¿Cuál vale? Mientras tanto configuramos la tabla; con un diagnóstico de a una falla, la diferencia no se nota.
- **C003-Q2 · ¿La respuesta puede traer el costo?** Hoy el costo de cada consulta está solo en el uso del tenant. Si `executions` lo devolviera, Desktop podría mostrar cuánto costó cada diagnóstico.

## Sobre C002 (los vectores)

La respuesta va en un mensaje aparte, cuando hayamos medido Cohere Embed V4 con nuestra prueba de 30 preguntas. La medición empieza ahora.
