# Documentos compuestos y documentos incluidos — propuesta

**Estado:** propuesta para que el dueño la revise antes de construir (D-525, P-350), 25 oct 2026. No se construyó nada. Puesta al día con D-527 el mismo día: lo que tiene emisor y fecha propios es un documento aparte, y el compuesto se analiza también como un conjunto.
**De dónde sale:** la observación 3 del dueño (un PDF que junta muchos documentos), la [investigación del cortador](../investigacion/corte/informe.md) y una prueba en seco del cortador sobre la base, sin modificarla (sus detalles tienen datos reales y quedan fuera del repositorio; acá van solo números).

## En pocas palabras

- Un PDF puede juntar varios documentos. Hoy la app lo trata como uno solo: un tipo, un título y todas sus personas mezcladas.
- **El archivo no se toca.** Cada documento incluido pasa a ser un documento más de la base, con su página, su tipo, su fecha, sus personas, sus casos y sus copias, y dice de qué compuesto es y qué páginas, o qué parte de una página, ocupa.
- **Lo que tiene emisor y fecha propios es un documento aparte (D-527):** una certificación, una hoja de firmas, un comprobante de timbre o una respuesta al dorso es un documento, relacionado con lo que certifica, firma o acompaña. Lo que no los tiene (un dorso en blanco, un anexo sin firma) sigue siendo parte (sección 3).
- **El compuesto se analiza también como un conjunto,** después de leer cada documento incluido: las relaciones entre personas, bienes, organizaciones y documentos, y un control del corte contra lo que anuncia el escrito. Una persona revisa solo lo que, si está mal, cambia algo (sección 6).
- **Los cortes los propone la app:** primero reglas, sobre la ficha de forma de cada página y sin modelo; el modelo, solo en los pasos dudosos; y lo dudoso lo confirma una persona, en una pantalla con las páginas en miniatura.
- **Lo ya leído no se vuelve a leer:** el texto, las personas y el resumen del compuesto se reparten por página entre sus incluidos, sin consultas. Las consultas son solo para los cortes dudosos (51 en la prueba en seco, para 12 PDF).
- **Core entra como un proveedor más** para lo dudoso y para dos documentos en una misma hoja, con el mismo formato de respuesta que el cortador local.
- Quedan **10 preguntas para el dueño** (sección 11), cada una con una recomendación.

## 1. El problema

Un ejemplo inventado. Un PDF de 23 páginas, «Documentación adjunta», que un estudio presentó con un escrito:

| Páginas | Qué es |
|---|---|
| 1 a 3 | el escrito (digital, firmado) |
| 4 a 6 | un poder (escaneado) |
| 7 y 8 | una partida de nacimiento y su certificación al dorso (escaneo, otro papel) |
| 9 | dos fotos de documento de identidad en la misma hoja: la de Ana Pereyra y la de Luis Gómez |
| 10 a 22 | seis partidas más, cada una con su hoja de firmas digital |
| 23 | una declaratoria de herederos |

Lo que hace hoy la app con ese PDF:

- **Es un solo documento,** con el tipo y el título de su primera parte (un escrito), no los del conjunto.
- **Las personas se mezclan:** todas las que nombra cualquiera de las 23 páginas figuran como nombradas en «el escrito». No se sabe cuál viene del poder y cuál de una partida.
- **No se puede buscar por tipo:** buscar «partidas de defunción» no encuentra la que está en las páginas 15 y 16.
- **No puede estar en un caso por partes:** si la declaratoria también sirve en otro expediente, hay que poner ahí el PDF entero.
- **«Copias y versiones» no ve lo repetido:** el mismo documento de identidad adjuntado en dos escritos son dos PDF distintos que comparten una página, y nunca se dice que es el mismo documento.
- Lo manuscrito que se leyó creó personas falsas; por eso ya se decidió no transcribirlo solo (D-518).

**Cuánto pasa** (medido sobre la base de prueba, solo números): 127 PDF, 583 páginas. 14 PDF tienen señales gruesas de ser compuestos (los armó una herramienta para unir PDF, o alternan escaneo y digital, o cambian de papel), y 3 más están en la hoja de partes del dueño. En la prueba en seco, 12 de esos PDF (124 páginas) dieron 79 documentos incluidos.

## 2. Las palabras (D-512)

| Concepto | Adentro (canónico) | En el código | En la pantalla, vertical jurídica |
|---|---|---|---|
| El PDF que junta varios documentos | documento compuesto | `compound` | escrito con documentación adjunta, o documentación adjunta |
| Cada documento que junta | documento incluido | `piece` (el nombre interno que ya se usa con Core) | documento adjunto |
| El límite entre dos documentos incluidos | corte | `cut` | corte |

- Las palabras de la pantalla salen del vocabulario de la vertical, nunca del código, y no dependen de dónde vino el PDF (del portal, de una carpeta o de un correo).
- Que un documento sea compuesto es su **estructura**. Qué papel cumple en el caso (escrito, despacho, notificación) es otra cosa y se guarda aparte, como hoy.

## 3. Cómo se guarda un documento incluido

### La idea

- **El archivo no se toca.** El PDF original sigue entero, con su huella: es la prueba.
- **Cada documento incluido es un documento más de la base.** Tiene el mismo registro que hoy tiene cualquier documento, así que todo lo que la base ya sabe hacer con un documento (tipo, datos, personas, casos, copias y versiones, búsqueda, plantillas) le sirve sin inventar algo nuevo en cada capa.
- **Además, un registro nuevo dice dónde está:** de qué compuesto es, en qué orden y qué páginas o qué región ocupa.

### Qué es un documento aparte (D-527)

**Lo que tiene emisor y fecha propios es un documento aparte,** relacionado con lo que certifica, firma o acompaña. Lo que no los tiene sigue siendo parte del documento.

