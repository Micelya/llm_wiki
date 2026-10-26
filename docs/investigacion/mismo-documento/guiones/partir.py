"""Parte cada uno de los seis PDF de partes.json en sus documentos.

Uso: python partir.py <carpeta_salida>
Escribe <salida>/parte-<f>-<id>.pdf con las páginas de cada documento incluido,
y <salida>/../partes.tsv (archivo, id, páginas, sha del compuesto).
"""
import json
import os
import sys

from pypdf import PdfReader, PdfWriter

BASE = os.path.expanduser("~/prueba-huellas")


def main():
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)
    partes = json.load(open(os.path.join(BASE, "datos/pruebas/hojas-de-respuestas/partes.json"), encoding="utf-8"))
    docs = json.load(open(os.path.join(BASE, "salida/docs.json")))
    por_sha = {}
    for d in docs.values():
        if not d["rel"].startswith("pruebas/"):
            por_sha.setdefault(d["sha"], d["rel"])
    filas = []
    for f_i, f in enumerate(partes["files"]):
        rel = por_sha[f["sha256"]]
        reader = PdfReader(os.path.join(BASE, "datos", rel))
        for d in f["documents"]:
            w = PdfWriter()
            for p in range(d["first_page"] - 1, d["last_page"]):
                w.add_page(reader.pages[p])
            name = f"parte-{f_i}-{d['id']}.pdf"
            with open(os.path.join(out, name), "wb") as fh:
                w.write(fh)
            filas.append(f"{name}\t{f_i}\t{d['id']}\t{d['last_page'] - d['first_page'] + 1}\t{f['sha256']}\t{d['type']}")
    open(os.path.join(out, "..", "partes.tsv"), "w").write("\n".join(filas) + "\n")
    print(f"partes={len(filas)}")


if __name__ == "__main__":
    main()
