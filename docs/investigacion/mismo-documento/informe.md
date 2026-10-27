# El mismo documento en dos lugares: el método de la app, los compuestos y pHash

26 de octubre de 2026. Prueba hecha en una sesión de Claude Code en la nube. Sigue a [`../huellas-y-embeddings/informe.md`](../huellas-y-embeddings/informe.md), que comparó huellas y embeddings por su cuenta, sin medir lo que ya hace la app. Los documentos y todo lo que salió de ellos quedaron fuera del repositorio.

## En resumen

1. **El método de la app ya es muy bueno.** En la carpeta de la empresa acierta 53 de los 57 pares medibles de la hoja, y entre PDF y Word reconoce el mismo documento. El método es el texto canónico, MinHash con secuencias de 5 palabras, la medida exacta sin el texto de rutina y la clasificación de diferencias. Se equivoca en 1 par (un «mismo documento» que propone como versión) y no encuentra 3: dos de esos pares son un Word contra un PDF.
2. **Con los compuestos partidos en sus documentos, el mismo método encuentra los sueltos.** De los 104 documentos incluidos en los seis PDF de prueba, 5 tienen un suelto con una relación fuerte («contenido en», versión, ejemplar firmado). Otros 9 tienen uno de la misma plantilla o parecido. Lo que falta en la app no es otro algoritmo de comparación: es partir el compuesto antes de comparar.
3. **pHash agrega poco y se equivoca en lo corto.** Encontró un suelto para 5 documentos incluidos:
   - 1 coincide con la app;
   - 1 es real y la app solo lo vio como parecido;
   - 3 son falsos: no comparten ni una secuencia de 5 palabras.
   
   En la hoja, pHash corregiría el único error de la app, pero, usado solo, marcaría 3 falsos donde la app acierta.
4. **Los escaneos sin texto siguen sin resolverse:** 18 de los 104 documentos incluidos no tienen texto, y ni la app ni pHash les encontraron un suelto. No se puede saber si es porque no lo tienen: la hoja no lo dice.
5. **Cómo construirlo, en orden:**
   1. partir los compuestos (investigación de corte en `../corte/`);
   2. pasar cada documento incluido por el método actual;
   3. usar pHash solo para desempatar entre «mismo documento» y «versión»;
   4. reconocer el texto de los escaneos antes de comparar.

## Cómo se probó

- **El código de la app, sin cambios:** la prueba que ya existe en `src-tauri/src/docbase/similar/check.rs`. Corre las pasadas de lectura y de copias y versiones sobre carpetas reales y, si se le da la hoja, mide contra ella.
  - Para compilarla en Linux hubo que instalar las librerías de la interfaz: GTK y WebKit.
  - Se compiló en modo de prueba, sin optimizar, así que los tiempos de la app son peores que los de la versión que se instala.
- **Tres corridas:**
  1. la carpeta de la empresa con la hoja de «Copias y versiones»: el punto de partida;
  2. los 104 documentos incluidos, cada uno como un PDF aparte, junto con las carpetas de la empresa y del caso. El corte de cada documento sale de `partes.json`, con las páginas originales del compuesto, sin rehacerlas;
  3. el cruce de esas relaciones con pHash, reusando las huellas de la prueba anterior.
- **Sin modelos:** las pasadas de la app que usan un modelo no corren en esta prueba, y el texto de los escaneos no se reconoció (en la app queda pendiente).
- **Reglas:** no se tocó ningún umbral de la app, y pHash usa los de la prueba anterior (distancia ≤ 6; páginas comunes, desde 5 contenidos). Los registros de la app, que tienen rutas reales, se leyeron con guiones que solo cuentan.

## 1. El método de la app contra la hoja

Carpeta de la empresa: 66 documentos, 55 con texto.

| Relación esperada | Pares | Aciertos | Equivocados | No encontrados |
|---|---|---|---|---|
| Sin relación | 12 | 12 | 0 | 0 |
| Vista previa | 1 | 0 | 0 | 1 |
| Mismo documento | 12 | 11 | 1 (propuso «versión») | 0 |
| Misma plantilla | 26 | 24 | 0 | 2 |
| Ejemplar firmado | 4 | 4 | 0 | 0 |
| Versión | 2 | 2 | 0 | 0 |
| **Total** | **57** | **53** | **1** | **3** |

- **Los 3 que no encontró:**
  - la vista previa, que es un Word contra el PDF firmado;
  - un par de misma plantilla, también Word contra PDF;
  - otro par de misma plantilla que propuso solo como «parecido».