| Pieza | Es | Relación (identificador neutro, D-512) | En pantalla |
|---|---|---|---|
| La certificación de copia fiel al dorso de una partida | un documento aparte | `certifies` (nueva) | «certifica a» |
| La hoja adicional de firmas de una digitalización | un documento aparte | `signature-sheet-of` (nueva) | «hoja de firmas de» |
| El informe de auditoría de un servicio de firma | un documento aparte | `signing-certificate-of` (ya está en el catálogo) | «constancia de firma de» |
| Un comprobante de pago o de timbre | un documento aparte | `payment-of` (nueva) | «comprobante de pago de» |
| La respuesta de un registro al dorso del pedido | un documento aparte | `reply-to` (ya está en el catálogo) | «respuesta a» |
| Las notas de inscripción de un registro al final de una escritura | un documento aparte | `registration-of` (nueva) | «notas de inscripción de» |
| Un dorso en blanco, un anexo sin firma ni fecha | parte del documento | — | — |

- **Por qué:** así el corte no necesita reglas especiales. Separar una partida de su certificación no es un error: la relación se establece después, mirando el contenido (sección 6). Se probó lo contrario, una regla para que la hoja de firmas no corte, y se sacó: mejoraba la prueba en seco, pero con reglas frágiles ([investigación](../investigacion/corte/informe.md)).
- **Las relaciones nuevas van al catálogo** (R-26), dirigidas y sin ser «el mismo documento», con su palabra de la vertical en el vocabulario. El programa no las nombra como excepción.
- **En la hoja de partes:** 104 documentos en lugar de 88, con 16 relaciones de este tipo.

### Qué se guarda de cada documento incluido

| Qué | Ejemplo | Dónde se guarda |
|---|---|---|
| De qué compuesto es y en qué orden | el 3.º de «Documentación adjunta» | registro nuevo |
| Qué ocupa: páginas, o una región de una página | páginas 4 a 6; o la mitad de arriba de la página 9 | registro nuevo |
| Si el corte está propuesto o confirmado, y quién lo confirmó | propuesto; confirmado por una regla segura; confirmado por una persona | registro nuevo |
| Con qué otros documentos se relaciona | «certifica a» la partida de la página 7; «adjunto a» el escrito | registro nuevo, con las relaciones del catálogo |
| Por qué se cortó | «la página 7 es otro papel»; «lo dijo el modelo, con confianza alta»; «lo marcó una persona» | registro nuevo |
| Tipo y etapa | partida de nacimiento | como cualquier documento; si no se sabe, «sin clasificar» |
| Fecha, emisor y los demás datos de su tipo | 12 mar 1961; Registro Civil | como cualquier documento |
| Personas y lo que dice de ellas, con su cita | Ana Pereyra, nacida en tal fecha | como cualquier documento |
| Su texto | el texto de las páginas 4 a 6 | se corta del texto del compuesto, sin volver a leer |

- **La región** es un rectángulo medido en proporciones de la página («de arriba hasta la mitad»), así no depende de la resolución del escaneo. Es para los casos como dos fotos de documento de identidad en una hoja (D-518). En la investigación fueron 2 cortes de 80.
- **El texto** de cada incluido sale del que la base ya guarda del compuesto, que está separado por página. No se vuelve a leer nada. Para una región: si la lectura guardada no tiene la posición de las palabras, el texto de la página queda en el primer documento de esa página hasta que se lea la región (una consulta, cuando una persona confirma la región).
- **Ninguna página se descarta:** cada página queda en exactamente un documento incluido (en la investigación se cumplió siempre). Lo que no se puede clasificar queda «sin clasificar», a la vista y en la búsqueda.
- **El compuesto sigue siendo un documento:** conserva su archivo, su huella, sus ubicaciones y su texto entero. Su tipo es «documento compuesto» y su título habla del conjunto, armado sin modelo con los tipos de sus incluidos: «Documentación adjunta: escrito, poder, siete partidas y declaratoria».

### Los estados de un corte

Como todo lo que propone la app, lo dudoso lo confirma una persona (R-03):

- **Confirmado por una regla segura:** las reglas que no fallaron ni en la investigación ni en la prueba en seco (la página siguiente está en blanco o tiene solo el pie; el mismo código de documento en las dos páginas; hojas notariales correlativas; las dos caras de una tarjeta). No piden nada.
- **Propuesto:** todo lo demás (las reglas de forma, el cambio de escaneo a digital, «página x de y», las fórmulas de apertura y de cierre, la respuesta del modelo). Se ve con su motivo y espera a una persona. Se marcan «para mirar» solo los que, si están mal, cambian algo (sección 6.4); no un documento corto que certifica, firma o acompaña al anterior.
- **Confirmado por una persona:** lo que una persona confirmó, unió, partió o marcó. Se guarda con lo que se le mostró, como las otras decisiones, y una lectura nueva no lo cambia.
- Qué hacer con lo que se apoya en un corte propuesto (por ejemplo, decir que ese documento de identidad es el mismo que otro) es la pregunta 3.

### Lo que se descartó

- **Partir el archivo en PDF nuevos:** no; el original no se toca (ya decidido). Una copia aparte se hace solo a pedido (sección 7.4).
- **Guardar los incluidos solo como secciones de la página del compuesto,** sin ser documentos de la base: cada capa (personas, casos, copias, búsqueda, plantillas) tendría que aprender qué es «una parte de un documento», y aun así el mismo documento de identidad no podría estar en dos escritos. Es la pregunta 1.

### Para quien lo construya

- Una tabla nueva, `content_piece`: el compuesto, el orden, el documento incluido (un `content` propio), la primera y la última página, la región (si hay), el estado, quién lo decidió, el motivo y la confianza. Va en una versión nueva de la base (la siguiente a la 29 de hoy), con su migración y su prueba.
- Un incluido no tiene un archivo propio (`file`): su identidad es la huella del compuesto más sus páginas o su región. Como la base identifica un documento por su texto, dos incluidos con el mismo texto son el mismo documento (lo que se busca para el mismo documento de identidad digital), salvo los de muy poco texto propio (sección 5.4).

## 4. Cómo se detectan los cortes

