# C002 · Respuesta: los vectores, medidos y en uso

- **Número:** C002 (responde al mensaje C002 de Core) · **Fecha:** 2 de noviembre de 2026 · **Estado:** respondido; con dos preguntas (C002-Q1 y C002-Q2)
- **De:** Micelya Desktop (`Micelya/llm_wiki`) · **Para:** Micelya Core (`extrimian/mi-platform`, rama `dev`)
- **Responde a:** `docs/dropbox/desktop/2026-10-30-C002-vectores-desplegados.md` del repositorio de Core, y a lo que pedía la 004-R5 (decir si el modelo alcanza).
- **Respuesta esperada en:** `docs/dropbox/desktop/` del repositorio de Core, con el número C002, solo para las dos preguntas.
- **Depende de esto en Desktop:** el paso «Vectores» del proceso de la base de documentos, que ya usa la ruta.

Es un mensaje informativo: no hace falta que Core cambie nada.

## En pocas palabras

- **Funciona.** Medimos Cohere Embed V4 por la ruta nueva con una carpeta de prueba de 297 documentos y con la prueba de 30 preguntas de la 004-R5.
- **Las dos variantes dan el mismo vector:** `us.` y `global.` coinciden salvo el redondeo.
- **Lo vamos a usar** con `us.cohere.embed-v4:0`, 1.024 números, de a 96 textos y una llamada a la vez: un vector por documento, para encontrar documentos parecidos y sugerir su tipo. Viene encendido en la app, con la opción de apagarlo.
- **Para buscar por significado en el chat todavía no alcanza** con un vector por documento entero. Lo volvemos a medir por página o por pedazo antes de decidir (Desktop P-610).

## Lo medido (1 nov 2026, ambiente de desarrollo, usuario técnico de Desktop)

### La carpeta

| Qué | Resultado |
| --- | --- |
| Textos | 297 documentos, 297 textos distintos (las copias comparten el suyo y no se piden) |
| Llamadas | 4, de hasta 96 textos; ninguna falló ni dio 503 |
| Espera | 13,0 / 10,5 / 8,4 / 2,4 segundos; 34 segundos en total |
| Tokens que informó la ruta | 286.433, unos 964 por documento |
| Costo, a USD 0,12 el millón de tokens de `us.` | USD 0,034 la carpeta; unos 12 centavos de dólar cada mil documentos |
| Vectores | 1.024 números, norma 1 en todos |

### `us.` contra `global.`

Una llamada más con los mismos 96 primeros textos por `global.cohere.embed-v4:0`:

| Qué | Resultado |
| --- | --- |
| Parecido (coseno) entre el vector `us.` y el `global.` del mismo texto | mínimo 0,99999994, mediana 1,0, en 96 de 96 |
| El documento más parecido, ¿es el mismo con las dos? | sí, en 96 de 96 |
| Tokens y espera | los mismos que con `us.` |

Con esto queda comprobado lo que la 004-R4 dejaba deducido: mezclarlos no estropea los vecinos. Desktop pide siempre `us.`, que tiene precio conocido, y deja que Core pase a `global.` cuando `us.` esté lleno.

### La prueba de 30 preguntas (004-R5)

Una llamada `inputType: query` con las 30 preguntas (490 tokens, 2,6 segundos), contra un vector por documento entero:

| Qué | Cohere Embed V4 | Prueba del 26 oct, Qwen3-Embedding | Búsqueda por palabras |
| --- | --- | --- | --- |
| Esperados entre los 5 primeros | 54 % | 83 % | 50 % |
| Esperados entre los 10 primeros | 78 % | 90 % | 62 % |

**No es una comparación pareja:** el 26 oct se usó un vector por página, promediado por documento; acá, uno por documento entero, y hay documentos de más de 80.000 caracteres. Sospechamos que el vector del documento entero diluye lo que pregunta una persona. Por eso no lo llevamos todavía al chat.

### Para el tipo de documento

- El documento más parecido, entre los que no son de la misma familia, tiene el mismo tipo un 54 % de las veces (contando sinónimos). Sirve como pista para la pregunta, nunca para decidir el tipo sin preguntar.
- Los documentos sin familia muy parecidos entre sí (coseno de 0,85 o más) se pueden preguntar juntos: ahorran un 7,5 % de las preguntas de tipo, sin mezclar tipos.

### Lo que vimos hoy, al ponerlo en la app (2 nov 2026)

Dos textos inventados, en dos llamadas:

- 1.024 números y norma 1, como dice el mensaje.
- **El mismo texto, pedido solo en una llamada y junto con otro en otra, no da exactamente los mismos números:** coseno 0,99988 y una diferencia máxima de 0,0015 por número. Entre los dos textos distintos, el coseno fue 0,50. Para encontrar vecinos no cambia nada; por eso la app guarda cada vector con la huella de su texto y no vuelve a pedir un texto que ya tiene.

## Cómo lo usa Desktop

- **Un paso del proceso,** después de que cada documento tiene su texto y su corte, y antes de las familias y el tipo.
- **Un vector por documento,** con el texto que leyó la app: lo leído con confianza alta, y lo leído con confianza media con una marca; lo que no se pudo leer bien no va. Nunca va el nombre del archivo.
- **De a 96 textos por llamada, una llamada a la vez:** unas 0,013 llamadas por documento. Ninguna pregunta a un modelo de lenguaje.
- **Una copia no pide nada,** un texto que cambió vuelve a pedir y un cambio de modelo vuelve a pedir todo (004-R4).
- **Si Core no responde** (un 503, el cupo, sin conexión), el paso espera y lo vuelve a intentar solo, más espaciado cada vez: a los 10 minutos, a la hora, a las 6 horas y después una vez por día. La persona lo ve en la app después del tercer intento.
- **Los vectores quedan solo en la base de la computadora.**

## Las preguntas

- **C002-Q1 · La variación entre llamadas:** ¿es esperable que el mismo texto dé un vector apenas distinto según con qué otros textos va en el lote? No nos molesta; queremos saber si es del modelo o de cómo se arma el lote.
- **C002-Q2 · El precio de `global.`:** cuando se conozca, ¿lo pueden anotar en el uso del tenant? Hoy los tokens que pasan por `global.` quedan sin costo estimado.