- **Las 12 propuestas que la hoja no juzga** son documentos de la misma plantilla, que la hoja no lista par por par.
- **Tiempo:** 10 segundos de lectura y 6 de comparación.

## 2. Los compuestos partidos

234 documentos en la base (171 con texto): los 104 incluidos, más los sueltos de la empresa y del caso.

| Qué encontró la app para cada documento incluido | Documentos incluidos |
|---|---|
| Un suelto con relación fuerte («contenido en», versión, ejemplar firmado) | 5 |
| Un suelto solo de la misma plantilla o «parecido» | 9 |
| Ningún suelto, y el documento tiene texto | 72 |
| Ningún suelto, y no tiene texto | 18 |

- **Relaciones entre documentos incluidos y sueltos:** 2 «contenido en», 2 versiones, 1 ejemplar firmado, 8 de misma plantilla y 16 parecidos. 21 son con sueltos de la carpeta del caso y 8 con sueltos de la de la empresa.
- **Entre los mismos documentos incluidos** la app propuso 43 relaciones, casi todas parecidos y misma plantilla. Esas relaciones son las que el criterio de `partes.json` llama «relacionados».

## 3. El cruce con pHash

| | Documentos incluidos |
|---|---|
| Suelto según la app (fuerte) y según pHash | 1 (el mismo suelto) |
| Solo según la app (fuerte) | 4 |
| Solo según pHash | 4 |

**Revisión de los casos de un solo método,** con números y sin mirar los documentos:
- **De los 4 que solo encontró pHash:**
  - en 3 el documento incluido y el suelto no comparten ni una secuencia de 5 palabras: son falsos, páginas cortas que se ven parecidas;
  - en 1 comparten el 86 % del texto: es real, y la app lo vio solo como «parecido».
- **De los 4 que solo encontró la app:**
  - en 3 el suelto es más largo y contiene al documento incluido. pHash no lo vio porque la prueba anterior solo buscaba el suelto dentro del compuesto, no al revés: es un límite de esa prueba, no de pHash;
  - el cuarto es una versión con otras páginas, que pHash no puede ver.

**En la hoja de la empresa:**
- pHash dice «mismo» en el par que la app propuso como versión: lo corregiría.
- Marca 3 falsos donde la app acierta: 2 de misma plantilla y 1 sin relación.
- No ayuda en los 3 pares que la app no encontró: dos son de Word y el tercero no coincide en la imagen.

## Lo que no se pudo probar, y por qué

- **Escaneos sin texto:** la app deja el texto pendiente hasta que lo reconoce un modelo, y en esta prueba no se usó ninguno. Hace falta repetir la segunda corrida con el texto reconocido.
- **Si los 72 documentos incluidos sin suelto de verdad no lo tienen:** `partes.json` no dice cuáles tienen copia suelta.
- **pHash en los dos sentidos** (el suelto dentro del compuesto y el documento incluido dentro del suelto): no se midió.
- **Embeddings:** no se volvieron a probar. En la hoja, la app no deja de encontrar ningún par de PDF por falta de candidatos, así que no había nada que agregar ahí.

## Recomendación

1. **Partir los compuestos** y tratar cada documento incluido como un documento más en «Copias y versiones». Es lo que más agrega.
2. **Mantener el método actual de la app** (texto canónico, MinHash, la medida exacta y las diferencias): ya reconoce el mismo documento entre formatos y no confunde las plantillas.
3. **pHash, solo como desempate** cuando la app duda entre «mismo documento» y «versión», o para proponer candidatos entre escaneos. Nunca como prueba sola.
4. **Reconocer el texto de los escaneos antes de comparar,** porque sin texto no hay comparación posible.
5. **Revisar los dos fallos entre Word y PDF**, la vista previa y la plantilla. Es lo único que la app no resuelve en la hoja.

## Fichas de Claude

- **Sub-agentes:** ninguno.
- **Agente principal:** unas 35 mil fichas desde el arranque, según el contador de la sesión; el tope era de 150 mil. El contador probablemente no incluye la relectura del contexto.
- **Un error:** una vez, leyendo los errores de la app, pasaron nombres reales de archivos al contexto del agente. No quedaron en ningún archivo.

## Revisión de datos reales

Antes del commit se buscaron en esta carpeta:
- nombres de carpetas de clientes;
- nombres de las hojas de entidades;
- números largos, correos y rutas personales.

No apareció ninguno.
