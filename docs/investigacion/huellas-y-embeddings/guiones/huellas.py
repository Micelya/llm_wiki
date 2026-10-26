"""Parte A: huellas de cada página de cada PDF.

Uso: python huellas.py <carpeta_datos> <salida.jsonl> <carpeta_textos> [subcarpetas a excluir...]

Por página guarda:
- exacta: sha256 del flujo de contenido decodificado más los datos crudos de sus
  imágenes y formularios (recursos XObject), en orden de nombre;
- dhash y phash (64 bits) de la página dibujada a 256 px de alto, en grises;
- texto: sha256 del texto normalizado (minúsculas, sin acentos, solo letras y
  dígitos separados por un espacio), si tiene al menos 40 caracteres;
- chars: largo del texto normalizado.
El texto crudo de cada página va a <carpeta_textos>/<n>/<pagina>.txt.
"""
import hashlib
import json
import os
import re
import sys
import unicodedata
from concurrent.futures import ProcessPoolExecutor

import imagehash
import pypdfium2 as pdfium
from pypdf import PdfReader

MIN_CHARS = 40


def norm(t):
    t = unicodedata.normalize("NFKD", t.lower())
    t = "".join(c for c in t if not unicodedata.combining(c))
    return " ".join(re.findall(r"[a-z0-9]+", t))


def xobjects(res, h, seen):
    try:
        xo = res.get("/XObject") if res else None
        xo = xo.get_object() if xo is not None else None
    except Exception:
        return
    if not xo:
        return
    for name in sorted(xo.keys()):
        obj = xo[name].get_object()
        key = id(obj)
        if key in seen:
            continue
        seen.add(key)
        try:
            h.update(obj.get_data() if obj.get("/Subtype") == "/Form" else obj._data)
        except Exception:
            pass
        if obj.get("/Subtype") == "/Form":
            xobjects(obj.get("/Resources"), h, seen)


def exacta(page):
    h = hashlib.sha256()
    try:
        c = page.get_contents()
        h.update(c.get_data() if c is not None else b"")
    except Exception:
        return None
    xobjects(page.get("/Resources"), h, set())
    return h.hexdigest()


def procesar(args):
    n, path, textos = args
    out = []
    try:
        reader = PdfReader(path)
        pdf = pdfium.PdfDocument(path)
    except Exception as e:
        return [{"doc": n, "error": type(e).__name__}]
    os.makedirs(os.path.join(textos, str(n)), exist_ok=True)
    for i in range(len(pdf)):
        p = pdf[i]
        tp = p.get_textpage()
        raw = tp.get_text_range()
        with open(os.path.join(textos, str(n), f"{i}.txt"), "w", encoding="utf-8") as f:
            f.write(raw)
        t = norm(raw)
        w, hgt = p.get_size()
        img = p.render(scale=256 / max(hgt, 1), grayscale=True).to_pil()
        try:
            ex = exacta(reader.pages[i])
        except Exception:
            ex = None
        out.append({
            "doc": n, "pag": i, "exacta": ex,
            "dhash": str(imagehash.dhash(img)), "phash": str(imagehash.phash(img)),
            "texto": hashlib.sha256(t.encode()).hexdigest() if len(t) >= MIN_CHARS else None,
            "chars": len(t), "w": round(w), "h": round(hgt),
        })
    return out


def main():
    datos, salida, textos = sys.argv[1:4]
    excluir = sys.argv[4:]
    lista = [l.rstrip("\n").split("\t") for l in open(os.path.join(datos, "..", "lista.tsv"), encoding="utf-8")]
    trabajos = []
    for n, (fid, rel, size) in enumerate(lista):
        if not rel.lower().endswith(".pdf") or any(rel.startswith(e) for e in excluir):
            continue
        trabajos.append((n, os.path.join(datos, rel), textos))
    paginas = errores = 0
    with open(salida, "w") as f, ProcessPoolExecutor() as ex:
        for res in ex.map(procesar, trabajos):
            for r in res:
                if "error" in r:
                    errores += 1
                else:
                    paginas += 1
                f.write(json.dumps(r) + "\n")
    print(f"pdfs={len(trabajos)} paginas={paginas} errores={errores}")


if __name__ == "__main__":
    main()
