# Sesión 20 — 26 de octubre de 2026: cierre y traspaso

Para retomar: este documento, `CLAUDE.md` (sobre todo «Guardrails de consumo de Claude» y «Pruebas en la nube») y `pendientes.json`. El traspaso anterior es [2026-10-25-sesion-19-cierre.md](2026-10-25-sesion-19-cierre.md).

**Datos reales:** todo lo que tiene nombres, números o textos reales está fuera del repositorio, que es público: en `C:\Users\Usuario\LocalRepo\llm_wiki_mi\` (en la PC del dueño) y en la carpeta de Drive de pruebas (sección 7).

## 1. En una frase

Se armó el entorno de las pruebas en la nube:
- la base completa de la app está en Drive, y se vio funcionar desde una sesión en la nube;
- hay un instructivo para los agentes en la nube, y un guion que busca datos reales antes de subir una rama;
- se revisaron las tres pruebas en la nube del 26 oct y se abrieron sus pedidos de integración;
- se juntaron las dos ramas de otras sesiones (el borrador del replanteo y la forma de traer archivos de Drive) y se les sacaron dos ejemplos con datos reales.

## 2. Estado al cerrar (26 oct, noche)

| Qué | Estado |
|---|---|
| `main` | Igual que al cerrar la sesión 19 (el #71 integrado). |
| Pedido **#72** (rama `integracion/replanteo-y-drive`) | **Abierto.** El borrador del modelo de conocimiento y el inventario de la base, las decisiones D-530 a D-547, los pendientes, la base en Drive, el instructivo de la nube, la búsqueda de datos reales y este traspaso. Lo integra el dueño. |
| Pedidos **#73, #74 y #75** | **Abiertos.** Las pruebas en la nube de huellas y embeddings, del mismo documento en dos lugares y de manuscritos. No se tocan entre sí. Los integra el dueño. |
| Ramas `claude/confident-dijkstra-v0e55z` y `claude/loving-keller-xguuoe` | Todavía en GitHub: el agente no tiene permiso para borrarlas. Se borran después de integrar el #72 (P-367). Hay una copia de las dos en la PC (ramas locales `respaldo/…`). |
| Rama `config/permiso-paddleocr-vl` | Borrada, a pedido del dueño. |
| `llm_wiki_wt` | Desde ahí corre la app, en `fd1923d` (ya en `main`). `src-tauri/Cargo.toml` figura modificado solo por el fin de línea: restaurarlo con la app cerrada. |
| La base real | Versión 29. **La lectura automática sigue apagada** desde el 25. No se tocó: solo se leyó con la copia propia de SQLite. |
| Caso nuevo del PJN | Todavía no está en la carpeta de trabajo de la app: ahí sigue solo el expediente anterior. |
| Pendientes | 189 abiertos: 103 por hacer, 58 a verificar, 24 en espera, 4 en curso. De Claude: 4, 46 y 51 (prioridades 1, 2 y 3). |

## 3. Lo logrado

**La base en Drive (D-547; llevar la base a Drive, P-358, hecho):**
- `herramientas\a-drive.py`, en la PC:
  - hace la copia propia de SQLite, que no frena la app, la controla y la comprime (42 MB en 12 MB);
  - la deja en `pruebas\base\` de Drive con su ficha (fecha, versión, commit de la app, huella, filas por tabla y dónde está cada origen);
  - rehace las planillas de `estado-de-la-app` y copia los documentos nuevos o cambiados;
  - primero se corre en seco. Nunca escribe fuera de POC-Micelya de Drive ni borra ahí. La historia queda en `llm_wiki_mi\copias-de-la-base\`.
- Primera copia: 26 oct 22:10. **El dueño la vio funcionar desde la nube:** bajó con el enlace, la huella coincidió, la base abrió en la versión 29 con 198 ubicaciones.

**Para los agentes en la nube:**
- **Bajar:** `docs/plantillas/bajar-de-drive.py` baja solo las subcarpetas pedidas, por el enlace compartido, y comprueba la huella de la base.
- **El instructivo,** que sigue todo agente en la nube antes de una prueba, como dice `CLAUDE.md`: `docs/plantillas/pruebas-en-la-nube.md`. Dice cómo bajar, comprobar y usar la base, cómo apuntar los orígenes a lo bajado, la búsqueda de datos reales y la entrega.
- **Buscar datos reales:** `docs/plantillas/revisar-datos-reales.py` compara las líneas que una rama agrega con estas listas:
  - los nombres de personas, organizaciones y casos de la base;
  - la lista de palabras sensibles, que ahora también está en Drive, en `pruebas\revision-de-datos\`;
  - las hojas de respuestas.

  También busca números con forma de DNI o CUIT, correos, teléfonos y enlaces de Drive. **En la PC también sirve:** se le pasa la carpeta de Drive sincronizada (`G:\Mi unidad\…\POC-Micelya`) como carpeta de datos.
- **Cómo traer archivos de Drive y entregar resultados al dueño** (D-546): la sección de `CLAUDE.md`, que vino de una sesión en la nube. Se renumeró: usaba el número de la primera decisión del replanteo.

**Las tres pruebas en la nube del 26 oct** (sin datos reales en sus ramas):
- **Huellas y embeddings (#73):**
  - la huella de la imagen de la página (pHash) es la mejor para reconocer páginas iguales;
  - Qwen3-Embedding-0.6B encuentra el 83 % de lo esperado entre los 5 primeros, contra el 50 % de la búsqueda por palabras;
  - lo mejor es combinar: candidatos por embeddings, confirmados con pHash y datos clave.
- **El mismo documento en dos lugares (#74):**
  - el método de la app acierta 53 de 57;
  - lo que falta es partir los compuestos antes de comparar;
  - pHash sirve solo para desempatar.
- **Manuscritos (#75):**
  - Qwen3-VL-2B lee 25 de 37 valores manuscritos (21 exactos), contra 12 de RapidOCR, a unos 5 minutos por página y 12,7 GB;
  - PaddleOCR-VL es el mejor en lo impreso, pero mucho más lento;
  - se anotó en la lectura de lo manuscrito, uno por uno (P-334): es el candidato, y lo decide el dueño porque cambia D-518.

**El replanteo (#72):** el borrador del modelo de conocimiento, el inventario de la base y las decisiones D-530 a D-545 vienen de otra sesión del dueño. **Dos ejemplos del borrador tenían datos reales** (una persona con su empleador y fechas, que el dueño había dado como ejemplo en la conversación, y el nombre de un cliente). Se cambiaron por ejemplos inventados.

**Orden en la PC:**
- los resultados de las cuatro pruebas en la nube están en `llm_wiki_mi\resultados-nube\`, con su `LEEME.md` y las huellas;
- se borró `pruebas-cloud`, que estaba dentro de POC-Micelya, un origen de la app;
- `C:\Documentos temporales para borrar\POC-Micelya-Pruebas` quedó vacía.
- Tres hojas de respuestas nuevas, que salieron de la prueba de Docling, están en `conjunto-prueba\` y en Drive:
  - `formulario-01.json`: 27 pares rótulo-valor;
  - `impreso.json`: dos trozos impresos transcritos;
  - `manuscrito.json`: 37 valores.

## 4. Lo que espera al dueño

1. **Integrar el #72** (trae este traspaso) **y los #73, #74 y #75.**
2. **Borrar las dos ramas `claude/…`** después del #72 (P-367). Si quiere que el commit con el ejemplo real desaparezca del todo, se le pide al soporte de GitHub.
3. **Revisar el borrador del modelo de conocimiento** (P-360) y decidir sobre la lectura de lo manuscrito (P-334).
4. **Bajar el caso nuevo del PJN** y, si va a una prueba en la nube, pedir que se actualice Drive.
5. **Ver en la app lo construido:** los 58 pendientes «a verificar».
6. **Siguen abiertos de antes:**
   - la segunda tanda de observaciones;
   - el mensaje 004 a Core (borrador en `llm_wiki_mi\desatendido-9\core-004-borrador.md`);
   - la cuenta de prueba de una persona para el inicio de sesión en Core.

## 5. Lo que sigue para el agente local

1. **Escribir el diseño del proceso replanteado** (P-356) con:
   - el borrador del modelo y la revisión del dueño (P-360);
   - las notas del replanteo;
   - los resultados de las tres pruebas.

   Absorbe la propuesta de compuestos (P-350).
2. **Cuando el dueño lo pida, actualizar Drive:** `python -I herramientas\a-drive.py` en seco, después con `--copiar`.
3. **Revisar cada rama que llegue de la nube** con `revisar-datos-reales.py` antes de abrir su pedido de integración, y mirar cada coincidencia.
4. **Pendientes de código que no necesitan decisión:**
   - P-355: la revisión acotada del corte;
   - P-337: la revocación con los datos de validación del propio archivo;
   - P-333: el caso del portal «creado a mano» y la carátula;
   - P-328: las siglas de normas tomadas como organizaciones.
5. **Restaurar `src-tauri/Cargo.toml` en `llm_wiki_wt`** con la app cerrada: `git checkout -- src-tauri/Cargo.toml`.

## 6. Cómo trabajar (lo aprendido en la sesión)

- **Los resultados de una prueba en la nube se bajan directo a `llm_wiki_mi\resultados-nube\`,** nunca dentro de POC-Micelya: es un origen de la app y los sumaría a la base.
- **Los ejemplos que el dueño da en la conversación pueden ser datos reales.** Un agente en la nube los puso en un documento de diseño. Por eso la búsqueda de datos reales mira también los documentos de diseño, y compara con todos los nombres de la base, no solo con los de los seis PDF.
- **Antes de numerar decisiones o pendientes, mirar todas las ramas abiertas:** dos sesiones usaron el mismo número de decisión.
- **Permisos:** el agente local no puede integrar pedidos ni borrar ramas de GitHub (el control de permisos lo frena); lo hace el dueño.
- **Bajar de Drive por el enlace es rápido y no gasta fichas:** la base (12 MB) baja en unos 10 segundos. El conector, en cambio, pasa el contenido por el contexto.
- **`herramientas\pend.py` pone ahora la fecha del día** en lo que agrega o cambia.

## 7. Dónde está cada cosa

**En la PC del dueño:**
- `C:\Users\Usuario\LocalRepo\llm_wiki_mi\`:
  - `resultados-nube\`: lo que entregó cada prueba en la nube, comprimido, con `LEEME.md` y `huellas.json`;
  - `conjunto-prueba\`: las hojas de respuestas y la lista de palabras sensibles;
  - `copias-de-la-base\<fecha>\`: cada copia de la base que se subió a Drive, con su ficha;
  - `herramientas\`, con su `LEEME.md` (ahora con `a-drive.py`).
- Ramas locales `respaldo/confident-dijkstra` y `respaldo/loving-keller` en el repositorio: las ramas originales de las otras sesiones, tal como llegaron.

**En Google Drive** (`Mi unidad\@Temporal - Se puede borrar\Desarrollo Micelya\POC-Micelya\`, con `pruebas\LEEME.md`):
- `pruebas\base\`: la base y su ficha;
- `pruebas\revision-de-datos\`: la lista de palabras sensibles;
- `pruebas\hojas-de-respuestas\`, ahora con las tres hojas nuevas;
- `pruebas\replanteo\`: un archivo de otra sesión, que no está descrito en la guía.

**Reglas:** el agente local puede escribir en toda la copia de POC-Micelya en Drive, nunca fuera de ella. `C:\Documentos temporales para borrar\POC-Micelya` sigue siendo de solo lectura. El enlace de la carpeta nunca va al repositorio.
