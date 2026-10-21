# Portales judiciales en la app: navegador asistido — propuesta

**Estado:** propuesta para revisar · 21 de octubre de 2026 · nada de esto está construido
**Alcance:** traer a la base documental los expedientes y sus actuaciones desde portales judiciales, empezando por el Sistema de Consulta Web del Poder Judicial de la Nación (SCW, `https://scw.pjn.gov.ar/scw/home.seam`), sin saltear los controles del portal. Pendiente P-248.

## 1. Lo que sabemos del portal (21 oct)

**Verificado** (leyendo la página pública y su manual de usuario, sin pasar el desafío):

- Es una aplicación web clásica (JSF/Seam): formularios con un estado de sesión (`javax.faces.ViewState`, `jsessionid`), identificadores de elementos generados (`j_idt…`) que no son estables entre versiones.
- **Consulta pública:** por expediente (jurisdicción, número y año), por parte y de Reparación Histórica. Antes de consultar hay que resolver un **desafío propio del PJN** (`captcha.pjn.gov.ar`, un recuadro dentro de la página con el botón "Ver desafío" que, resuelto, deja una respuesta en el formulario).
- **Consulta de usuarios registrados:** "Iniciar sesión" con identificación y contraseña. Los letrados y partes entran con su **CUIT/CUIL y contraseña** del Sistema de Administración de Usuarios (SAU) del PJN; los no letrados, con correo y contraseña.
- **Detalle de un expediente:** datos principales, **actuaciones** (fecha, oficina, descripción, fojas, y el despacho del tribunal, que se puede **ver o descargar**), intervinientes y expedientes vinculados. Existe además el "Libro digital" (actuaciones con su PDF).
- **Restricciones:** los expedientes de Penal y Familia solo los ven los usuarios registrados que intervienen en la causa; los restringidos por el tribunal no se ven por ningún medio.
- **No encontramos una API oficial** para consultar o descargar actuaciones (búsqueda del 21 oct); solo instructivos del portal web.

**No verificado** (hay que verlo en un prototipo con el operador): cada cuánto vuelve a pedir el desafío (por consulta, por sesión, por tiempo), cómo se descarga cada despacho (enlace directo o formulario), cuántas actuaciones muestra por página, y cómo es "Mis causas" o favoritos con la sesión de un letrado.

## 2. Principios

1. **Nunca saltear los controles del portal.** El desafío lo resuelve una persona; no se usan servicios ni técnicas para resolverlo o evitarlo, ni se disfraza la app. Si el portal vuelve a pedirlo, la app se detiene y avisa.
2. **El abogado inicia sesión él mismo, en la página del portal.** La app no pide ni guarda su contraseña (más adelante, si el dueño lo decide, se podría usar el llavero del sistema). La sesión queda en un perfil del navegador propio de ese portal, como en cualquier navegador.
3. **La app solo ve lo que el usuario puede ver,** con su sesión, y a un ritmo de persona (pausas entre pedidos, un expediente a la vez), para no cargar el portal.
4. **Términos de uso:** antes de usarlo con clientes, revisar con un abogado el reglamento del portal y las acordadas del PJN sobre su uso.
5. **Lo que se trae es un documento más de la base:** se guarda en una carpeta de un origen, con sus datos (expediente, fecha, fojas, descripción, oficina, dirección de origen), y la base lo procesa como cualquier archivo.

## 3. Cómo funcionaría: el "navegador asistido"

