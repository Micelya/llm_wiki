# Lectura de manuscritos en el procesador: RapidOCR frente a modelos de visión chicos

26 de octubre de 2026. Prueba hecha en una sesión de Claude Code en la nube. Usa las hojas y los guiones de medición de la prueba de Docling ([`../docling/informe.md`](../docling/informe.md)). Las páginas, las hojas y lo que leyó cada modelo quedaron fuera del repositorio. Acá solo hay números.

## En resumen

1. **Qwen3-VL-2B lee lo manuscrito dos veces mejor que lo mejor que teníamos:** 25 de 37 valores casi bien, 21 de ellos exactos. El mejor resultado anterior, Docling con RapidOCR, era 12, con 6 exactos.
2. **También lee bien lo impreso y el formulario:** 0,9 % de caracteres errados en la página escrita a máquina (antes 2,4 %), con todas las tildes, y los 27 pares del formulario (antes 26). En el poder impreso erró un poco más que RapidOCR: 3,3 % contra 1,5 %.
3. **El costo es alto en el procesador:** unos 5 minutos por página y 12,7 GB de memoria, en precisión completa. RapidOCR tarda 5 segundos y usa 0,7 GB. Para una PC sirve en segundo plano y solo para las páginas manuscritas, no para todas.
4. **PaddleOCR-VL no se pudo correr.** Ni la versión incluida en transformers ni su propio código, autorizado por el dueño, funcionan con las versiones de transformers que probé.
5. **Licencias:** Qwen3-VL-2B, RapidOCR, PaddleOCR-VL, transformers y PyTorch son Apache 2.0 o BSD, compatibles con una app con licencia GPL v3.

## Cómo se probó

- **Equipo:** procesador Xeon de 2,8 GHz con 4 núcleos, 15 GB de memoria, Linux, sin placa de video.
  - Es la misma clase de máquina que en las pruebas anteriores, pero a más velocidad (2,8 GHz contra 2,1): el entorno se reinició en otra máquina.
- **Páginas,** dibujadas a 200 ppp con pypdfium2:

  | Página | Qué es |
  |---|---|
  | `02-p09` | partida de defunción, manuscrita |
  | `02-p15` | certificado médico, manuscrito |
  | `02-p01` | poder, impreso |
  | `04-p14` | página escrita a máquina |
  | `01-p01` | formulario |

  - **Tamaño de la imagen:** se acotó el lado mayor a 2.200 px para todos, y a 1.600 px para el modelo de visión.
  - **Por qué:** sin ese límite, una página escaneada grande llega a 4.456 × 6.689 px. La prueba de huellas mostró que eso dispara la memoria del OCR.
- **Hojas y guiones:** los mismos de la prueba de Docling, sin cambios: `manuscrito.py`, `cer.py` y `pares01.py`. Las hojas se bajaron con el enlace compartido y nunca se abrieron: los guiones solo imprimen números.
- **Modelos:**
  - **RapidOCR 3.10.0,** con su configuración normal (los modelos PP-OCRv6 que trae el paquete).
  - **Qwen3-VL-2B-Instruct,** con transformers 5.20.2 y PyTorch 2.14.1 para procesador, en precisión completa (float32).
    - Instrucción: «transcribí todo el texto, incluido lo manuscrito, en orden de lectura», en inglés.
    - Sin muestreo, hasta 2.048 piezas de texto por página.
- **Ninguno de los modelos medidos es de Claude.** La hoja de manuscrito la armó un agente de Claude leyendo la imagen. Ninguno de los valores de la hoja se corrigió. La prueba no permite saber si alguno está mal, porque las respuestas de los modelos no se leyeron.

## Resultados

| Medida | RapidOCR 3.10 | Qwen3-VL-2B | Mejor anterior (Docling con RapidOCR) |
|---|---|---|---|
| **Manuscrito,** 37 valores: casi bien (exactos) | 10 (3) | **25 (21)** | 12 (6) |
| …partida `02-p09`, 14 valores | 4 (1) | 7 (6) | — |
| …certificado `02-p15`, 23 valores | 6 (2) | 18 (15) | — |
| **Impreso,** caracteres errados: poder `02-p01` | 1,5 % | 3,3 % | 0,2 % |
| …a máquina `04-p14` | 9,0 % | **0,9 %** | 2,4 % (capa vieja: 3,8 %) |
| Tildes del 04, de 10 | 8 | 10 | — |
| **Formulario 01,** pares bien de 27 | 25 | **27** | 26 |

