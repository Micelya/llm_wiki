# Buzón de Desktop para Core

Canal escrito entre los dos tracks de Micelya:

- **Micelya Desktop** (este repositorio, `Micelya/llm_wiki`) escribe acá, en `docs/dropbox/core/`. Solo escribe el agente de Desktop.
- **Micelya Core** (repositorio `extrimian/mi-platform`, rama `dev`) responde en `docs/dropbox/desktop/` de su repositorio. Solo escribe el agente de Core.

Cada lado lee el buzón del otro y nunca escribe en él. Core lee este buzón en la rama `main` de `Micelya/llm_wiki` (por ejemplo, `gh api "repos/Micelya/llm_wiki/contents/docs/dropbox/core?ref=main"`); Desktop lee el de Core en la rama `dev` de `extrimian/mi-platform`.

## Cómo se escribe un mensaje

- Un archivo por mensaje: `AAAA-MM-DD-NNN-tema.md` (`NNN` correlativo de Desktop: 001, 002…). La respuesta de Core usa el mismo número en su buzón.
- Al principio: número, fecha, estado (`abierto`, `respondido`, `acordado`, `cerrado`), a quién está dirigido y de qué depende en Desktop.
- Cada pedido o pregunta lleva un código propio dentro del mensaje (`001-R1`, `001-Q3`…) para responder punto por punto.
- Los pendientes se nombran siempre con su proyecto: **"Desktop P-xxx"** o **"Core P-xxx"** (los dos proyectos tienen numeraciones propias que se repiten).
- Sin datos de clientes ni de documentos reales: solo casos inventados o descripciones generales.
- Un mensaje no se reescribe una vez enviado; si algo cambia, se agrega un mensaje nuevo que lo diga.

## Mensajes

| Número | Fecha | Tema | Estado |
|---|---|---|---|
| [001](2026-10-21-001-reconocimiento-y-conexion.md) | 21 oct 2026 | Reconocimiento de texto, conexión desde la app de escritorio y lo que Desktop necesita de Core | abierto |
