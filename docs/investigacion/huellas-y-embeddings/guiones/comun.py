"""Datos comunes: tabla de documentos, páginas y hojas de respuestas."""
import csv
import hashlib
import json
import os
from collections import defaultdict

BASE = os.path.expanduser("~/prueba-huellas")
DATOS = os.path.join(BASE, "datos")
PRUEBAS = os.path.join(DATOS, "pruebas")
SALIDA = os.path.join(BASE, "salida")
PJN = "pruebas/expedientes-pjn/"


def grupo(rel):
    if rel.startswith(PJN):
        return "expedientes-pjn"
    return rel.split("/")[0]


def documentos():
    """n -> {rel, sha, grupo, content_id}. Solo los PDF que tienen páginas."""
    cache = os.path.join(SALIDA, "docs.json")
    if os.path.exists(cache):
        return {int(k): v for k, v in json.load(open(cache)).items()}
    base = list(csv.DictReader(open(os.path.join(PRUEBAS, "estado-de-la-app", "documentos-de-la-base.csv"), encoding="utf-8")))
    por_sha = {r["sha256"]: r for r in base}
    docs = {}
    for n, line in enumerate(open(os.path.join(BASE, "lista.tsv"), encoding="utf-8")):
        fid, rel, size = line.rstrip("\n").split("\t")
        sha = hashlib.sha256(open(os.path.join(DATOS, rel), "rb").read()).hexdigest()
        r = por_sha.get(sha)
        docs[n] = {"rel": rel, "sha": sha, "grupo": grupo(rel),
                   "content_id": r["content_id"] if r else None,
                   "tipo_app": r["tipo_app"] if r else None,
                   "casos_app": r["casos_app"] if r else None}
    json.dump(docs, open(cache, "w"))
    return docs


def paginas():
    """n -> lista de páginas en orden."""
    out = defaultdict(list)
    for l in open(os.path.join(SALIDA, "paginas.jsonl")):
        r = json.loads(l)
        if "error" not in r:
            out[r["doc"]].append(r)
    for v in out.values():
        v.sort(key=lambda r: r["pag"])
    return dict(out)


def por_ruta(docs):
    """Ruta relativa a POC-Micelya -> n."""
    return {d["rel"]: n for n, d in docs.items()}


def hoja(nombre):
    return json.load(open(os.path.join(PRUEBAS, "hojas-de-respuestas", nombre), encoding="utf-8"))
