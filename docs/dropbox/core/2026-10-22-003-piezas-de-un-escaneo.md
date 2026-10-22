# 003 · Partir un escaneo en piezas y decir qué es cada una

- **Número:** 003 · **Fecha:** 22 de octubre de 2026 · **Estado:** abierto
- **De:** Micelya Desktop (`Micelya/llm_wiki`) · **Para:** Micelya Core (`extrimian/mi-platform`, rama `dev`)
- **Respuesta esperada en:** `docs/dropbox/desktop/` del repositorio de Core, con el número 003, punto por punto.
- **Relacionado con:** el mensaje 002 (002-R4, leer por piezas y pedir un tratamiento por tipo).
- **Depende de esto en Desktop:** Desktop P-173 (identificar las piezas de un documento), Desktop P-063 (integración con Core).

## Contexto

En los expedientes judiciales es común que un solo PDF escaneado junte cosas muy distintas. Por ejemplo, un escrito con la documentación que acredita una personería: páginas del escrito, fotos de DNI de frente y de dorso, constancias, facturas, partes manuscritas y sellos, a veces dos piezas en una misma página.

Leído todo igual, el resultado es pobre justo en lo que más importa: en la prueba del 22 oct, una cuarta parte de las palabras de un escrito así vino con confianza menor a 0,90, concentrada en las fotos de documentos y los manuscritos.

El dueño de Desktop pidió que **Core haga el corte y la clasificación**: tiene Bedrock (modelos que leen imágenes) además de Textract, y es el lugar donde después se aplica el tratamiento especial de cada tipo (DNI, facturas y otros documentos conocidos), que Core ya está evaluando. Desktop deja de describir imágenes por su cuenta y usa lo que Core devuelva.

## Pedidos

Prioridad: **A** = necesario para usarlo con el estudio de referencia · **B** = mejora importante · **C** = mejora.

- **003-R1 (A) · Identificar las piezas de un documento.** Dado un documento (PDF o imagen), que Core devuelva, por cada página, las piezas que contiene:
  - el tipo de cada pieza, de una lista acordada: escrito o texto corrido, DNI frente, DNI dorso, pasaporte, factura, recibo, constancia o certificado, manuscrito, foto, sello o firma, tabla o formulario, página en blanco, otro;
  - dónde está cada pieza en la página (un recuadro, como el de los renglones);
  - la confianza de la clasificación;
  - si una pieza sigue en la página siguiente (un escrito de varias páginas es una sola pieza).

  Puede ser un trabajo nuevo, con el mismo patrón de cinco pasos que la lectura de documentos, o una opción de la lectura.
- **003-R2 (A) · Leer cada pieza con el tratamiento que le corresponde.** Que el resultado de la lectura venga **por pieza**, cada una con su tipo y su texto, y, para los tipos con tratamiento especial, con sus datos estructurados. Por ejemplo:
  - del DNI: apellido, nombres, número, fecha de nacimiento, ejemplar y vencimiento;
  - de la factura: emisor, CUIT, número, fecha, importes.

  Si Core prefiere que Desktop mande cada pieza por separado con su tipo, que lo diga y cómo se pide (002-R4).
- **003-R3 (B) · Manuscrito en castellano.** Las piezas manuscritas leídas con un modelo que las entienda, si Textract no alcanza, con su confianza y sin inventar lo que no se ve: lo ilegible, marcado como tal.
- **003-R4 (C) · Costo por pieza.** Cuánto cuesta identificar las piezas y leer cada tipo, para mostrarlo antes de pedir.

## Preguntas

- **003-Q1 ·** ¿La lista de tipos les parece bien, o Core propone otra? Desktop la va a mostrar en la pantalla del documento («qué tiene cada página»).
- **003-Q2 ·** ¿Cuándo podría estar algo de esto en el ambiente de desarrollo, aunque sea una primera versión? El producto mínimo de Desktop se está construyendo ahora; Desktop puede empezar con la lectura por página (ya probada) y sumar las piezas cuando estén.