```mermaid
flowchart LR
    F["Ficha de forma de cada página<br/>(sin modelo)"] --> R["Reglas, en orden<br/>(sin modelo)"]
    R -->|regla segura| S["Corte confirmado"]
    R -->|regla de forma| P["Corte propuesto"]
    R -->|ninguna decide| D{"Paso dudoso"}
    D -->|ninguna regla cortó en el PDF| U["Continúa:<br/>un solo documento"]
    D -->|si no| M["Modelo, solo en ese paso"]
    M --> P
    P --> H["Una persona confirma o corrige"]
```

1. **La ficha de forma** (D-522) dice de cada página, sin modelo, si es un escaneo o digital, su papel y su resolución, sus imágenes, su texto propio, y si trae «página x de y», un código de documento o una fórmula de apertura o de cierre.
2. **Las reglas** se aplican en orden y la primera que vale decide «continúa», «empieza otro» o «dudoso» (la tabla completa, con sus aciertos y errores, está en la [investigación](../investigacion/corte/informe.md)).
3. **El modelo** mira solo los pasos dudosos, con las imágenes de las dos páginas.
4. **Una persona** confirma o corrige lo propuesto (sección 7.3).

### Los números de la investigación

Seis PDF de prueba, 134 páginas, 80 cortes y 88 documentos, medidos contra la hoja de partes del dueño:

| Variante | Consultas al modelo | Cortes encontrados | De cada 100 cortes propuestos, cuántos son reales | Documentos exactos |
|---|---|---|---|---|
| Solo reglas | 0 | 62 de 80 (78 %) | 84 | 47 de 88 |
| **Reglas, y el modelo solo en lo dudoso (la que conviene)** | **47** | **79 de 80 (99 %)** | **85** | **72 de 88** |
| Reglas, y el modelo en lo dudoso y en las reglas de forma | 100 | 73 de 80 (91 %) | 94 | 69 de 88 |
| El modelo en todos los pasos | 128 | 73 de 80 (91 %) | 96 | 70 de 88 |

- La variante que conviene encuentra casi todos los cortes con la tercera parte de las consultas. Sus errores son cortes de más, casi todos en adjuntos de un documento (la certificación al dorso, el comprobante de un timbre, la hoja de firmas), y se arreglan uniendo dos documentos en la pantalla, que es más fácil que encontrar un documento perdido dentro de otro.
- **Con la hoja al día con D-527** (104 documentos, 96 cortes), solo reglas: 71 de 96 cortes (74 %), 96 de cada 100 propuestos son reales, 57 de 104 documentos exactos. Casi todos los «cortes de más en adjuntos» pasan a ser cortes correctos. Las otras variantes no se volvieron a medir: usan el modelo.

### Lo que enseñó la prueba en seco sobre la base

12 PDF, 124 páginas, sin cambiar nada: 79 documentos incluidos, 40 con algo para mirar; 51 consultas para cortar y 79 más para el tipo y una descripción de cada documento. De los 67 cortes, 27 los hizo la regla «cambia de escaneo a digital», 14 una fórmula de cierre, 10 el modelo, 7 otra captura, 5 una fórmula de apertura y 4 otro papel.

- Un PDF de 24 páginas (partidas escaneadas, cada una con su hoja de firmas digital) quedó en 24 pedazos, cuando probablemente son unos 13: la regla de escaneo a digital, que en la investigación no había fallado, corta antes de cada hoja de firmas. Con D-527 eso es correcto: cada hoja de firmas es un documento aparte, «hoja de firmas de» su partida. Lo que falta es proponer la relación, no unir.
- 28 de las 51 consultas para cortar se fueron en dos PDF que resultaron uno y dos documentos.

### Mejoras antes de construir

Son reglas generales, en el catálogo (R-26), y se miden contra la hoja de partes, que mide y no enseña (R-25):

1. **Una hoja de firmas, una constancia de firma o una certificación es un documento aparte (D-527),** y se propone su relación con el anterior («hoja de firmas de», «certifica a»). Se reconoce por el vocabulario del catálogo o por el sello verificado del servicio de firma (D-519). Se probó lo contrario (unirla al anterior) y se sacó.
2. **El cambio de escaneo a digital pasa a ser una regla de forma:** propone, no confirma.
3. **La regla de otro papel vale solo entre escaneos de 100 puntos por pulgada o más** (una foto no tiene tamaño de papel), como ya hace la regla de las dos caras de una tarjeta.
4. **Las reglas de forma consultan al modelo solo si el documento siguiente sería de una o dos páginas;** si no, quedan propuestas, sin consulta.
5. **Ahorro:** si ninguna regla encontró un corte en un PDF, sus pasos dudosos no van al modelo: queda como un solo documento, y una persona puede partirlo. En la prueba en seco habría ahorrado 28 de las 51 consultas, a cambio de a lo sumo 1 corte (el único que hizo el modelo en esos dos PDF). Con las tablas de la investigación, en la hoja de partes ahorra 6 consultas, evita 2 cortes de más y no pierde ninguno. Además, los PDF de la base que no son compuestos (la gran mayoría) nunca van al modelo. Sale de las tablas, no se corrió: se mide antes de usarlo.
6. **En lo que se lee nuevo, la pregunta del corte va en la misma consulta que lee la página escaneada** (lo propuso el dueño en la observación 3): para esas páginas no hay consulta aparte. Sin medir.

### El tipo y la descripción de cada incluido

- Los da el paso de tipo que la app ya tiene: primero las marcas del catálogo, y el modelo una vez por grupo de documentos parecidos. La descripción se arma con los datos del tipo (fecha, emisor, personas), sin consulta.
- En la prueba en seco fueron 79 consultas aparte; con el paso que ya existe deberían ser bastante menos (sin medir).

### Dos documentos en una hoja (regiones)

- Si la página trae dos imágenes separadas (dos fotos puestas en un PDF digital), la ficha lo ve sin modelo y se propone una región por imagen.
- Si es un solo escaneo con dos tarjetas, un cortador por páginas no lo ve. Por ahora lo marca una persona, dibujando el rectángulo; lo automático se le pide a Core (sección 9).

