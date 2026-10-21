# Vistas previas de enlaces (hovercards) — sugerencias

> **Origen:** investigación del dueño del producto con Claude.ai, traída el 21 de octubre de 2026. Es material de referencia para el pendiente P-246 (citas y enlaces con vista previa flotante), no una especificación: hay que verificarlo y adaptarlo a Micelya Desktop (las páginas ya no son archivos de un wiki sino artículos armados desde la base; el resumen y la cita de cada objeto ya existen en la base). Se transcribe tal como llegó.

---

> Este documento reúne sugerencias para verificar y adaptar. No es una especificación cerrada: sirve como punto de partida para la implementación.

## El problema

En una wiki, los artículos se enlazan mucho entre sí. Para saber de qué trata una página enlazada, el lector tiene que abrirla y pierde el hilo de lo que estaba leyendo. Una alternativa es mostrar una **vista previa flotante** (*hovercard*) al pasar el mouse o enfocar un enlace, con un resumen breve y, si existe, una imagen representativa. Es el patrón de las *Page Previews* de Wikipedia.

Para que funcione bien hay que resolver cuatro cosas:

- **Posicionamiento:** la tarjeta no debe salirse de la pantalla ni tapar lo que el usuario está leyendo.
- **Interacción:** que no se abra por pasar el mouse sin intención y que no se cierre al mover el mouse hacia ella.
- **Contenido:** qué resumen y qué imagen mostrar, y cómo obtenerlos rápido.
- **Accesibilidad y dispositivos:** uso con teclado y pantallas táctiles, donde no existe el "pasar por encima".

## Buenas prácticas sugeridas

- **Demoras:** abrir después de ~500 ms de hover y cerrar con ~300 ms de margen. Conviene ajustar los valores en pruebas con usuarios.
- **Zona segura:** permitir mover el mouse del enlace a la tarjeta sin que se cierre.
- **Accesibilidad:** abrir también al enfocar con teclado, cerrar con Escape y no poner información crítica solo en la tarjeta. Como referencia se puede revisar el criterio 1.4.13 de las WCAG (pautas de accesibilidad web del W3C).
- **Táctil:** desactivar las vistas previas o definir un gesto alternativo. El toque simple debería seguir navegando.
- **Contenido precalculado:** generar y guardar un resumen corto y una imagen principal por página cuando se edita, en lugar de calcularlos en cada hover.
- **Variantes:** si la página no tiene una imagen adecuada, mostrar solo texto. La forma de la tarjeta puede adaptarse a la proporción de la imagen.
- **Rendimiento:** pedir el contenido al detectar intención, cancelar el pedido si el mouse se va y cachear las respuestas.
- **Estabilidad visual:** tamaños fijos y un estado de carga para evitar saltos. Respetar la preferencia de reducir animaciones (`prefers-reduced-motion`).
- **Preferencia de usuario:** permitir desactivar la función.
- **Seguridad:** sanitizar el HTML del resumen. Si se previsualizan enlaces externos, el servidor que busca los metadatos debe protegerse contra SSRF (que lo usen para hacer pedidos a la red interna).

## Tecnologías a evaluar

**Posicionamiento e interacción**
- *Floating UI*: posicionamiento de elementos flotantes; incluye la "zona segura" (`safePolygon`).
- *Radix UI HoverCard* / *shadcn/ui*: componentes React con accesibilidad resuelta.
- *Ark UI / Zag.js*: equivalente independiente del framework.
- *React Aria*: foco fuerte en accesibilidad.
- APIs nativas en evolución: *Popover API*, *CSS Anchor Positioning* e *interest invokers* (`interestfor`). Hay que revisar el soporte de los navegadores antes de adoptarlas.

**Contenido**
- Endpoint propio de resumen por página: título, extracto e imagen. Puede servir de referencia la API REST `/page/summary` de Wikipedia.
- Módulos *Popups* y *PageImages* de MediaWiki: son open-source y útiles para estudiar cómo resuelve Wikipedia la selección de imagen y el formato.
- Enlaces externos: metadatos *Open Graph* y *oEmbed*, con librerías como `open-graph-scraper` o `metascraper`.

## Próximos pasos sugeridos

1. Definir qué datos de resumen guarda cada página de la wiki y cuándo se generan.
2. Elegir la librería de interfaz según el framework de la aplicación.
3. Probar demoras, comportamiento táctil y accesibilidad con un prototipo antes de cerrar el diseño.
