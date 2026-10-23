# Actualizar Micelya sin perder los datos

**Estado:** construido en la corrida desatendida del 23 oct (D-500), a revisar por el dueño. **Sobre:** el requisito de que una actualización respete los datos de una versión anterior (D-496, P-297).

## En pocas palabras

- **Los datos no viven con el programa.** Están en la carpeta del usuario de Windows, `%APPDATA%\com.micelya.desktop`. Instalar una versión nueva encima reemplaza el programa y deja esa carpeta como estaba.
- **La base cambia de forma sola**, al abrirla con una versión nueva de la app, y siempre hacia adelante (de la versión 24 a la 25, nunca al revés). Hoy está en la versión 25.
- **Antes de cambiarla, la app guarda sola un respaldo**, con la huella de cada archivo. Si no puede guardarlo, no cambia nada y lo dice con palabras: qué pasó, que los datos están intactos y qué hacer.
- **Se conservan los 3 últimos respaldos.** Uno viejo se borra recién cuando hay uno nuevo y la actualización salió bien; el único que haya no se borra nunca.
- **Para volver a la versión anterior:** desinstalar sin marcar «Eliminar los datos de aplicación» (el instalador de hoy lo dice en inglés: «Delete the application data»), instalar la anterior y volver al respaldo de antes del cambio. Se pierde lo que se hizo en la app después de la fecha del respaldo; los documentos de las carpetas no se tocan nunca.
- **Dónde se ve:** Ajustes → «Datos y respaldos».

## 1. Qué hay en la carpeta de datos y qué cambia entre versiones

