# 005 · Lo que le falta a la lectura con análisis para los formularios y los escaneos

- **Número:** 005 · **Fecha:** 29 de octubre de 2026 · **Estado:** abierto
- **De:** Micelya Desktop (`Micelya/llm_wiki`) · **Para:** Micelya Core (`extrimian/mi-platform`, rama `dev`)
- **Respuesta esperada en:** `docs/dropbox/desktop/` del repositorio de Core, con el número 005, punto por punto.
- **Relacionado con:** el mensaje 003 (003-R1 piezas, 003-R3 manuscrito, 003-R4 costo por pieza). Lo de acá no repite esos pedidos: sale de probarlos.
- **Depende de esto en Desktop:** Desktop P-063 (integración con Core), Desktop P-173 (las piezas de un documento), Desktop P-464 (seguir este mensaje).

## Contexto

Desktop mandó 11 páginas a la lectura de Core en desarrollo, con `tables`, `forms` y `signatures` en todas y `pieces` en 4. Eran:

- un formulario digital de alta de cliente, de 3 páginas;
- un recibo con una tabla de cuotas;
- una boleta de pago con una tabla de infracciones;
- 4 páginas escaneadas de un expediente: formularios de registro completados a mano, con sellos y firmas.

Se comparó lo que devolvió Core con las respuestas que Desktop anotó a mano para esas páginas y con la lectura local de Desktop.

**Lo que anduvo bien, para que no se toque:**

- los pares rótulo-valor de un formulario digital;
- las tablas de las páginas digitales: celda por celda, igual que la lectura local;
- la clase amplia de cada pieza (registro civil, constancia, escrito);
- la lectura de manuscrito con un modelo, que marca lo ilegible.

Leer las 11 páginas costó alrededor de US$ 0,80, a precio de lista, y tardó de 3 a 10 segundos por página.

Abajo va lo que falta. Cada pedido lleva su prioridad: (A) la necesitamos para el piloto, (B) mejora mucho, (C) cuando se pueda.

## Pedidos

- **005-R1 (A) · Dónde está cada par rótulo-valor.** Hoy `keyValues` trae la página, el rótulo, el valor y la confianza, pero no la posición ni el orden. Cuando un formulario repite un rótulo, no se sabe a qué sección pertenece cada valor.
  - **Ejemplo:** un formulario tiene «Localidad» en la sección del domicilio particular y otra vez en la del domicilio laboral, con dos valores distintos. Core devuelve los dos pares con el mismo rótulo y sin posición, así que no se sabe cuál es cuál.
  - **Lo que pedimos:** para el rótulo y para el valor, un recuadro como el de los renglones, y el orden de lectura. Si se puede, también la zona de la diagramación a la que pertenece el par, que suele ser la sección.
- **005-R2 (A) · Qué formulario es.** Sin `pieces`, Core da los pares pero no dice qué documento es. Con `pieces` da una clase amplia: registro civil, constancia o certificado, escrito. Para separar y archivar hace falta saber cuál es el formulario.
  - **Ejemplo:** un acta de nacimiento y un acta de defunción salen las dos como «registro civil». Un bono profesional pegado a un escrito sale como «escrito».
  - **Lo que pedimos:** que cada pieza traiga, además de la clase, un subtipo o un nombre corto del documento, como lo diría una persona («acta de defunción», «formulario de alta de cliente»), con su confianza. Si no lo sabe, que diga «sin determinar» en lugar de forzar uno.
  - **Una alternativa que también nos sirve:** que Desktop mande la lista de subtipos que espera para ese vertical y Core elija uno de ella.
