# Asuntos: un solo concepto — propuesta

**Estado:** aprobada por el dueño el 20 oct 2026 (D-373 a D-376). Reemplaza lo que decían sobre asuntos la [base documental](base-documental.md) y la [propuesta del wiki](wiki-sobre-la-base-propuesta.md) (sección 5, D-293 y D-294).

Un **asunto** es lo que en un estudio se llama un caso: un reclamo con sus cartas y facturas, un contrato con sus pagos. En la vertical de abogados la pantalla dice "Caso"; el término genérico es "Asunto" y sale del vocabulario, nunca del código.

## 1. El problema

Hoy la app tiene **tres mecanismos** que hablan de asuntos y no se conocen entre sí:

1. **Entidades de tipo asunto.** La lectura de cada documento anota lo que el documento nombra como asunto ("el expediente tal", "el contrato de X"), y eso se guarda como una entidad más, junto a personas y organizaciones. En la base real hay **85**, ninguna con algo que decir de ella. Por eso Personas y organizaciones se llena de fichas vacías.
2. **Grupos automáticos.** Una regla junta documentos que comparten evidencia y dice "un grupo de dos documentos o más es un asunto". Pero cuenta copias y versiones como documentos distintos: de **7 grupos**, **4** son un solo documento con sus versiones o copias. Solo **3** juntan documentos distintos (un reclamo con 26 documentos; un acuerdo de terminación con sus recibos; un acuerdo de colaboración con su nota de rescisión).
3. **Etiquetas "Caso".** Una persona puede etiquetar un grupo, a mano o con una regla de carpeta ("cada subcarpeta de acá es un caso").

El resultado: tres listas distintas, ninguna confiable, y el usuario no sabe cuál es "el" asunto.

## 2. Lo acordado (20 oct)

1. **Un solo concepto de asunto.** "Asunto" en general; "Caso" en la vertical de abogados.
2. **Nace de dos maneras:** lo crea una persona, o lo propone el sistema. **No** por reglas de carpeta.
3. **Lo que propone el sistema es una propuesta** hasta que una persona la confirma en Atención. Mientras tanto se ve como "propuesto".
4. **Un documento puede estar en varios asuntos.** El caso típico: un poder que sirve para varios casos.
5. **Estado del asunto:** existe desde ahora; todos quedan "abierto". Cómo se cierra o se archiva se decide aparte (P-213).
6. **Las referencias no son entidades.** Un número de acta, un expediente o "el contrato de X" son **datos del documento**: se pueden buscar y sirven de evidencia para proponer asuntos. Salen de la lectura que ya existe; no se escriben reglas propias para encontrarlas (quizás Amazon Textract más adelante, P-026).
7. **Se cuentan familias, no copias.** Un documento con sus versiones o copias nunca es un asunto por sí solo.
8. **Una sola vista por cosa.** Personas y organizaciones, Asuntos y el detalle de Documentos se unifican con el wiki: arriba el artículo (lo que dicen los documentos, con citas y certeza); en una pestaña, los datos y las correcciones.

## 3. Qué pasa con los datos de hoy

- Las **85 entidades de tipo asunto** dejan de ser entidades y pasan a ser **referencias** de sus documentos. Desaparecen de Personas y organizaciones.
- De los **7 grupos**, los 4 que son un solo documento desaparecen. Los **3** con documentos distintos quedan como **propuestas** para confirmar en Atención.
- Las etiquetas "Caso" que ya existan se respetan; las reglas de carpeta dejan de crear asuntos.
- No hace falta volver a leer los documentos: todo sale de lo que ya está en la base.

## 4. Las pantallas

- **Asuntos** (en abogados, "Casos"): la lista, con su estado (abierto) y si está confirmado o propuesto.
- **Página del asunto:** sus documentos (cada familia una vez), con el motivo por el que está cada uno, sus partes y sus referencias.
- **Detalle del documento:** sus referencias (número de acta, expediente…) y los asuntos en los que está.
- **Atención:** los asuntos propuestos, para confirmar o descartar, con la evidencia que los une.
- **Crear y editar a mano:** crear un asunto, ponerle nombre, agregar o sacar documentos (también desde el detalle del documento).

## 5. Una sola vista por cosa

Hoy la misma cosa se ve dos veces: en una pantalla de la base y en una página del wiki. Se unifican así:

| Pantalla de la base | Página del wiki |
|---|---|
| Personas y organizaciones | Página de la entidad |
| Asuntos | Página del asunto |
| Detalle de Documentos | Página del documento (Sources) |
| Orígenes | No tiene par en el wiki |

En todas: **arriba el artículo**, que se lee como una página; **en una pestaña, los datos y las correcciones** (unir, separar, dato incorrecto, sacar de un asunto).

## 6. Etapas

En esta tanda (20 oct):

1. **S2.** Las referencias pasan a ser datos del documento; Personas y organizaciones deja de mostrar asuntos.
2. **S3.** Los asuntos propuestos por el sistema, contando familias; se confirman en Atención.
3. **S4.** Crear y editar un asunto a mano.
4. **S5.** La página única de persona u organización.

Después: la página única del asunto y la del documento (P-214), y el ciclo de vida del asunto (P-213).
