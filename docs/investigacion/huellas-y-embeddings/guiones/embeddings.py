"""Parte B: embeddings por página, en el procesador.

Uso: python embeddings.py <modelo> <nombre_corto>
Texto de cada página: el de pypdfium2 o, si tiene menos de 40 caracteres, el del
OCR. Se recorta a MAX_CHARS. Guarda salida/emb-<nombre>.npz (claves n:pagina) y
las preguntas de preguntas.json embebidas como consulta.
"""
import os
import resource
import sys
import time

import numpy as np
from fastembed import TextEmbedding

import comun

MAX_CHARS = 2000
QWEN_QUERY = "Instruct: Given a search query, retrieve relevant passages that answer the query\nQuery: "


def texto_pagina(n, p):
    base = os.path.join(comun.SALIDA, "textos", str(n))
    t = open(os.path.join(base, f"{p['pag']}.txt"), encoding="utf-8").read()
    if p["chars"] < 40:
        o = os.path.join(base, f"{p['pag']}.ocr.txt")
        if os.path.exists(o):
            t = open(o, encoding="utf-8").read()
    return " ".join(t.split())[:MAX_CHARS]


def main():
    modelo, corto = sys.argv[1], sys.argv[2]
    docs = comun.documentos()
    pags = comun.paginas()
    vistos, claves, textos = set(), [], []
    for n in sorted(pags):
        if docs[n]["sha"] in vistos:
            continue
        vistos.add(docs[n]["sha"])
        for p in pags[n]:
            claves.append(f"{n}:{p['pag']}")
            textos.append(texto_pagina(n, p) or "(vacía)")
    m = TextEmbedding(modelo, cache_dir=os.path.join(comun.BASE, "modelos"), threads=4)
    t0 = time.time()
    emb = np.array(list(m.embed(textos, batch_size=8)), dtype=np.float32)
    dt = time.time() - t0
    preguntas = [q["pregunta"] for q in comun.hoja("preguntas.json")["preguntas"]]
    if "qwen" in corto:
        qe = np.array(list(m.embed([QWEN_QUERY + q for q in preguntas])), dtype=np.float32)
    else:
        qe = np.array(list(m.embed(preguntas)), dtype=np.float32)
    np.savez(os.path.join(comun.SALIDA, f"emb-{corto}.npz"), claves=np.array(claves), emb=emb, preguntas=qe)
    rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss // 1024
    print(f"{corto} paginas={len(textos)} dim={emb.shape[1]} segundos={dt:.0f} "
          f"por_pagina={dt / len(textos):.3f} memoria_MB={rss}")


if __name__ == "__main__":
    main()
