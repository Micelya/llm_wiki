"""Lee el registro de la prueba de la app y cuenta las relaciones entre cada
documento incluido (parte-*.pdf) y los documentos sueltos. Solo imprime números.

Uso: python pares_partes.py <registro> <partes.tsv> <salida.json>
"""
import json
import os
import re
import sys
from collections import Counter, defaultdict

BASE = os.path.expanduser("~/prueba-huellas")


def main():
    log, tsv, salida = sys.argv[1:4]
    partes = {}
    for l in open(tsv):
        name, f_i, pid, npag, sha, tipo = l.rstrip("\n").split("\t")
        partes[name] = {"sha": sha, "tipo": tipo, "pags": int(npag)}
    docs = json.load(open(os.path.join(BASE, "salida/docs.json")))
    rels = sorted(((d["rel"], d) for d in docs.values()), key=lambda x: -len(x[0]))

    def suelto(path):
        p = path.replace("\\", "/")
        for rel, d in rels:
            if p.endswith(rel) or p.endswith(rel.split("/", 1)[-1]):
                return d
        return None

    lines = open(log, encoding="utf-8", errors="replace").read().split("\n")
    en_pares, rel, a = False, None, None
    pares = []
    for l in lines:
        if l.startswith("== pairs (pass 1)") or l.startswith("\n== pairs") or "== pairs (pass 1)" in l:
            en_pares = True
            continue
        if en_pares and l.startswith("== "):
            break
        if not en_pares:
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
            pares.append((rel, a, m.group(1)))
            a = None
    por_parte = defaultdict(list)
    otros = Counter()
    sin_mapa = 0
    for rel, a, b in pares:
        pa = next((n for n in partes if a.endswith(n)), None)
        pb = next((n for n in partes if b.endswith(n)), None)
        if pa and pb:
            otros["parte-parte:" + rel] += 1
            continue
        if not pa and not pb:
            otros["suelto-suelto:" + rel] += 1
            continue
        parte, otro = (pa, b) if pa else (pb, a)
        d = suelto(otro)
        if d is None:
            sin_mapa += 1
            continue
        if d["sha"] == partes[parte]["sha"]:
            otros["parte-su-compuesto:" + rel] += 1
            continue
        por_parte[parte].append((rel, d["grupo"]))
    rel_count = Counter(r for v in por_parte.values() for r, g in v)
    grupos = Counter(g for v in por_parte.values() for r, g in v)
    out = {"pares_de_la_app": len(pares), "partes": len(partes),
           "partes_con_algun_suelto": len(por_parte),
           "relaciones_parte_suelto": dict(rel_count), "grupo_del_suelto": dict(grupos),
           "otros_pares": dict(otros), "rutas_sin_mapa": sin_mapa,
           "partes_con_suelto_por_relacion": {r: sum(1 for v in por_parte.values() if any(x == r for x, _ in v)) for r in rel_count}}
    json.dump({"resumen": out, "detalle": {k: v for k, v in por_parte.items()}}, open(salida, "w"), indent=1)
    print(json.dumps(out))


if __name__ == "__main__":
    main()
