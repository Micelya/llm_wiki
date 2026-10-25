# Prueba en la nube: huellas de página y embeddings para encontrar duplicados, contenedores y relaciones

Prompt para un agente de Claude Code que corre **en la nube** sobre el repositorio `Micelya/llm_wiki`. Lo escribió el agente local el 25 oct 2026, a pedido del dueño del producto. El dueño no es programador: escribile en castellano simple.

## Antes de empezar

1. **Leé `CLAUDE.md`, sobre todo «Guardrails de consumo de Claude».** Esta prueba tiene un **tope de 400 mil fichas de Claude en total.**
   - Antes de empezar, escribí tu plan con un tope por parte.
   - Si llegás al tope, cortá y entregá lo que tengas.
   - Mirar imágenes es lo más caro: evitalo salvo para juzgar una duda puntual.
2. **Leé `docs/design/replanteo-del-proceso-notas.md`:** es para qué sirve esta prueba.
3. **Tus datos están solo en Google Drive,** en `Mi unidad\@Temporal - Se puede borrar\Desarrollo Micelya\POC-Micelya`.
   - Leé primero `pruebas\LEEME.md`: dice qué es cada cosa.
   - **Tienen datos reales:** no los subas a ningún servicio externo de procesamiento y nunca los copies al repositorio, que es público.
   - Copialos a una carpeta de trabajo fuera del repositorio, por ejemplo `~/prueba-huellas/`.

## Qué queremos saber

Queremos saber cuánto ayudan dos técnicas, **sin modelos de lenguaje grandes y corriendo en una PC:**
- a reconocer el mismo documento aunque esté suelto en una carpeta y también dentro de un PDF que junta varios;
- a encontrar documentos duplicados, versiones y documentos relacionados;
- a clasificar documentos y a buscar.

## Parte A · Huellas de página (sin modelo)

Para cada página de cada PDF de `POC-Micelya` (sin `pruebas\`) y de `pruebas\expedientes-pjn\`, calculá:
- **la huella exacta del contenido de la página:** el flujo de contenido y sus recursos, por ejemplo con pypdfium2 o pypdf;
- **la huella de la imagen de la página:** la página dibujada en chico y una huella perceptual, como dHash o pHash. Decí qué tolerancia usás;
- **la huella del texto normalizado.**

Con eso, medí:
1. **Qué documentos sueltos aparecen también dentro de otro PDF,** y en qué páginas. Contalos por carpeta, en especial en `caso-ps` frente a `expedientes-pjn`.
2. **Contra `pruebas\hojas-de-respuestas\partes.json`:**
   - de los documentos incluidos en los seis PDF de prueba, cuántos tienen una aparición suelta en las carpetas;
   - si la huella encuentra los cortes: dónde termina lo que coincide con un documento suelto.
3. **Contra `pruebas\hojas-de-respuestas\respuestas.json` («Copias y versiones»):** cuántos pares de «el mismo documento» o «uno dentro de otro» encuentra cada huella, y cuántos falsos.
4. **Qué huella falla y cuándo:** documentos vueltos a escanear, PDF vueltos a guardar, páginas con sellos o firmas agregadas.

## Parte B · Embeddings, en la PC

- **Elegí uno o dos modelos de embeddings abiertos que corran sin conexión en el procesador,** y que anden en castellano.
  - Verificá la licencia del código y del modelo en su fuente: tiene que servir para vender una app con licencia GPL v3.
  - Anotá el tamaño, el tiempo por página y por documento, y la memoria.
- **Calculá los embeddings por documento y por página** (y por documento incluido de `partes.json`), con el texto que saca pypdfium2.
  - Para las páginas escaneadas sin texto, decí qué hiciste. Podés usar RapidOCR, que ya se probó (`docs/investigacion/docling/informe.md`).
- **Medí:**
  1. **«Copias y versiones»:** de los pares de `respuestas.json`, cuántos aparecen entre los vecinos más cercanos, y cuántos vecinos cercanos no son pares.
     - **Ojo con la trampa jurídica:** dos contratos casi iguales con otra parte, otro monto u otra fecha son documentos distintos. El embedding los ve casi idénticos. Medí cuántos de esos casos hay y decí cómo se separarían (por ejemplo, combinando con los datos clave).
  2. **Casos:** si los documentos de un mismo caso quedan cerca. Compará con `pruebas\estado-de-la-app\documentos-de-la-base.csv` (columna `casos_app`), que **no es una verdad**, solo lo que la app dice hoy.
  3. **Tipos de documento:** clasificar por los vecinos más cercanos, dejando cada documento afuera por turno. Compará con `tipo_app`, que tampoco es una verdad.
  4. **Búsqueda:** si `pruebas\hojas-de-respuestas\preguntas.json` trae preguntas con sus documentos esperados, compará la búsqueda por embeddings con una búsqueda por palabras (BM25) en esas preguntas.
- **Combinación:** probá huella de página más embedding, por ejemplo la huella para lo exacto y el embedding para proponer candidatos, y decí si mejora.

## Reglas

- **La hoja de respuestas mide, no enseña (R-25):** no ajustes umbrales mirando casos de la hoja. Si un umbral cambia, que sea por una razón general, y decila.
- **No inventes resultados:** lo que no se pudo probar, decilo y explicá por qué.
- **Nada de datos reales en el repositorio:**
  - ni nombres o apellidos de personas;
  - ni números de documento, CUIT, expediente, teléfonos o correos;
  - ni domicilios;
  - ni texto copiado de los documentos.

  Para referirte a algo, usá el número de contenido de `documentos-de-la-base.csv`, o una descripción genérica. Antes del commit, buscá esos patrones en tus archivos y decí en el informe que lo hiciste.

## Qué entregar

1. **Fuera del repositorio:** lo que sacó cada técnica, con datos reales. Si podés escribir en Drive, en `POC-Micelya\pruebas\resultados-huellas-y-embeddings\`; si no, en tu carpeta de trabajo.
2. **En el repositorio, en una rama nueva `investigacion/huellas-y-embeddings`** creada desde `main`, todo dentro de `docs/investigacion/huellas-y-embeddings/`:
   - `informe.md`, en castellano simple, con:
     - **En resumen,** en cinco renglones;
     - una tabla por parte, con lo medido;
     - **el costo en la PC:** tiempo, memoria y tamaño;
     - **las licencias;**
     - **la recomendación:** qué conviene para cada uso (el mismo documento en dos lugares, duplicados, versiones, contenedores, relaciones, tipos y búsqueda), qué corre en la PC y en qué orden lo construiríamos;
     - lo que no se pudo probar, y por qué;
     - **las fichas de Claude que usaste.**
   - `metricas.json`: solo números.
   - `guiones/`: tus guiones, sin rutas personales ni datos.

   Un solo commit, con el mensaje en inglés y una línea `Co-Authored-By` tuya al final. Hacé push de la rama. **No abras un pedido de integración:** lo revisa el agente local antes.