1. **Una ventana de navegador dentro de la app** (Tauri permite abrir una vista web aparte, con su propio perfil persistente), que muestra la página real del portal. El operador elige el expediente, resuelve el desafío o inicia sesión, como lo haría en Chrome.
2. **La app reconoce en qué pantalla está** (al terminar de cargar cada página lee el contenido visible, buscando por los rótulos del portal y no por los identificadores generados) y ofrece la acción: "Traer este expediente".
3. **Al aceptar, la app sigue sola dentro de la misma sesión:** recorre la lista de actuaciones (y sus páginas), abre cada despacho y **descarga cada documento** (Tauri permite interceptar las descargas de esa ventana y guardarlas donde corresponda). Muestra el avance y se detiene si el portal vuelve a pedir el desafío o la sesión vence, para que el operador lo resuelva y siga.
4. **Guarda todo en la base:** una carpeta por expediente dentro de un origen "Expedientes" (por ejemplo `PJN/<jurisdicción> <número>-<año>/`), cada documento con un archivo de datos al lado (fecha, fojas, descripción, oficina). La base lo inventaría, lo lee y **propone un asunto con la referencia del expediente** (encaja con el diseño de asuntos: el número de expediente es una referencia que identifica el caso).
5. **Actualizar después:** "Actualizar expediente" vuelve a entrar (con un nuevo desafío si hace falta) y trae solo las actuaciones nuevas.
6. **Con la sesión de un letrado,** la misma ventana puede recorrer "Mis causas" y ofrecer traer o actualizar varias.

**Un conector por portal:** lo propio de cada portal (cómo se ve cada pantalla, dónde están las actuaciones, cómo se descarga) va en un "conector" aparte; la ventana, la detención ante desafíos, las descargas y el guardado en la base son comunes. Así, después del PJN, se suman otros portales (provinciales, de la Ciudad, administrativos) sin rehacer lo común.

**Alternativa para las descargas:** con la sesión ya abierta, la app podría pedir los documentos directamente (tomando las cookies de la ventana) en lugar de hacerlo dentro de la página. Es más rápido pero más frágil y más parecido a un robot; se prefiere descargar dentro de la ventana y se deja esto solo si la primera forma no alcanza.

## 4. Plan de trabajo (esfuerzo relativo)

| # | Paso | Esfuerzo | Depende de |
|---|---|---|---|
| 1 | **Prototipo con el operador:** ventana del portal dentro de la app con perfil propio; el operador abre el expediente de ejemplo que dio el dueño (anotado fuera del repositorio, en `llm_wiki_mi\portales`) y descarga a mano; la app captura las descargas y las guarda en un origen. Se registra cómo son las pantallas reales (detalle, actuaciones, descarga, frecuencia del desafío). | Chico a mediano | — |
| 2 | **Conector del PJN, consulta pública:** "Traer este expediente" recorre las actuaciones y descarga todo, con avance, pausas y detención ante el desafío; datos de cada actuación al lado de cada documento. | Mediano | 1 |
| 3 | **Integración con la base:** carpeta por expediente, asunto propuesto por la referencia del expediente, "Actualizar expediente" incremental, aviso en Atención de actuaciones nuevas. | Mediano | 2 |
| 4 | **Sesión del letrado:** inicio de sesión del abogado en la ventana, expedientes de Penal y Familia propios, recorrer "Mis causas". | Mediano | 2 |
| 5 | **Segundo portal** con el mismo marco (a elegir). | Mediano por portal | 3 |

## 5. Decisiones abiertas para el dueño

1. Qué portales siguen al PJN y en qué orden.
2. Si la app puede, más adelante, recordar la contraseña del abogado en el llavero del sistema (propuesta: no por ahora; el abogado inicia sesión en la página).
3. Dónde se guardan los expedientes traídos (un origen nuevo "Expedientes" o una carpeta elegida).
4. Si cada expediente traído crea un asunto propuesto (para confirmar) o uno confirmado.
5. La revisión de los términos de uso del portal con un abogado antes de usarlo con clientes.

## 6. Riesgos

- El portal cambia sin aviso: el conector se escribe buscando por rótulos y estructura, con pruebas sobre páginas guardadas, y avisa cuando no reconoce una pantalla.
- El desafío puede pedirse muy seguido: la experiencia depende de esa frecuencia; se mide en el paso 1.
- Bloqueos por exceso de pedidos: ritmo de persona y un expediente a la vez.
- Expedientes restringidos o de competencias reservadas: la app no los ve y lo dice.