### Lo manuscrito (D-518)

- Se detecta y se marca en cada documento incluido; no se transcribe solo; una persona puede pedir la lectura de un documento (P-334).
- La ficha de forma todavía no distingue lo impreso de lo manuscrito: eso hay que construirlo, o pedírselo a Core.

## 5. Qué cambia en cada capa

### 5.1 La lectura y lo que se dice de cada documento

- Hoy la base lee cada documento una vez con el modelo, y cada persona que encuentra, cada cosa que dice de ella y cada punto del resumen guardan la página de su cita.
- **Con los incluidos, eso se reparte por página:** lo de la página 7 pasa a ser de la partida, y lo de la 8, de su certificación. Sin consultas. Lo que no dice de qué página es queda en el compuesto.
- Lo nuevo se sigue leyendo una vez por compuesto (el mismo costo que hoy) y se reparte igual. Un incluido se lee aparte solo si su tipo tiene un tratamiento propio que lo mejora:
  - **documento de identidad:** la zona de lectura automática del dorso trae número, fecha de nacimiento y dígitos de control, que se comprueban sin modelo;
  - **formularios** (partidas, certificados): los campos, como datos;
  - **manuscrito:** solo a pedido de una persona.
- «Lo esencial» del compuesto se agrupa por documento incluido.
- Al cambiar un corte (unir o partir), lo de esas páginas se vuelve a repartir, sin consultas.

### 5.2 Las personas

- La página de una persona dice en qué documento incluido se la nombra: «Partida de nacimiento, en Documentación adjunta, página 7», y no «el PDF de 23 páginas».
- Hoy, si el escrito y el poder nombran a la misma persona, la lectura lo une adentro del documento. Partidos, son dos documentos, y lo une la resolución de identidades entre documentos, que ya existe. Lo que dijo la lectura («es la misma que…») se conserva como evidencia para esa resolución, para que no aparezcan identidades dudosas nuevas.
- Una lectura dudosa de manuscrito no crea una persona: queda «sin confirmar» y se compara con las personas del caso (observación 3).

### 5.3 Los casos: ¿apuntan al compuesto o al incluido?

Propuesta: a los dos, cada uno con su papel (pregunta 4).

- **El compuesto entra al caso como hoy:** es lo que se presentó.
- **Sus incluidos están en el caso a través de él:** se ven debajo, no se guardan otra vez, y si el compuesto sale del caso, salen con él.
- **Un incluido puede entrar por su cuenta a otro caso:** la declaratoria que también sirve en el otro expediente. Ahí se guarda, como cualquier documento.
- Las propuestas de caso por evidencia (el mismo número de expediente, el mismo importe) miran el incluido donde está la evidencia, no el PDF entero.

### 5.4 «Copias y versiones»

- **El mismo documento de identidad adjuntado en dos escritos es el mismo documento:**
  - si los dos son digitales y tienen el mismo texto, la base ya los trata como uno (identifica un documento por su texto): el mismo incluido aparece en los dos compuestos, sin decisión;
  - si son dos escaneos, se comparan como hoy cualquier par («prácticamente el mismo», «copia»), y el número de documento cuenta como dato clave (D-497): si coincide, se propone «el mismo»; si difiere, son distintos. Lo confirma una persona en «Copias y versiones».
- **Un incluido y un archivo suelto** (la declaratoria adjuntada acá y también guardada como PDF propio en otra carpeta) se comparan igual y pueden ser el mismo documento. Hoy eso salía como «uno dentro de otro».
- **Un compuesto y sus propios incluidos nunca forman un par:** están uno dentro del otro por construcción.
- **Un texto muy corto no hace a dos incluidos el mismo documento** (por ejemplo, una hoja de firmas genérica): por debajo de un mínimo de texto propio, que va en el catálogo, no se unen por texto.

### 5.5 La búsqueda

- Cada incluido tiene su texto en el índice, y su tipo y sus datos en los filtros: «partidas de defunción» encuentra la de las páginas 15 y 16.
- El resultado muestra el incluido, con «en Documentación adjunta, páginas 15 y 16». Si lo buscado está en un incluido, el compuesto entero no aparece además: aparece una vez.
- El compuesto se encuentra por su nombre y por su conjunto.

### 5.6 Las plantillas

- Las plantillas se arman con los documentos incluidos, no con el compuesto: siete partidas de compuestos distintos pueden ser «hechas con la misma plantilla».
- Un compuesto nunca es plantilla ni ejemplar de una: es un conjunto, no un modelo.
- Generar un documento desde una plantilla no cambia.

### 5.7 Lo que pueden pedir el chat y otros programas

- Las acciones nuevas (confirmar un corte, unir, partir, marcar una región, cambiar el tipo, guardar aparte) entran al contrato de operaciones (D-498): el chat puede proponerlas y las confirma una persona en la app.

## 6. El documento compuesto como un conjunto

Después de leer cada documento incluido, se hace una pasada más sobre el compuesto entero. Lo que encuentra no lo dice ningún documento solo. Los ejemplos son inventados.

### 6.1 Qué relaciones busca

- **Entre personas:** el parentesco de un grupo de partidas. La partida de nacimiento de Ana Pereyra dice que sus padres son Juan Pereyra y Marta Ruiz; la de Luis Pereyra, lo mismo; el acta de matrimonio une a Juan y a Marta. Entonces Ana y Luis son hermanos, hijos de ese matrimonio.
- **Entre personas y bienes:** un bien y su titular. La escritura dice que Juan Pereyra compró un departamento, y el informe del registro, que está inscripto a su nombre.
- **Entre personas y organizaciones:** el apoderado y el escribano. Marta Ruiz otorga un poder al abogado Carlos Díaz ante la escribana Laura Sosa, y el escrito lo firma Carlos Díaz como apoderado.
- **Entre documentos:** las de la sección 3 («certifica a», «hoja de firmas de», «comprobante de pago de», «respuesta a»), y «adjunto a» el escrito que los acompaña.
- **Cómo:** primero con los datos que ya trae cada tipo (los padres en una partida, el titular en una escritura, el apoderado en un poder), sin modelo. Para las relaciones entre documentos ayudan el orden en el PDF, el tipo y un código o número compartido. El modelo, a lo sumo una consulta por compuesto y solo para lo que quede sin resolver, si mide mejor (D-513).
- Todo lo que encuentra es una **propuesta** (R-03): se ve con su motivo y con los documentos de donde sale.

