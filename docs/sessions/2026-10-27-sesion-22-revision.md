# Sesión 22, revisión del dueño — 27 de octubre de 2026, tarde

Para retomar: este documento, `CLAUDE.md` y `pendientes.json`. El traspaso anterior es [2026-10-27-sesion-22-cierre.md](2026-10-27-sesion-22-cierre.md).

**Datos reales:** ninguno en el repositorio. Lo de esta tanda está en `C:\Users\Usuario\LocalRepo\llm_wiki_mi\revision-27oct\`. Ahí están el plan, los informes de los dos pasos, el recorrido de la app con las rutas y la copia de la base medida.

## 1. En una frase

El dueño revisó lo construido en la corrida 13 y fijó criterios. Una persona mira solo lo relevante o dudoso. El texto de lectura incierta no funda conocimiento. El modelo grande es el último escalón. La firma no se pierde al separar la constancia. Y la medición cuenta solo límites exactos. El reproceso completo queda postergado.

## 2. Hecho

- **Decisiones D-603 a D-612 (del dueño y aplicadas)** y D-613 y D-614 (asumidas en la medición). Cambian D-556, D-570, D-571, D-590, D-593, D-597 y D-598, y reemplazan D-587 y D-594.
- **La medición de copias y versiones cuenta solo límites exactos,** abre con su cobertura y deja lo aproximado aparte (35758a9). Con la hoja nueva y toda la carpeta: 55 pares evaluados. Tal como queda, 47 aciertos, 3 errados y 5 faltantes; con los cortes confirmados, 49, 4 y 2. Hay 2 errores de corte y 24 pares excluidos con su motivo. Con la aproximación de antes eran 57 pares (48, 3, 6 y 51, 4, 2).
- **El mapa de las correcciones de código,** en los pendientes P-414, P-415, P-417, P-418, P-408, P-419 y P-420. Los dos hallazgos que mandan:
  - un paso dudoso sin respuesta cuenta hoy como «sigue»;
  - un PDF de una sola parte queda confirmado a la fuerza.

  Lo incierto y la evidencia piden una versión 6 de la base.
- **La propuesta de la prueba del clasificador chico:** [clasificador-de-cortes-propuesta.md](../design/clasificador-de-cortes-propuesta.md).
- **Mensaje 005 a Core:** el dueño lo envió (P-371 hecho). Su ampliación es un borrador fuera del repositorio que se envía cuando él lo decida (P-425).

## 3. Lo que espera al dueño

- **Aprobar la prueba del clasificador** y elegir su conjunto nuevo (P-421).
- **Ampliar la hoja de partes,** con Claude proponiendo y él comprobando (P-431).
- **El recorrido corto de la app** (P-424). Mejor después de P-414, P-415 y P-408.

## 4. Lo que sigue para el agente local

En este orden:

1. **La regla «archivo parejo»,** medida (P-420).
2. **Los tres estados del corte** con la versión 6 (P-414).
3. **El modelo grande solo donde corresponde** (P-418).
4. **La firma** (P-408 y P-419).
5. **El texto incierto** (P-415).
6. **La cuenta de las consultas** (P-417).

Cada uno se mide con los arneses corregidos. El reproceso, solo después de P-422.

## 5. Consumo

- Sub-agentes: 154.077 (medición) y 149.519 (mapa de código; se pasó de su tope de 130 mil, dentro del 20 %).
- Agente principal, estimado: unas 130 mil.
- Total: unas 0,43 M, contra un tope de 0,5 M.
