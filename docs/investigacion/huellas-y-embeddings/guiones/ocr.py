"""OCR con RapidOCR de las páginas sin texto (menos de 40 caracteres normalizados).

Dibuja la página a 200 ppp y guarda el texto en textos/<n>/<pagina>.ocr.txt.
"""
import os
import sys
import time

import pypdfium2 as pdfium
from rapidocr_onnxruntime import RapidOCR

import comun


def main():
    docs = comun.documentos()
    pags = comun.paginas()
    ocr = RapidOCR()
    hechos = 0
    t0 = time.time()
    vistos = set()
    for n in sorted(pags):
        sha = docs[n]["sha"]
        if sha in vistos:
            continue
        vistos.add(sha)
        faltan = [p["pag"] for p in pags[n] if p["chars"] < 40]
        if not faltan:
            continue
        pdf = pdfium.PdfDocument(os.path.join(comun.DATOS, docs[n]["rel"]))
        for i in faltan:
            out = os.path.join(comun.SALIDA, "textos", str(n), f"{i}.ocr.txt")
            if os.path.exists(out):
                continue
            img = pdf[i].render(scale=200 / 72).to_numpy()
            res, _ = ocr(img)
            with open(out, "w", encoding="utf-8") as f:
                f.write("\n".join(r[1] for r in res or []))
            hechos += 1
    dt = time.time() - t0
    print(f"paginas_ocr={hechos} segundos={dt:.0f} por_pagina={dt / max(hechos, 1):.2f}")


if __name__ == "__main__":
    main()
