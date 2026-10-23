# Portales judiciales en la app: navegador asistido — propuesta

**Estado:** propuesta del 21 de octubre de 2026 · decidido por el dueño el 21 de octubre (D-446) · construido el 22 de octubre para el PJN (página «Portales», D-447 a D-449) · primera prueba real con el dueño el 22 de octubre: después de arreglar el desafío bloqueado y la consulta que no terminaba (D-480, D-491), funcionó de punta a punta · el 23 de octubre, en la corrida desatendida, se corrigió el lector de la tabla, actualizar corta en la página conocida, las descargas no piden permiso y la ventana se oculta con el avance a la vista (D-503): falta que el dueño lo vea (P-298) · puesto al día el 23 de octubre de 2026
**Alcance:** traer a la base documental los expedientes y sus actuaciones desde portales judiciales, empezando por el Sistema de Consulta Web del Poder Judicial de la Nación (SCW, `https://scw.pjn.gov.ar/scw/home.seam`), sin saltear los controles del portal. Pendiente P-248.

## 1. Lo que sabemos del portal

**Verificado leyendo la página pública y su manual** (21 oct, sin pasar el desafío):

- Es una aplicación web clásica (JSF/Seam): formularios con un estado de sesión, identificadores de elementos generados que no son estables entre versiones.
- **Consulta pública:** por expediente (jurisdicción, número y año), por parte y de Reparación Histórica. Antes de consultar hay que resolver un **desafío propio del PJN** (un recuadro dentro de la página con el botón «Ver desafío»).
- **Consulta de usuarios registrados:** «Iniciar sesión» con identificación y contraseña (los letrados y partes, con su CUIT o CUIL y la contraseña del sistema de usuarios del PJN; los no letrados, con correo y contraseña).
- **Restricciones:** los expedientes de Penal y Familia solo los ven los usuarios registrados que intervienen en la causa; los restringidos por el tribunal no se ven por ningún medio.
- **No hay una API oficial** para consultar o descargar actuaciones; solo instructivos del portal web.

**Lo que mostraron las dos pruebas del dueño con un navegador** (21 oct):

- **Formulario:** jurisdicción (una lista), número y año; el desafío antes de «Consultar». La sesión va en cookies y cada consulta lleva su propio identificador en la dirección.
- **El expediente** abre en su propia página. Las actuaciones están en una tabla de **15 filas por página**, que cambia de página sin recargar la pantalla. Columnas: fecha, tipo, detalle y fojas (por ejemplo «12 / 14»), y en las que tienen documento, **«Descargar»**, que devuelve el PDF directamente con la sesión. Las filas sin documento (por ejemplo «MOVIMIENTO») no se bajan. «Ver» abre un visor que no sirve para bajar. «Ver históricas» es otra página.
- **Paginación:** una ventana móvil de unos 10 números; la página actual no es un enlace; las flechas no tienen texto, solo un ícono con su nombre al pasar el mouse («Primera página», «Anterior», «Siguiente», «Última página»). Se recorre con «Siguiente» hasta que desaparece o la página deja de cambiar; **nunca por número**.
- **Dos tablas paginadas en la misma página:** debajo de las actuaciones está «Notas», que dice quién consultó el expediente, con su identificación fiscal. El paginador correcto se busca siempre a partir de la tabla de actuaciones (la que tiene la columna «A FS.»). **Las Notas no se guardan nunca.**
- **Volumen:** hay expedientes con más de 20 páginas de actuaciones y cientos de PDF, algunos grandes, y con vinculados (acumulados, incidentes).
- **Reanudar:** con ese volumen, un recorrido se corta (el desafío que vuelve, la sesión que vence, la app que se cierra); hay que poder retomarlo sin bajar dos veces lo mismo.
- **PDF protegidos:** el portal marca los documentos como protegidos; puede que no se les pueda sacar el texto. La base ya lee los PDF que solo tienen permisos y se abren sin contraseña; si un PDF no trae texto, va al reconocimiento de escaneos (usa cupo). Falta comprobarlo con documentos reales del portal.