### 6.2 El control del corte contra lo que anuncia el escrito

- Muchos escritos enumeran lo que acompañan: «adjunto: a) partida de nacimiento, b) acta de matrimonio, c) poder». Se lee esa lista, con el vocabulario del catálogo y sin modelo, y se compara con los documentos que dejó el corte.
- **Si coincide,** el corte queda respaldado y se confirma solo.
- **Si no coincide** (el escrito anuncia tres y el corte encontró dos, o encontró uno que no se anuncia), es «para mirar»: probablemente dos documentos quedaron pegados o uno se partió.
- Las certificaciones, los comprobantes y las hojas de firmas no cuentan como anexos aparte: van con el que certifican, firman o acompañan.

### 6.3 Que una persona los juntó es un dato

- Que alguien haya puesto esos documentos en un solo archivo es evidencia de que pertenecen al mismo asunto: la partida, el acta y la declaratoria de una misma sucesión.
- Es evidencia, no prueba: suma al proponer que los incluidos van al mismo caso (pregunta 4) y que una persona nombrada en dos de ellos es la misma, pero no alcanza sola.

### 6.4 Cuándo revisa una persona

Solo lo que, si está mal, cambia algo:

- **dos documentos pegados** de personas distintas o de tipos distintos: la partida de Ana y la de Luis en un solo documento mezclan a sus personas;
- **un documento partido al medio:** las páginas 1 y 2 de una escritura en dos documentos, ninguno con el contenido entero;
- **el control contra el escrito que no coincide** (sección 6.2);
- **una relación entre personas que cambia quién es quién** (un parentesco, un apoderado), como cualquier propuesta.

No revisa el corte entre un documento y su certificación, su hoja de firmas o su comprobante: si se cortó, es correcto; si quedó junto, el contenido es el mismo y no se pierde nada.

## 7. Las pantallas

### 7.1 La página del documento compuesto

```
Documentación adjunta: escrito, poder, siete partidas y declaratoria
Escrito con documentación adjunta · 23 páginas · 18 documentos adjuntos · 2 para mirar
[Abrir el original]  [Revisar los cortes]

Documentos adjuntos
 N.º  Páginas  Qué es                     Fecha         Emisor           Personas
 1    1 a 3    Escrito                    4 jun 2024    (el estudio)     Ana Pereyra y 2 más
 2    4 a 6    Poder general judicial     2 feb 2023    (una escribanía) Ana Pereyra, Luis Gómez
 3    7        Partida de nacimiento      12 mar 1961   Registro Civil   Ana Pereyra
 4    8        Certificación de copia fiel (certifica a 3)
 5    9 (arriba) Documento de identidad   —             —                Ana Pereyra       ◐ región marcada a mano
 …
 18   23       Sin clasificar             —             —                —

Lo esencial
 · Escrito: …
 · Poder: …
```

- El título y el tipo son los del conjunto, no los de la primera parte.
- Arriba, la lista de los documentos incluidos: páginas, qué es, fecha, emisor, personas y, si hace falta, por qué hay que mirarlo.
- «Lo esencial», agrupado por documento incluido.
- El texto entero y las ubicaciones, como hoy.

### 7.2 La página de cada documento incluido

Es igual a la de cualquier documento (título por su tipo y sus datos, personas, datos, casos, copias y versiones, texto), y además:

- arriba, «Documento adjunto de Documentación adjunta, página 7», con enlace al compuesto, y «Anterior · Siguiente» para recorrer los incluidos;
- las páginas del original, en imagen, para mirarlas sin abrir el PDF, y «Abrir el original en la página 7»;
- el estado del corte, si está propuesto: «Propuesto: la página 7 es otro papel. [Confirmar] [Unir con el anterior] [Revisar los cortes]»;
- «Guardar este documento aparte»;
- si tiene manuscrito: «Tiene partes escritas a mano que no se leyeron. [Pedir la lectura]».

### 7.3 Cómo se confirma o se corrige un corte

«Revisar los cortes» abre la tira de páginas del compuesto en miniatura, con una raya entre dos páginas donde hay un corte:

- raya llena: confirmado; raya punteada: propuesto, con su motivo al pasar el ratón («la página 7 es otro papel»);
- un clic en una raya: «Unir»; un clic entre dos páginas sin raya: «Partir acá»;
- arrastrar sobre una página: «Marcar región», para dos documentos en una hoja;
- cada documento muestra su tipo, que se puede cambiar o dejar «sin clasificar»;
- «Confirmar todos» confirma lo propuesto que la persona dejó como está; todo se puede deshacer.

La vista de atención dice «N documentos compuestos con cortes para mirar» y lleva a esta pantalla, con los «para mirar» primero. Cuánto trabajo da: en la prueba en seco, 40 de 79 documentos tenían algo para mirar; con las mejoras de la sección 4 deberían ser menos (sin medir), y con D-527 queda solo lo que cambia algo (sección 6.4).

### 7.4 «Guardar este documento aparte», a pedido

- Hace un PDF con esas páginas del original, sin modificarlas, o con la región recortada como una página.
- Va a la carpeta de trabajo, en «Generados», con un nombre armado con el tipo, la fecha y las partes («Partida de nacimiento – Ana Pereyra – 1961.pdf»); la persona puede elegir otro lugar.
- La base sabe desde el primer momento que ese PDF es el mismo documento que el incluido: no lo vuelve a leer.
- La copia no lleva la firma digital del original (una firma cubre el PDF entero); el PDF nuevo dice de qué archivo y de qué páginas salió.

