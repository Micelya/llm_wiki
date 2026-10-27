"""Lee las páginas de prueba con PaddleOCR-VL por su herramienta oficial (paddleocr,
sobre PaddlePaddle), en el procesador.

Uso: python leer_paddle.py <clave>...
Usa las mismas imágenes que leer.py (img/<clave>.png). Salida:
txt/paddleocr-vl-oficial/<clave>.txt (el Markdown que arma la herramienta) y .json
con el tiempo y la memoria. Si ya existe, la saltea.
"""
import json
import os
import resource
import sys
import time

BASE = os.path.expanduser("~/prueba-ms")


def main():
    from paddleocr import PaddleOCRVL
    d = os.path.join(BASE, "txt", "paddleocr-vl-oficial")
    os.makedirs(d, exist_ok=True)
    t = time.time()
    pipe = PaddleOCRVL(device="cpu")
    carga = time.time() - t
    for clave in sys.argv[1:]:
        if os.path.exists(os.path.join(d, f"{clave}.txt")):
            continue
        t = time.time()
        partes = []
        for res in pipe.predict(os.path.join(BASE, "img", f"{clave}.png")):
            md = res.markdown
            partes.append(md.get("markdown_texts", "") if isinstance(md, dict) else str(md))
        dt = time.time() - t
        txt = "\n".join(partes)
        open(os.path.join(d, f"{clave}.txt"), "w", encoding="utf-8").write(txt)
        json.dump({"segundos": round(dt, 1), "carga_s": round(carga, 1), "chars": len(txt),
                   "memoria_MB": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss // 1024},
                  open(os.path.join(d, f"{clave}.json"), "w"))
        print("paddleocr-vl-oficial", clave, f"segundos={dt:.0f} chars={len(txt)}", flush=True)


if __name__ == "__main__":
    main()
