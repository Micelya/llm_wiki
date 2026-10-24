# Retiro del wiki heredado

**Estado:** decidido por el dueño el 21 de octubre de 2026 (D-395 a D-400) · construido e integrado el 21 de octubre (PR #60); la carpeta del wiki se borró después del respaldo · falta que el dueño lo vea en el uso diario (P-228) · el recortador web volvió el 22 de octubre, sobre la base (D-450, D-451), y la API local con el MCP el 23, en un solo servidor dentro de la máquina (D-509) · puesto al día el 23 de octubre de 2026
**Alcance:** qué sale, dónde queda cada contenido y en qué etapas se hace. No describe código.

## 1. Por qué

Hoy conviven dos estructuras. Por un lado la **base documental**, con sus pantallas: Inicio, Atención, Documentos, Personas y organizaciones, Asuntos y Orígenes. Por otro el **wiki heredado**: una carpeta de archivos de texto con páginas, sus pantallas (Wiki, Sources, Lint, Review) y los proyectos de wiki.

Desde la [propuesta del wiki sobre la base](wiki-sobre-la-base-propuesta.md) casi todo lo que muestra el wiki ya sale de la base: los archivos son una copia que se desactualiza (P-221). Tener las dos cosas repite información, confunde al usuario ("¿miro el wiki o la pantalla?") y ata la base a una carpeta.

## 2. Lo que se decidió

1. **Proyectos de wiki (D-395).** Se dejan de admitir los proyectos de wiki originales. La app abre en Inicio, sin elegir proyecto. Volver a tenerlos, con un propósito definido, queda como P-225.
2. **Investigación en internet (D-397).** Queda, adaptada: se lanza desde una persona, organización o asunto; el resultado se guarda **en la base**, asociado a esa ficha, con sus fuentes y separado de lo que dicen los documentos. El panel general sigue para investigaciones sueltas, también guardadas en la base.
3. **Archivos del wiki (D-396).** Desaparecen. Lo que no está repetido pasa a la base: notas de una persona, consultas guardadas, historial del chat y resumen general. La carpeta del wiki se borra al final, después de migrar y respaldar.
4. **Grafo (D-398).** El grafo, su relevancia y su análisis (comunidades, puntos clave) se calculan sobre la base. El grafo suma los asuntos con su estado.
5. **Lint (D-399).** Sale. La revisión con el modelo que agregue valor en la estructura nueva quedó decidida el 22 de octubre: a pedido, busca contradicciones entre los documentos de una ficha o un caso y las lleva a Atención (D-476); sin construir (P-226).
6. **API local y recortador web (D-400).** Salen con el wiki. El recortador volvió el 22 de octubre sobre la base: la extensión «Micelya Clipper» guarda cada página como archivo en «Recortes web», dentro de la carpeta de trabajo, que es un origen de la base (D-450, D-451). La API local y el MCP volvieron el 23, sobre el [contrato de operaciones](contrato-de-operaciones.md), en el mismo servidor que el recortador: los otros programas leen la base y piden cambios que una persona confirma (D-504, D-509). Falta que el dueño lo vea (P-227).

## 3. Qué había en el wiki y dónde queda

| En el wiki | Dónde queda |
|---|---|
| Página de un documento | Detalle del documento, en Documentos |
| Página de una persona u organización | Su página, con las pestañas «Artículo» y «Datos y correcciones» |
| Página de un asunto | Su página, en Casos (el nombre del asunto en la vertical de abogados) |
| Notas de una persona | En la base; se editan en cada pantalla |
| Consultas guardadas | En la base; lista en el chat |
| Historial del chat | En los datos de la app, migrado una sola vez |
| Resumen general e índice | Inicio |
| Investigación en internet | En la base; sección en la página de cada ficha |
| Sources | Orígenes, Documentos y Atención |
| Review y Lint | Atención; la revisión con el modelo, decidida y sin construir (D-476, P-226) |
| Grafo | Sobre la base, con los asuntos y su estado; un clic abre la pantalla |
| Búsqueda | Sobre la base; abre las pantallas |
| Proyectos de wiki | Salen (P-225) |
| API local y recortador web (Clip) | El recortador volvió sobre la base: «Recortes web» de la carpeta de trabajo (D-450). La API local y el MCP, sobre el contrato de operaciones, en el mismo servidor (D-509) |

## 4. Cómo se hace sin preguntar de nuevo al modelo

El código que hoy arma cada página del wiki sigue existiendo, pero **escribe en la base** en lugar de en archivos. Cada página conserva su mismo nombre (su ruta) como identificador. Así:

- no se le pregunta nada nuevo al modelo: lo ya leído y sintetizado se reutiliza;
- los enlaces entre páginas siguen funcionando y llevan a la pantalla que corresponde;
- pantallas, búsqueda, chat y grafo leen todos de la base.

## 5. Etapas

Todas hechas el 21 de octubre e integradas en la versión principal (PR #60). Falta que el dueño lo vea en el uso diario (P-228).

| Etapa | Qué se hace | Estado |
|---|---|---|
| W1 | Este documento y los registros (decisiones y pendientes). | Hecha |
| W2 | La base sin la carpeta: las páginas y el resumen se guardan en la base; notas y consultas guardadas se importan una vez; la lectura automática deja de depender del wiki. | Hecha |
| W3 | Pantallas: artículo y notas en persona u organización, documento y asunto; resumen en Inicio; enlaces y búsqueda abren pantallas. | Hecha |
| W4 | Grafo sobre la base: asuntos con estado y confirmación, etiquetas en las relaciones, relevancia y análisis sobre la base. | Hecha |
| W5 | Chat e investigación: el chat busca, lee y guarda en la base; el historial se migra; la investigación se lanza desde cada ficha y se guarda en la base. | Hecha |
| W6 | Retirar lo heredado: Wiki, Sources, Lint, Review, editor de archivos, proyectos, incorporación original, API y Clip. La app abre en Inicio; «Activar wiki» pasa a «Lectura automática» en Orígenes. | Hecha (el recortador volvió el 22 oct sobre la base; la API local y el MCP, el 23) |
| W7 | Prueba con los datos reales: respaldo con huellas, comprobación de que no hay preguntas nuevas al modelo, borrado de la carpeta del wiki y pedido de integración. | Hecha |

Antes de borrar la carpeta se comprobó que ningún origen de la base apuntaba dentro de ella, y el borrado se hizo con un guion que verifica el respaldo y no sigue enlaces.