**Lo que mostró la primera prueba real con la app** (22 oct): el desafío vive en un recuadro de otro origen dentro de la página; el servidor del PJN redirige a una dirección `http` antes de la `https`; la tabla va de la actuación más nueva a la más vieja, escribe el día sin cero («7/09/2025») y cada celda empieza con un rótulo que solo lee un lector de pantalla; los botones van en la primera celda y las filas MOVIMIENTO y EVENTO no tienen documento ni fojas. Qué cambió por eso, en la sección 3 y en la sección 7.

**Sigue sin verificar:** cada cuánto vuelve a pedir el desafío (por consulta, por sesión, por tiempo); cómo es «Mis causas» con la sesión de un letrado; si los PDF protegidos dejan sacar su texto.

## 2. Principios

1. **Nunca saltear los controles del portal.** El desafío lo resuelve una persona; no se usan servicios ni técnicas para resolverlo o evitarlo, ni se disfraza la app. Si el portal vuelve a pedirlo, la app se detiene y avisa.
2. **La app maneja el portal; la persona interviene solo cuando hace falta** (D-446): para el desafío o para iniciar sesión. Fuera de eso, la ventana solo sirve para recorrer las causas, detectar lo nuevo y bajarlo.
3. **El abogado inicia sesión él mismo, en la página del portal.** La app no pide ni guarda su contraseña (D-447). La sesión queda en un perfil del navegador propio de ese portal, como en cualquier navegador.
4. **El portal no tiene ningún acceso a la app.** La ventana no tiene permisos de la app, solo puede ir a direcciones del portal y la página nunca le manda nada a la app: es la app la que lee la página (D-447). Tampoco puede usar el servidor de la app (D-509).
5. **La app solo ve lo que el usuario puede ver,** con su sesión, y a un ritmo de persona: un expediente a la vez, un pedido a la vez, con pausas entre pedidos, para no cargar el portal.
6. **No se guarda lo que el portal muestra de otros.** La tabla «Notas» (quién consultó el expediente) no se lee ni se guarda nunca.
7. **Términos de uso:** antes de usarlo con clientes, revisar con un abogado el reglamento del portal y las acordadas del PJN sobre su uso; se hace con el estudio de referencia al entregarle el producto mínimo (D-478, P-265).
8. **Lo que se trae es un documento más de la base:** se guarda en «Expedientes», dentro de la carpeta de trabajo, que es un origen de la base, con sus datos al lado (expediente, fecha, fojas, tipo, detalle, oficina, dirección de origen), y la base lo procesa como cualquier archivo.

## 3. Cómo funciona: el "navegador asistido"

Construido el 22 de octubre para el PJN (D-446 a D-449, D-451) y ajustado con la primera prueba real (D-480, D-491, D-503). Las pruebas del programa usan páginas inventadas que copian la estructura del portal.