## 8. Cómo se reprocesa lo ya leído

- **Qué documentos:** las reglas corren sobre los 127 PDF de la base, sin modelo y en segundo plano. Se cortan los que tengan algún corte: se esperan los 15 de las mediciones (los 12 de la prueba en seco y los 3 de la hoja de partes), y quizá alguno más. Un PDF sin ningún corte queda como está.
- **Cuántas consultas al modelo, una sola vez:**
  - para cortar: en la prueba en seco fueron 51 para los 12 PDF; con el ahorro de la sección 4 se irían hasta 28 de esas 51. Los 3 de la hoja suman a lo sumo 47 (todo lo dudoso de los 6 PDF de la investigación);
  - para el tipo: el paso de tipo que ya existe, en lugar de las 79 consultas aparte de la prueba en seco;
  - para lo que dicen los documentos: ninguna; lo leído se reparte por página.
  - Antes de correrlo sobre la base, otra prueba en seco, con las mejoras, para tener el número medido.
- **Las respuestas de la hoja de partes no se cargan como confirmaciones** de los 3 PDF que están en ella: pasan por el mismo cortador, porque la hoja mide y no enseña (R-25); cargarla arruinaría las mediciones que vengan.
- **El respaldo:** antes de la versión nueva, una copia de la base. Los archivos no se tocan. Los cortes se deshacen: «Deshacer los cortes» devuelve todo al compuesto.
- **La versión de la base:** una versión nueva, con su migración y su prueba. La migración solo agrega lo nuevo y no dispara ninguna lectura (como se decidió para lo ya reconocido, D-523). Cortar lo ya leído es una tarea en segundo plano, y la vista de atención muestra lo que propuso.
- **Lo leído de lo manuscrito antes de decidir no transcribirlo (D-518):**
  - la lectura de imágenes con Codex escribe lo manuscrito entre corchetes («[manuscrito: …]»), así que donde quedó marcado se encuentra sin consultas;
  - propuesta: se conserva (ya se pagó), pero queda como «lectura de manuscrito sin confirmar»: no crea personas ni afirmaciones, se ve con su marca, y una persona puede confirmarla, que equivale a haber pedido la lectura. Las personas que salieron solo de ahí pasan a «sin confirmar»;
  - donde la lectura vieja no lo marcó, no se sabe sin detectar el manuscrito: antes hay que medir cuántas páginas son (pregunta 6).

## 9. El papel de Core

- Core aceptó ofrecer las piezas de un documento (respuesta al mensaje 003), sin fecha.
- **Entra como un proveedor elegible más,** como en la lectura: Desktop corre siempre las reglas en el dispositivo, sin modelo, y le pide a un proveedor solo lo dudoso. Hoy ese proveedor es Codex; cuando Core lo ofrezca, se elige en Ajustes. La base guarda qué proveedor respondió cada cosa.
- **Qué le pediríamos a Core:**
  1. la misma respuesta que da el cortador local: para cada documento, sus páginas o su región (un rectángulo en proporciones de la página), su tipo con las palabras canónicas de nuestro catálogo (se lo mandamos), su confianza y su motivo;
  2. poder mandarle solo un paso dudoso (dos páginas) o una página, no el PDF entero;
  3. encontrar dos documentos en una misma hoja, como las dos fotos de documento de identidad, que un cortador por páginas no ve;
  4. marcar dónde hay manuscrito, firmas y sellos, sin transcribir lo manuscrito (D-518);
  5. la lectura propia de cada tipo: la zona de lectura automática de un documento de identidad, los campos de un formulario, las tablas;
  6. el precio por página y por pieza, y que lo que falla no se cobre;
  7. que borre lo que le mandamos al terminar, como en la lectura.
- **Lo que queda en Desktop, sea cual sea el proveedor:** las reglas, el guardado, las pantallas y la confirmación de una persona. Si algún día todo el preprocesamiento corre en el dispositivo (lo que el dueño está evaluando, P-321), el formato no cambia.

## 10. Orden propuesto para construir

Cada etapa termina medida contra la hoja de partes y vista por el dueño en la app; ninguna se cierra por una lista de tareas.

| Etapa | Qué | Tamaño relativo | ¿Usa el modelo? |
|---|---|---|---|
| 1 | Guardar los incluidos y mostrarlos: la versión nueva de la base, la página del compuesto con su lista, la de cada incluido; solo los cortes de reglas seguras | Mediano | No |
| 2 | Revisar los cortes: la tira de páginas; unir, partir, confirmar, deshacer; la vista de atención | Mediano | No |
| 3 | El cortador entero: las reglas de forma, las mejoras de la sección 4 y el modelo en lo dudoso | Chico (ya existe en la investigación) | Solo en lo dudoso |
| 4 | Las capas: repartir lo leído por página, personas, casos, copias y versiones, búsqueda, plantillas; y la pasada del conjunto (sección 6) | Grande | No; el conjunto, a lo sumo una consulta por compuesto |
| 5 | Reprocesar la base, después de otra prueba en seco | Chico | Una vez |
| 6 | Regiones y «Guardar este documento aparte» | Mediano | No |
| 7 | Core como proveedor, cuando lo ofrezca | Según Core | Sí |

En conjunto es bastante más grande que las observaciones 1 y 2. Las etapas 1 a 3 solas ya dejan ver y corregir los cortes.

## 11. Preguntas para el dueño

### Pregunta 1. ¿Cada documento incluido es un documento de la base, o solo una sección de la página del compuesto?

- **Qué es:** cómo de «documento» es un documento incluido.
- **Ejemplo:** la partida de la página 7.
- **Opciones y qué cambia:**
  - a) **Un documento propio:** tiene su página, aparece al buscar por tipo, puede estar en otro caso y puede ser «el mismo documento» que otro.
  - b) **Una sección del compuesto:** se ve en la página del compuesto, con su tipo y sus personas, pero no puede estar sola en un caso ni ser la misma que otra; la búsqueda lleva al compuesto.
