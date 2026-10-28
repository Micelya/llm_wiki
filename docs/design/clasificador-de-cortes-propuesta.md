# Propuesta: una prueba acotada de un clasificador chico para decidir si una página sigue el documento anterior o empieza otro

27 oct 2026. Decisión D-610; la aprueba el dueño (P-421). Es una prueba: no elige una arquitectura definitiva ni autoriza un entrenamiento grande.

## 1. La pregunta

¿Un clasificador chico sobre las señales que ya calcula el cortador decide mejor que las reglas los pasos entre páginas? Y si es así, ¿deja menos trabajo para el modelo grande (D-606) y para las personas (D-603), sin separar ni unir mal más documentos?

## 2. Lo comprobado hoy

Medido en la carpeta de prueba, con la base armada desde cero y sin preguntar nada nuevo.

| Qué | Cifra |
|---|---|
| Hoja de partes | 6 PDF, 134 páginas, 128 pasos, 96 cortes reales, 104 documentos |
| Reglas solas, en la hoja | 71 cortes encontrados, 2 de más, 25 perdidos, 57 documentos exactos, 47 pasos dudosos |
| Reglas y modelo grande en los dudosos, en la hoja | 96 encontrados, 3 de más, 0 perdidos, 97 documentos exactos, con 47 consultas (367 s) |
| Toda la carpeta | 130 PDF, 601 páginas, 471 pasos: 17 «sigue» por regla, 141 «empieza otro», 313 dudosos (132 con respuesta del modelo, 181 sin) |
| PDF que la base no corta | 100 PDF (60 de una página) y 201 pasos, todos dudosos |

Dos cosas más:

- **Las reglas solo dicen «sigue» ante una marca explícita:** una página en blanco, «página x de y», un código, una serie o un par de fotos. Las 22 respuestas del modelo en pasos de PDF no cortados dicen «sigue».
- **La hoja de partes ya se usó varias veces para ajustar las reglas.** No es una prueba independiente.

## 3. Qué se compara

Las variantes, sobre los mismos pasos:

| Variante | Qué es |
|---|---|
| A | Las reglas solas (hoy, sin el modelo) |
| B | Las reglas y el modelo grande en todo paso dudoso (lo medido hasta ahora) |
| C | Las reglas y la regla «archivo parejo» (P-420): en un PDF sin ningún corte y de páginas iguales, un paso sin señal es «sigue» |
| D | Las reglas y el clasificador en los pasos dudosos; el modelo grande solo donde el clasificador queda incierto y la regla de D-606 lo permite |

C va antes que D porque es determinista y más barata (D-606). Si C alcanza en los PDF no cortados, el clasificador se mide solo donde C no decide.

## 4. El clasificador

- **Qué es:** un conjunto de árboles de decisión (árboles potenciados por gradiente). Se entrena fuera de la app, en el desarrollo.
- **Dónde funciona:** los pesos van al catálogo del cortador y la app los evalúa en Rust, en el lugar donde hoy un paso queda dudoso (`cut/mod.rs`). Su veredicto se guarda como la regla `classifier`.
- **Sin nada nuevo que instalar:** no necesita ningún motor aparte ni una GPU. ONNX Runtime, que la app ya abre para el motor local, queda solo para un modelo de imagen, si después hiciera falta.
- **Salida:** «sigue», «empieza otro» o «incierto». Dos umbrales elegidos en el entrenamiento, que no se muestran como porcentajes de certeza (D-605).
- **Lo que no reemplaza:** las huellas para los archivos idénticos ni las reglas que ya aciertan. Ninguna señal sola es una regla universal: una firma puede ir seguida de un anexo.

## 5. Las señales (unas 20, por cada par de páginas)

Salen de lo que el cortador ya calcula (`cut/signals.rs` y la ficha de forma de la página):

- **Numeración:** «página x de y», si continúa o vuelve a empezar.
- **Apertura y cierre:** título o fórmula de apertura arriba de la página nueva; fórmula de cierre o firma al final de la anterior.
- **Hechura:** digital, escaneo o captura; tamaño y orientación; resolución y color de la imagen principal; la capa de texto.
- **Parecido entre las dos páginas:** la distancia de la huella perceptual (`phash`), el código y la serie compartidos y el parecido del texto.
- **Distribución:** caracteres y renglones propios, la parte de renglones cortos y lo que cubren las imágenes.
- **Calidad de la lectura:** qué motor leyó la página y, cuando exista (P-415), cuánto se leyó con respaldo.
- **Contexto:** las mismas señales de la página anterior y de la siguiente; la posición del paso y cuántas páginas tiene el archivo.