1. **«Portales» en el menú.** Una página con un formulario (jurisdicción, número y año), la lista de expedientes seguidos con la fecha de su última actualización, y la ventana del portal.
2. **La ventana** es un navegador dentro de la ventana de la app, con su propio perfil persistente. Se crea una sola vez y se reutiliza: se muestra al entrar a «Portales» y se oculta, sin cerrarse, al salir (D-461). No tiene permisos de la app, solo va a direcciones `https` del PJN (una dirección `http` de `pjn.gov.ar` se cancela y se abre la misma con `https`, D-491), no abre ventanas nuevas y solo deja pasar la descarga que la app pidió. El permiso de bajar varios archivos seguidos lo concede sola, y solo a `pjn.gov.ar` y sus subdominios (D-503).
3. **Bloqueada mientras la app trabaja:** se ve, pero no se puede tocar. El bloqueo actúa solo en la página principal, cuya capa tapa toda la ventana, recuadros incluidos, y al bloquear saca el foco, para que las teclas no lleguen a un recuadro (D-480). Cuando la app reconoce por sus rótulos el desafío («Ver desafío») o el inicio de sesión (un campo de contraseña), la habilita para la persona con un aviso; mientras la persona la usa, la app solo mira. Con el desafío, **modo foco**: la página baja hasta su recuadro y el resto se oscurece; se quita cuando el desafío queda resuelto o la persona lo saca, y nunca se usa para iniciar sesión (D-491). Sigue sola cuando la página cambia, o con «Ya lo resolví, seguir». «Usar la ventana yo» la deja usar cuando la app no está trabajando, por si el portal muestra algo que la app no reconoce.
4. **Traer un expediente:** la app completa el formulario y consulta. Mientras la página carga o dice «Consulta en proceso», espera hasta 2 minutos sin volver a tocar «Consultar», con la ventana libre para «Cancelar» (D-491). Lee la carátula, la dependencia y la situación, y recorre **todas** las páginas de actuaciones con «Siguiente», buscado siempre a partir de la tabla de actuaciones, de a un pedido con 0,7 segundos entre dos; termina cuando no hay «Siguiente», cuando la página no cambia en 30 segundos o cuando vuelve una página ya vista. Lee fechas con día de una o dos cifras y cada celda sin el rótulo para lectores de pantalla; avisa si una página que no es la última no tiene 15 actuaciones, y una fila que no pueda leer, con su número (D-503). Baja con «Descargar» solo lo nuevo, de a un documento; las filas sin documento se registran sin bajar nada.
5. **Mientras baja** (D-503): recién cuando empieza a leer las páginas del expediente, la ventana del sitio se oculta y en su lugar se ve el avance, grande, con «Ver el portal» y «Detener»; vuelve sola si hay desafío, inicio de sesión o «Consulta en proceso». La persona puede ir a otras pantallas: una línea al pie de la ventana y Atención dicen que el recorrido sigue, que el portal la necesita y cómo terminó (bien, en «En curso»; con una falla, en «Problemas»), hasta que vuelve a abrir «Portales».
6. **Guarda en la carpeta de trabajo:** `<carpeta de trabajo>\Expedientes\PJN\<jurisdicción> <número>-<año>\`. Cada documento lleva un nombre que dice expediente, fojas, fecha, tipo y detalle, y al lado un archivo de datos (`.micelya.json`: fecha, fojas, tipo, detalle, oficina, dirección de origen, fecha de descarga y tamaño) que la base no toma por documento. Cada descarga se revisa antes de guardarla: solo queda si es un PDF no vacío (por ejemplo, una página de sesión vencida se descarta y queda como «falló»).
7. **Reanudar y no duplicar:** en la carpeta del expediente, un archivo de estado guarda cada actuación con su estado (sin documento, pendiente, bajado, ya estaba, falló) y se escribe después de cada documento. Si el recorrido se corta (el desafío o la sesión a mitad de camino, «Detener», el cierre de la app), sigue desde donde quedó sin repetir lo bajado. No se baja lo que la base ya tiene en cualquier origen con el mismo expediente, fecha, tipo y foja. Cada documento se intenta dos veces por recorrido; lo que falló se intenta en la próxima actualización.
8. **En la base:** como la carpeta de trabajo ya es un origen, lo traído se inventaría y se lee como cualquier documento, y entra a un **caso confirmado**, hecho por una persona, con la carátula y el número del expediente (D-446).
9. **Actualizar, a pedido:** «Actualizar» trae solo lo nuevo y, como un expediente solo suma actuaciones, corta en la primera página que la app ya tenía entera, siempre que el recorrido anterior haya llegado al final, que no quede nada pendiente ni fallido y que las fechas vengan de la más nueva a la más vieja; «Revisar todo el expediente» lo recorre entero, y la primera actualización de un expediente traído antes del 23 oct también (D-503). «Actualizar todos» pone los seguidos en fila, uno por vez (cada uno puede pedir el desafío). Los **vinculados** (acumulados, incidentes) se leen como referencias y se ofrecen para traer; no se traen solos.

**Un conector por portal:** lo propio del PJN (rótulos, tabla, paginador, descarga) está en un módulo aparte; lo común (ventana, bloqueo, descargas, guardado, estado) se reutiliza, para sumar otros portales sin rehacerlo.

**Alternativa para las descargas** (sin usar): con la sesión ya abierta, pedir los documentos directamente con las cookies de la ventana. Es más rápido pero más frágil y más parecido a un robot; queda solo si descargar dentro de la ventana no alcanza.

## 4. Plan de trabajo (esfuerzo relativo)

| # | Paso | Estado al 23 oct | Esfuerzo | Depende de |
|---|---|---|---|---|
| 1 | **Prototipo con el operador:** registrar cómo son las pantallas reales | Reemplazado por las dos pruebas del dueño con un navegador (21 oct), que dieron las pantallas, la paginación y la descarga. Falta medir cada cuánto vuelve el desafío | Chico a mediano | — |
| 2 | **Conector del PJN, consulta pública:** recorrer las actuaciones y bajar todo, con avance, pausas y detención ante el desafío; datos de cada actuación al lado | Construido el 22 oct (D-447 a D-449). En la primera prueba real (22 oct), con el desafío y la consulta arreglados (D-480, D-491), funcionó de punta a punta; el 23, el lector corregido y las descargas sin preguntar (D-503). Falta que el dueño vea los arreglos del 23 (P-298) | Mediano | 1 |
| 3 | **Integración con la base:** carpeta por expediente, caso del expediente, actualización incremental | Construido el 22 oct: carpeta en la carpeta de trabajo, caso confirmado (no propuesto, como decidió el dueño), «Actualizar» y «Actualizar todos» incrementales; el 23, actualizar corta en la página conocida, y el recorrido sigue y avisa fuera de «Portales» (D-503). No hay aviso en Atención de actuaciones nuevas (ver la sección 5) | Mediano | 2 |
| 4 | **Sesión del letrado:** inicio de sesión del abogado en la ventana, expedientes de Penal y Familia propios, «Mis causas» | En parte: la ventana se habilita para iniciar sesión cuando el portal lo pide. «Mis causas» no está. Se prueba con el estudio de referencia (D-478) | Mediano | 2 |
| 5 | **«Ver históricas»:** traer también las actuaciones históricas | Sin empezar (P-263) | Chico a mediano | 2 |
| 6 | **Las otras pestañas del expediente:** Intervinientes, Vinculados y Recursos | Sin empezar (P-301) | Chico a mediano | 2 |
| 7 | **Segundo portal** con el mismo marco (a elegir) | Sin empezar (P-264) | Mediano por portal | 3 |

## 5. Decisiones abiertas para el dueño

**Resueltas por el dueño el 21 oct (D-446):** «Portales» en el menú con un navegador dentro de la página; la app maneja el portal y la ventana se habilita solo para el desafío o el inicio de sesión; las novedades se traen a pedido; cada expediente crea un caso confirmado; se guarda en `<carpeta de trabajo>\Expedientes\PJN\`. **El 22 oct (D-478):** los términos de uso se revisan con el estudio de referencia al entregarle el producto mínimo, y con ellos se prueba el inicio de sesión del abogado.

**Asumidas en la corrida desatendida del 23 oct (D-503), a confirmar:** los archivos ya bajados con el lector anterior conservan su nombre (con «TIPO-ACTUACION» en el nombre; la alternativa es renombrarlos); la primera actualización de un expediente viejo lo recorre entero; la ventana se oculta recién al leer las páginas del expediente.

**Siguen abiertas:**

1. Qué portales siguen al PJN y en qué orden (P-264).
2. Si la app puede, más adelante, recordar la contraseña del abogado en el llavero del sistema (hoy no la pide ni la guarda).
3. Si hace falta un aviso en Atención con las actuaciones nuevas, o alcanza con verlas al tocar «Actualizar».
4. Si se traen también las actuaciones de «Ver históricas» (P-263) y las otras pestañas del expediente (P-301).
5. Qué hacer con un expediente grande: traerlo con la lectura automática encendida (cada PDF se lee con el modelo) o mostrar antes cuántos documentos son y dejar elegir.

## 6. Riesgos

- **El portal cambia sin aviso:** el conector busca por rótulos y estructura (nunca por los identificadores generados ni por número de página), tiene pruebas sobre páginas guardadas y avisa cuando no reconoce una pantalla o una fila.
- **El desafío puede pedirse muy seguido:** la experiencia depende de esa frecuencia, que todavía no se midió; «Actualizar todos» puede pedirlo una vez por expediente.
- **Bloqueos por exceso de pedidos:** ritmo de persona (0,7 segundos entre pedidos, un expediente y un documento a la vez).
- **Volumen:** cientos de PDF por expediente. Cada uno entra a la base y, con la lectura automática encendida, se lee con el modelo, que usa cupo; los escaneos, además, se reconocen.
- **PDF protegidos:** si alguno no deja sacar su texto ni reconocerlo, queda en la base sin texto; hay que verlo con documentos reales.
- **La ventana dentro de la app** usa una característica de Tauri marcada como inestable: hay que revisar que no cambie nada de la ventana principal. Mientras está a la vista, tapa los diálogos de la app que caen en esa zona.
- **Descargas con la ventana oculta** (D-503, sin comprobar con el portal real): que WebView2 siga bajando con la ventana oculta y que el pedido de permiso llegue por el evento que la app intercepta. Ese permiso se toma del WebView2 nativo, porque Tauri no lo expone: una actualización de Tauri puede pedir revisarlo.
- **Dos movimientos iguales el mismo día** cuentan como uno.
- **Expedientes restringidos o de competencias reservadas:** la app no los ve y lo dice.

## 7. Lo que mostró la primera prueba real (22 oct) y cómo quedó

- **La tabla de actuaciones del SCW:** 15 filas por página, de la más nueva a la más vieja. El día de la fecha va sin cero y el mes con cero («7/09/2025»); cada celda empieza con un rótulo que solo lee un lector de pantalla («Fecha: », «Tipo actuacion: »); los botones «Descargar» y «Ver» van en la primera celda; MOVIMIENTO y EVENTO no tienen documento ni fojas. El lector de la primera versión solo tomaba fechas con dos cifras en el día y salteaba las de los días 1 a 9, con su documento: por eso las páginas «mostraban» 6 a 14 actuaciones. Ahora lee todas, y una fila que no pueda leer se avisa con su número exacto.
- **Actualizar no recorre todo:** como un expediente judicial solo suma actuaciones, una actualización corta en la primera página que la app ya tenía entera, siempre que la anterior haya llegado al final, que no quede nada pendiente o fallido y que las fechas vengan de la más nueva a la más vieja. «Revisar todo el expediente» lo recorre entero.
- **Descargas sin preguntar:** la ventana del portal concede sola, y solo a `pjn.gov.ar` y sus subdominios, el permiso de WebView2 de bajar varios archivos seguidos.
- **La ventana se oculta mientras baja:** mientras la app trabaja sola, la ventana del portal se oculta y su lugar muestra el avance; vuelve cuando el portal pide a la persona, cuando procesa una consulta o con «Ver el portal». La persona puede ir a otras pantallas: el pie de la ventana y Atención dicen cómo va y cómo terminó.
- **Antes, el desafío y la consulta** (D-480, D-491; detalle en [la hipótesis del desafío](portales-pjn-desafio-hipotesis.md)): el guion que bloquea la ventana corría también dentro del recuadro del desafío, que quedaba bloqueado; ahora actúa solo en la página principal. Y la consulta quedaba para siempre en «Consulta en proceso» porque la ventana rechazaba la dirección `http` a la que redirige el PJN; ahora la pasa a `https`.