- **005-R3 (A) · Marcar qué texto es manuscrito y qué es dudoso, por palabra y por zona.**
  - **Lo que hoy da Core:**
    - cada palabra viene solo con su texto y su confidencia;
    - Textract distingue el texto impreso del manuscrito, pero esa marca no llega;
    - la cantidad de palabras manuscritas viene por zona y solo cuando se piden las piezas;
    - la lectura del manuscrito con el modelo marca «[ilegible]», pero sin posición y sin decir a qué renglón corresponde.
  - **Ejemplo:** un formulario impreso donde a mano se completaron el número de trámite y la cantidad de hojas. Desktop necesita saber que esos dos valores son manuscritos y qué tan seguros son, para no fundar un dato en una lectura dudosa.
  - **Lo que pedimos:**
    - en cada palabra y en cada renglón, si es impreso o manuscrito;
    - en la lectura del manuscrito, cada tramo dudoso o ilegible con su recuadro, o al menos con el renglón al que corresponde.
- **005-R4 (A) · Leer con el modelo también lo poco que se completó a mano.** En una de las 4 páginas escaneadas, la diagramación contó 2 palabras manuscritas y la pieza vino sin lectura de manuscrito. Pero en un formulario, lo que se completa a mano es justamente el dato: un número, una fecha, una cantidad de hojas.
  - **Ejemplo:** una carátula impresa con un único número de trámite escrito a mano.
  - **Lo que pedimos:**
    - que en las piezas de tipo formulario o constancia el manuscrito se lea aunque sea poco;
    - o que el umbral se pueda elegir en el pedido;
    - y que el resultado diga cuándo no se leyó y por qué, con un problema como `handwriting_skipped`.
- **005-R5 (B) · El manuscrito separado de lo impreso, y por campo.** La lectura con el modelo devuelve la página entera transcrita, lo impreso y lo manuscrito juntos. Para un formulario completado a mano, sirve más el par «rótulo impreso → valor a mano».
  - **Ejemplo:** en un acta de registro, «Apellido y nombre» seguido de lo escrito a mano.
  - **Lo que pedimos:** que la lectura del modelo devuelva, además del texto, los pares rótulo-valor de lo completado a mano, cada uno con su legibilidad. Esto complementa 003-R3, que pedía leer el manuscrito, y no lo repite.
- **005-R6 (B) · Distinguir una tabla de verdad de la grilla de un formulario, sobre todo en los escaneos.** En las páginas escaneadas, Core devolvió como tablas las grillas de los formularios. En una página dio 22 filas de 2 celdas, todas con una celda vacía, y en algunas celdas la confianza bajó a 0,17. También dio tablas en las páginas de formularios digitales. Las respuestas de Desktop no marcan tablas en ninguna de esas páginas. Las tablas de verdad de las páginas digitales salieron perfectas.
  - **Ejemplo:** un formulario de registro con renglones para completar, donde cada renglón tiene una etiqueta y un espacio vacío.
  - **Lo que pedimos:** que cada tabla diga si es una tabla de datos o la grilla de un formulario, o que las grillas de formulario vayan a los pares y no a las tablas.
  - **Una pregunta:** ¿probaron tablas de datos en páginas escaneadas? En esta prueba no había ninguna.
- **005-R7 (B) · Bloques de firma.** Hoy cada firma trae solo su recuadro y su confianza, y la confianza no separa bien. Una firma manuscrita real salió con 0,42. En una página sin firma, algo salió con 0,29.
  - **Ejemplo:** al pie de un formulario, una firma manuscrita, debajo la aclaración con el nombre y el cargo, y al lado un sello.
  - **Lo que pedimos:**
    - el tipo de firma: manuscrita, de un sistema de firma electrónica, o sello;
    - la aclaración y el cargo que están junto a la firma, si los hay;
    - una guía de qué confianza usar para dar una firma por cierta.

## Preguntas

- **005-Q1 ·** Para 005-R1 y 005-R3: Textract ya da la posición de los pares y el tipo de texto. ¿Es solo agregarlos al contrato, o hay un motivo para no darlos?
- **005-Q2 ·** ¿Qué decide hoy que una pieza se lea con el modelo de manuscrito? ¿Ese umbral se puede pedir desde Desktop?
- **005-Q3 ·** Para 005-R2: ¿prefieren que la lista de subtipos la mande Desktop en el pedido, o que Core tenga la suya por vertical?
