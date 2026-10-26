"""Cruza las relaciones de la app con pHash.

1. Hoja «Copias y versiones»: por cada par medible, la relación esperada, la que
   propone la app y la que dice pHash; y si pHash corregiría a la app.
2. Partes: qué documentos incluidos tienen un suelto según la app, según pHash
   y según los dos; y cuántos no tienen texto.
Solo imprime números.

Ojo: un par que la app une como un mismo contenido (mismo texto canónico) no aparece
entre sus pares, y aquí cuenta como "nada"; los aciertos de la app se leen de su
propia tabla, no de este cruce.

Uso: python cruce.py <registro_base> <registro_partes> <partes.tsv> <salida.json>
"""
import json
import os
import re
import sys
from collections import Counter

sys.path.insert(0, os.path.expanduser("~/prueba-huellas/guiones"))
import comun  # noqa: E402
from analisis_a import comunes, emparejar  # noqa: E402

FUERTES = {"same-document", "signed-copy", "version", "preview", "contained-in"}
ESPERADA = {"mismo-documento": "same-document", "ejemplar-firmado": "signed-copy", "version": "version",
            "vista-previa": "preview", "misma-plantilla": "same-template", "sin-relacion": "none",
            "contenido-en": "contained-in", "practicamente-el-mismo": "same-document"}


def pares_del_registro(log):
    out, en, rel, a = [], False, None, None
    for l in open(log, encoding="utf-8", errors="replace").read().split("\n"):
        if "== pairs (pass 1)" in l:
            en = True
            continue
        if en and l.startswith("== "):
            break
        if not en:
            continue
        m = re.match(r"^   \[(\S+)\]", l)
        if m:
            rel = m.group(1)
            continue
        m = re.match(r"^      a: (.*)$", l)
        if m:
            a = m.group(1)
            continue
        m = re.match(r"^      b: (.*)$", l)
        if m and a is not None:
            out.append((rel, a.strip(), m.group(1).strip()))
            a = None
    return out


def main():
    log_base, log_partes, tsv, salida = sys.argv[1:5]
    docs = comun.documentos()
    pags = comun.paginas()
    rels = sorted(((d["rel"], int(n)) for n, d in docs.items() if not d["rel"].startswith("pruebas/")),
                  key=lambda x: -len(x[0]))

    def doc_de(path):
        p = path.replace("\\", "/")
        for r, n in rels:
            if p.endswith(r) or p.endswith(r.split("/", 1)[-1]):
                return n
        return None

    rep = {}
    for n in sorted(pags):
        rep.setdefault(docs[n]["sha"], n)
    ph = {}
    for a, b, r in json.load(open(os.path.join(comun.SALIDA, "relaciones-huellas.json")))["phash"]:
        ph[frozenset((docs[a]["sha"], docs[b]["sha"]))] = r
    app = {}
    for rel, a, b in pares_del_registro(log_base):
        na, nb = doc_de(a), doc_de(b)
        if na is not None and nb is not None:
            app[frozenset((docs[na]["sha"], docs[nb]["sha"]))] = rel

    # 1. hoja
    resp = comun.hoja("respuestas.json")
    rr = comun.por_ruta(docs)
    tabla = Counter()
    corrige = Counter()
    for p in resp["pairs"]:
        if not p.get("measurable"):
            continue
        a, b = rr.get(p["a"]), rr.get(p["b"])
        if a is None or b is None:
            continue
        esp = ESPERADA.get(p["relation"], p["relation"])
        if docs[a]["sha"] == docs[b]["sha"]:
            tabla[(esp, "idénticos", "-")] += 1
            continue
        k = frozenset((docs[a]["sha"], docs[b]["sha"]))
        ra = app.get(k, "nada")
        es_pdf = a in pags and b in pags
        rp = ph.get(k, "nada") if es_pdf else "no-pdf"
        tabla[(esp, ra, rp)] += 1
        if ra != esp:
            if rp in ("mismo", "dentro") and esp in FUERTES:
                corrige["phash_avisa_mismo_documento_que_la_app_no_dio"] += 1
            elif rp in ("mismo", "dentro") and esp not in FUERTES:
                corrige["phash_tambien_se_equivoca"] += 1
            else:
                corrige["phash_no_ayuda"] += 1
        elif rp in ("mismo", "dentro") and esp not in FUERTES:
            corrige["app_bien_y_phash_falso"] += 1

    # 2. partes
    partes = [l.rstrip("\n").split("\t") for l in open(tsv)]
    app_partes = {}
    for rel, a, b in pares_del_registro(log_partes):
        for x, y in ((a, b), (b, a)):
            m = re.search(r"(parte-\d+-[^/\\]+\.pdf)$", x)
            if m and "parte-" not in y:
                n = doc_de(y)
                if n is not None:
                    app_partes.setdefault(m.group(1), set()).add((rel, docs[n]["sha"]))
    com = comunes(pags, docs, "phash")
    ns = sorted(rep.values())
    sin_texto = 0
    res = Counter()
    for name, f_i, pid, npag, sha, tipo in partes:
        c = rep[sha]
        f = next(f for f in comun.hoja("partes.json")["files"] if f["sha256"] == sha)
        d = next(d for d in f["documents"] if d["id"] == pid)
        rango = set(range(d["first_page"] - 1, d["last_page"]))
        texto = any(p["chars"] >= 40 for p in pags[c] if p["pag"] in rango)
        sin_texto += not texto
        ph_s = set()
        for s in ns:
            if docs[s]["sha"] == sha or len(pags[s]) >= len(pags[c]):
                continue
            m = emparejar("phash", pags[s], pags[c], com)
            if m and all(x is not None for x in m) and set(m) & rango:
                ph_s.add(docs[s]["sha"])
        ap = {s for r, s in app_partes.get(name, set()) if r in FUERTES and s != sha}
        ap_debil = {s for r, s in app_partes.get(name, set()) if r not in FUERTES and s != sha}
        clave = ("app_fuerte" if ap else ("app_debil" if ap_debil else "app_nada"),
                 "phash" if ph_s else "phash_nada", "con_texto" if texto else "sin_texto")
        res[" / ".join(clave)] += 1
        if ap and ph_s:
            res["mismo_suelto_en_los_dos"] += bool(ap & ph_s)
    out = {"hoja": {" | ".join(k): v for k, v in sorted(tabla.items())}, "hoja_correcciones": dict(corrige),
           "partes": dict(res), "partes_sin_texto": sin_texto, "partes_total": len(partes)}
    json.dump(out, open(salida, "w"), indent=1, ensure_ascii=False)
    print(json.dumps(out, ensure_ascii=False, indent=0))


if __name__ == "__main__":
    main()