| Archivo o carpeta | Qué es | ¿Cambia de forma entre versiones? | ¿Va en el respaldo? |
|---|---|---|---|
| `docbase\private.sqlite3` (con `-wal` y `-shm` al lado mientras la app está abierta) | La base: documentos, decisiones, casos, notas, plantillas | **Sí**: tiene número de versión y cambia sola hacia adelante. Una versión vieja de la app no abre una base de una versión más nueva (no la toca y lo dice) | Sí |
| `app-state.json` | Los ajustes | No tiene versión: lo que falta toma su valor de siempre y lo que sobra se ignora. Una sola vez, en una versión anterior, las claves pasaron al llavero de Windows, comprobando cada una | Sí |
| `chat\` | Historial y preferencias del chat | No tiene versión; se lee igual que los ajustes (todavía entiende el formato viejo de LLM Wiki) | Sí |
| `micelya-core-lecturas.json` | Lecturas pedidas a Micelya Core que falta borrar allá | No | Sí |
| `docbase\document-types.json`, `docbase\knowledge.json` (si existen) | Catálogos que reemplazan los de la app | No | Sí |
| `docbase\recognized\` | Texto reconocido de los escaneos, página por página | No, y ninguna actualización lo toca | No |
| `agent\` | Lo que produjo el agente del chat | No | No |
| `respaldos\` | Los respaldos | — | — |
| `%LOCALAPPDATA%\com.micelya.desktop` | El navegador interno y las sesiones de los portales | No | No |

**Conclusión:** solo la base cambia de forma de una manera que una versión anterior no entiende. Los ajustes, el chat y los catálogos van igual en el respaldo, porque son chicos y así volver deja todo como estaba si alguna vez cambian.

## 2. Qué hace la app sola al actualizarse

Cuando la app se abre:

1. Lee la versión de la base.
2. **Si es la que conoce:** sigue, sin respaldo.
3. **Si es más vieja:** antes de tocarla, guarda un respaldo en `respaldos\antes-de-v25-2026-10-24-0930\` (la versión a la que va y la fecha y hora). La copia la hace la base misma, no una copia del archivo abierto: así sale completa aunque la app la esté usando. Antes de seguir, comprueba la copia: que abra, que esté en la versión de la base, que esté sana y que cada tabla tenga las mismas filas. En la carpeta del respaldo quedan:
   - `docbase\private.sqlite3`, `app-state.json`, `chat\…`: los datos tal como estaban, en el mismo lugar que tienen en la carpeta de datos;
   - `huellas.sha256`: la huella SHA-256 de cada archivo;
   - `respaldo.json`: qué versión de la base tiene, a cuál iba, qué versión de la app la había escrito, cuándo se hizo;
   - `LEEME.txt`: qué es y cómo volver a él a mano, en castellano.
   Mientras se está haciendo, la carpeta se llama `sin-terminar-…`: si la app se cierra a mitad, eso no cuenta como respaldo y se borra la próxima vez.
4. **Si el respaldo no se puede hacer** (por ejemplo, el disco está lleno): no toca la base. En Inicio y en Ajustes se lee: «Antes de actualizar la base de la versión 24 a la 25, Micelya tiene que guardar un respaldo y esta vez no pudo (…). No se cambió nada: tus datos están intactos, como los dejó la versión anterior de la app. Qué hacer: fijate que haya lugar libre en el disco, cerrá Micelya del todo y volvé a abrirla. Si vuelve a pasar, avisá a soporte con este mensaje.»
5. **Con el respaldo hecho, actualiza la base.** Si la actualización falla, lo dice nombrando el respaldo («Tus datos están en el respaldo «…», hecho justo antes») y no borra ningún respaldo viejo.
6. **Si salió bien:** deja los 3 respaldos más nuevos y borra los anteriores. Solo borra carpetas que hizo la app (con su `respaldo.json`); una copia hecha a mano dentro de `respaldos` no se toca.
7. **Si la base es de una versión más nueva** (alguien volvió a instalar una versión anterior): no la toca y dice: «Esta base la escribió una versión más nueva de Micelya (…). No se tocó nada. Para seguir con esta versión, volvé al respaldo de antes de la actualización en Ajustes → Datos y respaldos, o instalá de nuevo la versión más nueva.»
8. Anota en la base qué versión de la app la abrió, para que el próximo respaldo diga quién la había escrito. Esto no cambia la forma de la base.

**Ajustes → Datos y respaldos** muestra: dónde están los datos y los respaldos, con un botón para abrir cada carpeta; la versión de la base y la que usa esta versión de la app; qué hizo la app al abrirse (la actualización, su respaldo, los viejos que borró); la lista de respaldos con su fecha, su versión y su tamaño; y el aviso de no marcar «Eliminar los datos de aplicación» al desinstalar. Si la base no se pudo abrir, dice por qué; si es de una versión más nueva, ofrece «Volver a este respaldo».

La copia tarda poco con una base como la de hoy (unos segundos); con una base mucho más grande, la app tarda más en abrir la primera vez después de una actualización, sin mostrar el avance.

## 3. Cómo probar mañana con el instalador

La rama `main` tiene la base en la versión 24 y no hace respaldos; la rama de esta sesión (`feat/session-18`) la tiene en la 25 y los hace. Eso sirve para la prueba:

- **Instalador A, «la versión anterior»:** hecho desde `main`.
- **Instalador B, «la nueva»:** hecho desde `feat/session-18` (con todo lo de esta noche).

Conviene que B tenga un número de versión más alto que A (por ejemplo 0.1.1; hoy las dos dicen 0.1.0): así el instalador la reconoce como actualización y no pregunta si reinstalar. Usar siempre el mismo tipo de instalador (el `.exe`, recomendado; no mezclar con el `.msi`).

**No probar con los datos reales.** La app instalada usa la misma carpeta de datos que la app del dueño. Hacer la prueba en otra cuenta de Windows (o en otra computadora), donde `%APPDATA%\com.micelya.desktop` no exista.

1. Instalar A. Abrirla, agregar una carpeta con documentos inventados y esperar a que los lea. Tomar algunas decisiones: confirmar un tipo de documento, crear un caso y ponerle un documento, escribir una nota en una ficha, nombrar una plantilla. Anotar cuántos documentos dice Inicio. Cerrar la app del todo (ícono junto al reloj → salir).
2. Instalar B encima, sin desinstalar A. Abrirla.
3. Comprobar:
   - Inicio dice los mismos documentos; las decisiones, el caso, la nota y el nombre de la plantilla están.
   - Ajustes → Datos y respaldos dice «Al abrir, la app actualizó la base de la versión 24 a la 25. Antes guardó el respaldo «antes-de-v25-…»» y lo muestra en la lista.
   - «Abrir la carpeta de respaldos» muestra esa carpeta, con `docbase\private.sqlite3`, `huellas.sha256`, `respaldo.json` y `LEEME.txt`.
   - Optativo, la huella: en PowerShell, dentro de la carpeta del respaldo, `Get-FileHash -Algorithm SHA256 docbase\private.sqlite3` tiene que dar lo mismo que la línea de `docbase/private.sqlite3` en `huellas.sha256` (en minúsculas).
4. Cerrar y volver a abrir B: no hace otro respaldo (la base ya está en la 25).
5. Si se quiere probar también la vuelta atrás, seguir la sección 4 con A.

## 4. Cómo volver a la versión anterior

1. Cerrar Micelya del todo (ícono junto al reloj → salir).
2. Por las dudas, copiar la carpeta `respaldos` (en `%APPDATA%\com.micelya.desktop`) a otro lugar, por ejemplo a Documentos.
3. Desinstalar Micelya desde Configuración → Aplicaciones. **No marcar «Eliminar los datos de aplicación»** (en el instalador de hoy, en inglés: «Delete the application data»): borra los datos y también los respaldos. Con el `.msi` esa casilla no existe y los datos quedan siempre.
4. Instalar la versión anterior.
5. Volver al respaldo de antes del cambio de forma:
   - **Si la versión anterior ya tiene «Datos y respaldos»** (esta y las que vengan): abrirla, ir a Ajustes → Datos y respaldos, tocar «Volver a este respaldo» en el más reciente, confirmar, tocar «Cerrar Micelya» y abrirla de nuevo.
   - **Si es más vieja que esta** (como el instalador A, hecho desde `main`), a mano, como dice el `LEEME.txt` del respaldo: con Micelya cerrada, en la carpeta de datos abrir `docbase`, mover a otra carpeta `private.sqlite3`, `private.sqlite3-wal` y `private.sqlite3-shm` (los que haya; no borrarlos), copiar en `docbase` el `private.sqlite3` de la carpeta `docbase` del respaldo, y abrir Micelya.
6. **Qué se pierde:** todo lo que se hizo en la app después de la fecha del respaldo (documentos que leyó, decisiones, casos, notas, plantillas nombradas). Los documentos de las carpetas de origen no se tocan nunca: la base vuelve a encontrar lo que haya ahí y lo lee de nuevo. La base que estaba en uso no se borra: el botón la deja en `respaldos\apartada-…`, y a mano queda donde se la movió.
7. Los ajustes y el chat no vuelven solos: la versión anterior los lee como están. Si hiciera falta, la copia de cómo estaban está en la carpeta del respaldo.

## 5. Decisiones asumidas (D-500), a revisar

- **Se conservan los 3 últimos respaldos.** Alternativa: más (cada uno ocupa lo que la base) o por antigüedad.
- **Los respaldos van dentro de la carpeta de datos**, como pedía la especificación. Riesgo: «Eliminar los datos de aplicación» al desinstalar los borra junto con los datos; por eso el aviso en Ajustes y el paso 2 de la sección 4. Alternativa: guardarlos en Documentos, que el desinstalador no toca.
- **El respaldo lleva la base, los ajustes, el chat, las lecturas de Core y los catálogos**, no el texto reconocido de los escaneos ni lo del agente ni el navegador interno (no cambian de forma y ninguna actualización los toca).
- **«Volver a este respaldo» solo aparece cuando esta versión no puede abrir la base** porque la escribió una más nueva. Con la base abierta y en uso no se ofrece: volver ahí no tendría sentido (la misma versión la actualizaría de nuevo al abrir) y cambiar la base debajo de la app es riesgoso. Alternativa: un botón siempre disponible que cierre la app y cambie la base al abrirla.
- **Después de volver, la app pide cerrarla y abrirla a mano** (con un botón para cerrarla), en lugar de reiniciarse sola: el reinicio automático choca con el mecanismo que evita abrir dos copias de la app y podría dejarla cerrada.
- **Nombres en castellano** para las carpetas y archivos del respaldo (`respaldos`, `antes-de-v25-…`, `huellas.sha256`, `respaldo.json`, `LEEME.txt`, `apartada-…`).
- **La base anota la versión de la app que la abrió**, como un ajuste más de la base (no es un cambio de forma), para que el respaldo diga qué versión instalar para volver.
- **La prueba automática cubre todas las versiones anteriores**, de la 1 a la 24, no solo tres.

## 6. Riesgos y lo que no cubre

- Desinstalar marcando «Eliminar los datos de aplicación» borra datos y respaldos (sección 5).
- Cada respaldo ocupa más o menos lo que la base: con tres, el triple. Si el disco se llena, la app no actualiza y lo dice.
- La primera apertura después de una actualización tarda lo que tarda la copia, sin mostrar el avance.
- El respaldo está en el mismo disco: no reemplaza una copia en otro lugar si se rompe el disco.
- La prueba con el instalador real (sección 3) es a mano; la prueba automática cubre la base, no el instalador.
- La vuelta con el botón existe desde esta versión; para volver a una más vieja (como `main`), es a mano.

## 7. Para quien programa

- El código: `src-tauri/src/docbase/upgrade.rs` (respaldo, huellas, retención, volver a un respaldo) y `DocBase::open_in_data_folder` en `src-tauri/src/docbase/mod.rs`, que es la única puerta por la que la app abre su base. `DocBase::open` queda para las pruebas y las comprobaciones sobre copias, sin respaldos.
- **Agregar una versión de la base no pide tocar nada de esto:** se agrega la entrada en `MIGRATIONS` (`schema.rs`) y listo. El respaldo no depende de qué versión viene, y la prueba (`upgrade_tests.rs`) arma una base en cada versión anterior con filas inventadas en todas sus tablas, leyendo de la base misma sus columnas, los valores que admiten sus controles y las filas que piden sus claves; después la abre por la misma puerta que la app y comprueba que no se perdió ni cambió ninguna fila (las decisiones de personas, exactas), que la base quedó sana y que el respaldo es la base como estaba y coincide con sus huellas.
- Si una versión nueva **cambia a propósito** datos que ya estaban, la prueba falla con la tabla y las filas: se anota en `CHANGED_ON_PURPOSE` (`upgrade_tests.rs`) con la versión, la tabla y el porqué.
- Si una tabla nueva pide un valor que la prueba no sabe inventar, la prueba dice cuál: se le enseña en `fill`.