- **Qué cuesta:** a) es lo más grande (la etapa 4 entera); b) más o menos la mitad, pero habría que rehacerla si después se quieren los casos o las copias.
- **Recomiendo a).**

### Pregunta 2. ¿Qué cuenta como «si está mal, cambia algo»?

- **Qué es:** D-527 dice que una persona revisa solo lo que, si está mal, cambia algo. Falta decir qué es eso: decide qué cortes se confirman solos y cuánto trabajo da la pantalla de cortes.
- **Ejemplo:** en un PDF con siete partidas, cada una con su hoja de firmas, nadie revisa las siete hojas de firmas; sí se revisa si dos partidas de personas distintas quedaron en un solo documento.
- **Opciones y qué cambia:**
  - a) **La lista de la sección 6.4:** dos documentos de personas o tipos distintos pegados, un documento partido al medio, el control contra el escrito que no coincide y una relación entre personas. Todo lo demás se confirma solo y se puede corregir después.
  - b) **Lo de a) y además todo corte del modelo con confianza media o baja,** aunque no se sepa si cambia algo.
  - c) **Todo propuesto, como antes de D-527:** una persona mira todo (en la prueba en seco, los 79 documentos).
- **Qué cuesta:** lo mismo de construir; cambia el trabajo de la persona: el mínimo con a), el máximo con c). a) necesita las personas y el tipo de cada documento antes de decidir, y eso ya lo da la lectura.
- **Recomiendo a),** y medir con la hoja de partes cuántos «para mirar» quedan.

### Pregunta 3. ¿Lo que se apoya en un corte propuesto espera a que se confirme?

- **Qué es:** la regla de que lo propuesto no alimenta lo propuesto (R-03). Un incluido propuesto tiene tipo, personas y texto, y además podría generar otras propuestas: «es el mismo documento que…», «va en tal caso».
- **Ejemplo:** el documento de identidad de la página 9 está propuesto. ¿Se propone ya que es el mismo que el adjuntado en otro escrito?
- **Opciones y qué cambia:**
  - a) **La lectura no espera; las propuestas, sí:** tipo, personas, texto y búsqueda usan el incluido propuesto, con su marca; lo que sería una propuesta nueva (copias y versiones, casos, plantillas) espera a que se confirme el corte.
  - b) **Nada espera:** se ve todo antes, pero es una propuesta sobre otra.
  - c) **Todo espera:** hasta que alguien confirme, el compuesto se ve como hoy.
- **Qué cuesta:** parecido en las tres; a) pide que cada capa sepa qué cortes están confirmados.
- **Con D-527:** casi todos los cortes se confirman solos (pregunta 2), así que espera poco. Las relaciones que propone la pasada del conjunto (sección 6) también son propuestas y siguen esta misma regla.
- **Recomiendo a).**

### Pregunta 4. ¿El caso apunta al compuesto o a sus incluidos?

- **Qué es:** qué documento queda en el caso cuando el que se presentó es un compuesto.
- **Ejemplo:** la «Documentación adjunta» está en el caso de una sucesión, y su declaratoria también sirve en otro expediente.
- **Opciones y qué cambia:**
  - a) **El compuesto entra al caso y sus incluidos con él; un incluido puede entrar solo a otro caso.**
  - b) **Cada incluido entra por separado:** el caso muestra 18 documentos en lugar de 1, y se pierde qué se presentó junto.
  - c) **Solo el compuesto:** la declaratoria no puede estar sola en el otro expediente.
- **Qué cuesta:** a) mediano; b) y c), un poco menos.
- **Con D-527:** que una persona haya juntado esos documentos en un archivo es evidencia de que son del mismo asunto (sección 6.3), y eso apoya a).
- **Recomiendo a).**

### Pregunta 5. ¿Lo que dice cada incluido se reparte de la lectura que ya está, o se lee cada uno de nuevo?

- **Qué es:** de dónde salen las personas, los datos y el resumen de cada incluido.
- **Ejemplo:** el PDF de 23 páginas ya se leyó, y cada persona que encontró tiene la página de su cita.
- **Opciones y qué cambia:**
  - a) **Repartir por página, sin consultas;** se lee aparte solo lo que tiene un tratamiento propio (documento de identidad, formularios) o lo que pide una persona.
  - b) **Leer cada incluido de nuevo,** con instrucciones de su tipo: quizá mejor, pero una consulta por incluido (79 en la prueba en seco) y otra vez con cada compuesto nuevo.
- **Qué cuesta:** a) ninguna consulta; b) una por incluido, siempre, que va contra bajar el costo del modelo (D-513) salvo que mida mejor.
- **Recomiendo a),** y medir b) solo en los tipos donde a) falle.

### Pregunta 6. ¿Qué pasa con lo manuscrito que ya se leyó antes de decidir no transcribirlo?

- **Qué es:** antes de la decisión del 24 de octubre se leyeron páginas manuscritas, y de ahí salieron personas, algunas falsas.
- **Ejemplo:** una partida con el nombre de la madre escrito a mano, leído con un error, creó una persona que no existe.
- **Opciones y qué cambia:**
  - a) **Se conserva como «lectura de manuscrito sin confirmar»:** no crea personas ni afirmaciones, se ve marcada, y una persona puede confirmarla.
  - b) **Se borra:** queda como si no se hubiera leído; si se quiere, se pide la lectura otra vez (y se paga otra vez).
  - c) **Se deja como está:** sigue creando personas, también las falsas.
- **Qué cuesta:** a) mediano (hay que encontrar lo manuscrito en lo guardado; donde Codex lo marcó, es fácil); b) chico; c) nada.
- **Recomiendo a).**

### Pregunta 7. ¿Se empieza con Desktop o se espera a Core?