Ninguna señal es un nombre, una ruta, una huella de archivo ni un texto de un caso: el clasificador no puede aprender estos documentos en particular (R-24 a R-26).

## 6. Los datos

**La etiqueta** de cada paso es «sigue» o «empieza otro», con el criterio de la hoja de partes y D-608:

- un anexo con firmas propias empieza otro documento;
- una constancia de firma empieza otro documento;
- una página en blanco sigue al anterior.

| Conjunto | Pasos | Para qué |
|---|---|---|
| La hoja de partes | 128 | Entrenar. No es prueba independiente |
| Los 24 PDF que la base corta y la hoja no cubre | 142 | Entrenar y validar; no se usaron para ajustar las reglas |
| Los 40 PDF de varias páginas que la base no corta | 201 | Entrenar y validar, sobre todo los «sigue» |
| Documentos y formatos nuevos, de otro origen | unos 100 a 200 | Solo el examen final, una vez |

- **Quién etiqueta:** Claude propone las etiquetas de los 343 pasos sin referencia, con el texto y las señales y, solo donde dude, con hojas de miniaturas. El dueño las comprueba archivo por archivo (P-431). Cada etiqueta guarda quién la dio: el dueño, Claude comprobado por el dueño o la hoja. Una propuesta de un modelo grande es siempre una propuesta que se comprueba.
- **Separación:** se valida por grupos. Cada grupo tiene todos los pasos de un archivo, de sus copias idénticas y de sus versiones (las familias de la base), de modo que el examen no tenga variantes de lo estudiado. El conjunto nuevo se mira una sola vez, al final.
- **El conjunto nuevo** lo elige el dueño: de 10 a 20 PDF de varias páginas, con compuestos, de un origen que el desarrollo no vio.

## 7. Qué se mide (por variante, en la validación por grupos y en el examen nuevo, por separado)

- **Errores:**
  - separaciones de más;
  - documentos distintos unidos (cortes perdidos);
  - documentos con límites exactos (D-609).
- **Lo que se resuelve solo:**
  - la parte de los pasos resueltos sin nadie;
  - los pasos que quedan inciertos;
  - los que irían a «Para mirar»;
  - las consultas al modelo grande.
- **Costo:** tiempo del recorrido entero en esta PC (la ficha de forma, la lectura, las señales y el clasificador, no solo el clasificador), memoria y lo que hay que instalar.
- **Cobertura,** como dice D-609: evaluados, excluidos y su motivo, y sin referencia.

## 8. Criterios para decidir si sirve

Es una propuesta; el dueño puede cambiarlos. Se comparan D (o C y D) contra A y B, en el examen nuevo, y se confirman en la validación por grupos.

1. **Calidad:** menos errores que A. Contra B, a lo sumo 1 error más cada 50 pasos.
2. **Ahorro:** al menos un 70 % menos de consultas al modelo grande que B.
3. **Lo que queda incierto:** menos de la mitad de los pasos que A deja dudosos.
4. **Costo:** menos de 10 ms por paso en esta PC, nada nuevo para instalar y el recorrido completo a lo sumo un 10 % más lento que hoy.

Si no cumple 1 o 2, la prueba se para. Lo aprendido (qué señales separan) va a las reglas o al catálogo. Si cumple todo, se propone llevarlo a la app detrás de una marca del catálogo, con su propia decisión.

## 9. Cuánto cuesta (relativo)

- **El dueño:** comprobar las etiquetas propuestas de unos 64 PDF y elegir el conjunto nuevo. Es lo más caro de la prueba.
- **Claude:** tres pasos de sub-agente:
  - exportar las señales y proponer las etiquetas, sin nombres;
  - entrenar y medir las cuatro variantes;
  - si pasa, el evaluador en el cortador.

  Es del orden de una corrida mediana con tope (0,4 a 0,6 M fichas); proponer etiquetas mirando miniaturas es la parte más cara.
- **Codex, solo en el desarrollo:** B necesita respuesta en todos los pasos dudosos. Hay 132 guardadas; faltan unas 181 de la carpeta, más las de los dudosos del conjunto nuevo, de a una consulta por vez.