- **RapidOCR dio peor que en la prueba de Docling** (10 contra 12 en lo manuscrito, y 9,0 % contra 2,4 % en el 04). La diferencia más probable es el tamaño de la imagen: acá se achicaron las páginas grandes, y Docling le pasa la página a su manera. Es una inferencia: no se comparó con la misma imagen.
- **Qwen3-VL erró más que RapidOCR en el poder impreso** (3,3 % contra 1,5 %). Es lo esperable en un modelo que genera texto: puede cambiar una palabra por otra parecida.

## Costo en el procesador

| | RapidOCR 3.10 | Qwen3-VL-2B |
|---|---|---|
| Segundos por página | 5 a 6 | 223 a 387 (promedio 306) |
| Carga del modelo | 0,7 s | 25 s (68 s la primera vez) |
| Memoria máxima | 0,7 GB | 12,7 GB |
| Tamaño del modelo | viene en el paquete | 4,3 GB |

- La memoria y el tiempo de Qwen3-VL corresponden a la precisión completa. En media precisión (bfloat16) ocuparía la mitad de memoria; no se midió si conserva la calidad ni cuánto tarda.
- Se dejó la instrucción y el límite de 2.048 piezas de texto; no se probó otra combinación.

## PaddleOCR-VL: por qué no se pudo correr

1. **La versión incluida en transformers 5.20** cargó con un aviso sobre unos pesos compartidos. En una página no terminó: se cortó una vez a los 71 minutos y otra a los 13. Probablemente quedó repitiendo texto (inferencia).
2. **Su propio código** (el del repositorio del modelo en Hugging Face, autorizado por el dueño) falló con cada versión de transformers que se probó:
   - con la 5.20 falta un campo de configuración;
   - con la 4.57 y la 4.55, una función interna cambió de nombre.
3. **Lo que no se probó:** la herramienta propia de PaddlePaddle (`paddleocr`, con PaddlePaddle en lugar de PyTorch), que es su forma oficial de correrlo. Queda para otra prueba, si vale la pena frente al resultado de Qwen3-VL.

## Recomendación

| Para qué | Qué usar |
|---|---|
| Lo impreso y las páginas con capa de texto | RapidOCR, como hasta ahora: rápido y liviano |
| Lo manuscrito, en la PC | Qwen3-VL-2B, solo para las páginas marcadas como manuscritas, en segundo plano |
| Antes de llevarlo a la app | Medir Qwen3-VL en media precisión (memoria y calidad), en una PC con Windows, y con más páginas manuscritas |

- **Cuánto tarda:** a 5 minutos por página, 100 páginas manuscritas son unas 8 horas de máquina en un procesador como este. Por eso conviene marcar antes qué páginas tienen manuscrito, con la ficha de forma de cada página.
- **Si no alcanza en la PC,** lo manuscrito queda para un modelo con imagen en Core.

## Lo que no se pudo probar, y por qué

- **PaddleOCR-VL:** no corrió (ver arriba).
- **Las otras 35 páginas manuscritas** de `partes.json`: no tienen verdad contra la cual medir.
- **Media precisión y otras instrucciones** para Qwen3-VL: no se probaron, para no gastar tiempo sin una pregunta clara.
- **Si la hoja de manuscrito tiene errores:** no se leyeron las respuestas de los modelos, así que no hay base para decirlo.

## Fichas de Claude

- **Sub-agentes:** ninguno.
- **Agente principal:** unas 30 mil fichas desde el arranque, según el contador de la sesión; el tope era de 70 mil. El contador probablemente no incluye la relectura del contexto.
- **Lo que más tiempo costó** fueron los intentos con PaddleOCR-VL: tiempo de máquina, casi sin fichas.

## Revisión de datos reales

Antes del commit se buscaron en esta carpeta:
- nombres de personas y de carpetas de clientes;
- números largos, correos y rutas personales.

No se encontró ninguno. Los valores se nombran solo por la clave de su página.