- **Qué es:** quién corta lo dudoso mientras Core no ofrece las piezas.
- **Ejemplo:** los 51 pasos dudosos de la prueba en seco.
- **Opciones y qué cambia:**
  - a) **Empezar ya con Desktop** (reglas, y Codex en lo dudoso), con Core como proveedor cuando lo ofrezca y con el mismo formato.
  - b) **Esperar a Core:** no se gasta cupo de Codex, pero no hay fecha, y sin los cortes no avanza nada de esta propuesta.
- **Qué cuesta:** a) usa cupo de Codex solo en lo dudoso (en la prueba en seco, 51 consultas para 12 PDF, una sola vez; después, unas pocas por cada compuesto nuevo); b) nada hoy, todo después.
- **Recomiendo a).**

### Pregunta 8. ¿Las regiones (dos documentos en una hoja) se hacen ahora o después?

- **Qué es:** partir una página en dos documentos.
- **Ejemplo:** las dos fotos de documento de identidad de la página 9.
- **Opciones y qué cambia:**
  - a) **En esta propuesta, a mano:** una persona dibuja el rectángulo; más la regla sin modelo para dos imágenes separadas. Lo automático, con Core.
  - b) **Después:** mientras tanto, esa página es un solo documento incluido, con las dos personas.
- **Qué cuesta:** a) mediano (la pantalla para dibujar y el recorte para guardar aparte); b) nada ahora. En la investigación fueron 2 cortes de 80.
- **Recomiendo a),** en la etapa 6, después de lo demás.

### Pregunta 9. ¿Las relaciones entre personas que salen del conjunto se proponen en la página de cada persona?

- **Qué es:** la pasada del conjunto (sección 6) encuentra relaciones que ningún documento dice solo: el parentesco de un grupo de partidas, el titular de un bien, el apoderado de alguien.
- **Ejemplo:** de dos partidas de nacimiento y un acta de matrimonio sale que Ana y Luis Pereyra son hermanos, hijos de Juan Pereyra y Marta Ruiz.
- **Opciones y qué cambia:**
  - a) **Se proponen en la página de cada persona,** con los documentos de donde salen, y una persona las confirma (R-03). Sirven para responder «¿quiénes son los herederos?».
  - b) **Solo se muestran en la página del compuesto,** como un resumen del conjunto; no llegan a las personas.
  - c) **Se guardan como confirmadas** cuando salen de datos de formulario (los padres en una partida), sin una persona.
- **Qué cuesta:** a) mediano o grande, según lo que la base ya guarde de las relaciones entre personas; b) chico; c) como a), pero un error de lectura crea un parentesco falso sin que nadie lo vea.
- **Recomiendo a).**

### Pregunta 10. Si el corte no coincide con lo que anuncia el escrito, ¿qué se hace?

- **Qué es:** el control de la sección 6.2: el escrito dice «adjunto: a) …, b) …, c) …» y el corte encontró otra cosa.
- **Ejemplo:** el escrito anuncia una partida, un acta y un poder, y el corte encontró dos documentos, además de las certificaciones.
- **Opciones y qué cambia:**
  - a) **Se marca «para mirar» en el compuesto,** con lo que falta o sobra, y la pantalla de cortes lo muestra primero.
  - b) **Se le pide al modelo que vuelva a cortar ese compuesto** con la lista del escrito como pista; si sigue sin coincidir, «para mirar».
  - c) **Solo una nota en la página del compuesto,** sin pedir nada.
- **Qué cuesta:** a) chico; b) una consulta más por cada compuesto que no coincide; c) casi nada, pero un documento perdido dentro de otro queda sin que nadie lo vea.
- **Recomiendo a),** y probar b) si en la prueba en seco son muchos.

## 12. Riesgos y lo que falta medir

- **Los cortes en los adjuntos** (la certificación al dorso, el timbre, la hoja de firmas) ya no son un error con D-527: son documentos aparte. El riesgo pasa a ser **proponer mal la relación** (por ejemplo, una certificación que abarca dos partidas, relacionada solo con una); si está mal, no cambia el contenido de ningún documento.
- **La pasada del conjunto depende de los datos de cada tipo:** si la lectura no trae los padres de una partida, no hay parentesco. Y el control contra el escrito depende de reconocer su lista de anexos, que se escribe de muchas formas.
- **Dos documentos en la misma página** no los ve ningún cortador por páginas (2 de 80 cortes).
- **Si el PDF original cambia** (otra versión del archivo), sus cortes vuelven a «propuesto»; lo que había confirmado una persona se ofrece otra vez si las páginas siguen iguales.
- **Un incluido no tiene un archivo propio,** y hoy la app abre la página de un documento por su archivo: el incluido necesita su propia dirección (el compuesto y su número). Es trabajo interno de la etapa 1.
- **La lista de documentos crece** (79 incluidos de 12 PDF en la prueba en seco): las listas tienen que poder plegar los incluidos bajo su compuesto.
- **Al llamar al modelo con imágenes,** hay que comprobar que las imágenes existen: si una ruta no existe, Codex contesta igual, sin verlas (de la investigación).
- **Falta medir:** las mejoras de la sección 4 (contra la hoja de partes y en otra prueba en seco); cuántas páginas ya leídas tienen manuscrito; y si repartir por página deja bien a las personas (la hoja de respuestas de personas, antes y después); y la pasada del conjunto (sección 6): cuántas relaciones encuentra y cuántas están bien.

## Documentos relacionados

- [Investigación del cortador](../investigacion/corte/informe.md): las reglas, las variantes y cómo repetir la medición.
- [Base documental](base-documental.md): cómo se guarda hoy un documento, su texto, sus personas y sus casos.
- [Documentos parecidos](parecidos-propuesta.md): «Copias y versiones».
- [Servicios de IA](servicios-de-ia-propuesta.md): Core y las piezas de un documento.
- [Contrato de operaciones](contrato-de-operaciones.md): lo que pueden pedir el chat y otros programas.
- Decisiones: vocabulario (D-512), manuscrito y regiones (D-518), ficha de forma (D-522), lo ya leído (D-523), costo del modelo (D-513), documentos aparte y el compuesto como conjunto (D-527).
