# Portal del PJN: el desafío bloqueado y el "modo foco" — hipótesis a validar

- **Pendiente:** P-281 (Portales: el desafío del PJN queda bloqueado y no se puede resolver).
- **Estado:** hipótesis, no decisión. La escribió el agente de Claude Code de la sesión del 21 y 22 de octubre; el dueño la trajo el 22 oct por la tarde para guardarla junto al pendiente.
- **Verificado el 22 oct (tarde), sin reproducir todavía:** la documentación de Tauri 2.11.1 dice "Windows: scripts are always added to subframes" (`tauri-2.11.1/src/webview/mod.rs`, línea 830), y la capa de Windows (`wry` 0.55.1) instala todos los guiones de inicio con `AddScriptToExecuteOnDocumentCreated`, sin distinguir la página principal de los recuadros. `INIT` no mira en qué marco corre y arranca bloqueado; el desbloqueo se manda solo a la página principal.

El texto que sigue es el original, sin cambios.

---

## Portal del PJN: causa del desafío bloqueado y "modo foco" (información de la sesión anterior)

**Causa del bloqueo (confirmada leyendo el código; falta reproducirla con una prueba):**
- El guion de bloqueo `INIT` (`src-tauri/src/portals/scripts.rs`) se instala con `initialization_script` en `src-tauri/src/portals/view.rs`, línea ~260.
- La documentación de Tauri 2.11 (`webview/mod.rs`) dice: "Windows: scripts are always added to subframes". En Windows ese guion corre también **dentro del recuadro del desafío**, que es un iframe de otro dominio (`captcha.pjn.gov.ar`), y arranca bloqueado (`locked = true`).
- El desbloqueo (`view.eval(scripts::lock(false, ""))`) se ejecuta **solo en la página principal**. El recuadro queda bloqueado: su capa tapa «Ver desafío» y su cartel "Micelya está usando esta ventana" aparece arriba y al centro del recuadro. Así se ve en la captura del dueño del 22 oct.
- Las pruebas no lo detectaron porque las páginas inventadas no tienen ningún iframe.

**Nuevo pedido del dueño: "modo foco" durante el desafío.**
- Cuando aparece el desafío:
  - desplazar la página hasta el recuadro;
  - oscurecer todo el resto con una capa en la página principal, con un hueco exacto sobre el recuadro que siga sus cambios de tamaño;
  - si hace falta, agrandar la zona del portal;
  - indicar en el panel de la app qué hacer.
- Al resolverse, quitar el foco y seguir solo.
- **No** abrir la página del desafío aparte: probablemente rompa la constancia que el desafío le devuelve a la página para «Consultar».
- **No** modificar la estructura de la página del portal: es solo una capa visual, sin tocar el desafío ni el formulario. Respeta las reglas del portal: no saltear controles y no tocar la página mientras la persona resuelve.
- Sin verificar todavía (se ve en la prueba real): si «Ver desafío» se abre dentro del recuadro o en la página principal, y cómo vuelve la constancia.

**Orden acordado:**
1. **Arreglar el bloqueo (P-281).**
   - Que `INIT` no actúe dentro de los recuadros: al principio, `if (window.top !== window) return;`. La capa de la página principal ya tapa los recuadros mientras está bloqueada.
   - Agregar una prueba con una página inventada que tenga un iframe.
2. **Prueba real con el dueño en el portal.** Confirmar que el desafío se puede resolver y mirar cómo se comporta («Ver desafío», tamaño del recuadro, constancia, «Consultar»).
3. **"Modo foco"**, como parte de P-281 o como una tarea nueva, con lo aprendido en el paso 2. Al final, otra prueba real con el dueño.
