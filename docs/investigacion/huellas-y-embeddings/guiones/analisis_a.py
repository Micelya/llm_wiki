"""Parte A: qué encuentran las huellas de página.

Huellas: exacta, dhash, phash, texto. Para dhash y phash dos páginas coinciden si
la distancia de Hamming es <= TOL (de 64 bits). Una página "común" (su huella
aparece en COMUN o más contenidos distintos: hojas en blanco, carátulas de
formulario) no cuenta como prueba de coincidencia: eso es una regla general, no
mirada en la hoja de respuestas.

Relación entre dos PDF de contenido distinto (A con menos páginas que B):
- mismo: igual número de páginas y todas coinciden;
- dentro: todas las páginas de A coinciden con páginas de B, B más largo;
- parcial: al menos una página coincide;
- nada.
"""
import json
import os
import sys
from collections import Counter, defaultdict
from itertools import combinations

import comun

TOL = int(os.environ.get("TOL", 6))
COMUN = 5
HUELLAS = ["exacta", "dhash", "phash", "texto"]


def ham(a, b):
    return bin(int(a, 16) ^ int(b, 16)).count("1")


def coincide(h, x, y):
    if x is None or y is None:
        return False
    return ham(x, y) <= TOL if h in ("dhash", "phash") else x == y


def comunes(pags, docs, h):
    """Huellas que aparecen en COMUN o más contenidos distintos."""
    vistos = defaultdict(set)
    claves = []
    for n, ps in pags.items():
        for p in ps:
            if p[h] is not None:
                claves.append((p[h], docs[n]["sha"]))
    if h in ("dhash", "phash"):
        # agrupar por cercanía: cuenta contenidos con alguna página a <= TOL
        valores = sorted({k for k, _ in claves})
        sh = defaultdict(set)
        for k, s in claves:
            sh[k].add(s)
        out = set()
        for v in valores:
            cerca = set()
            for w in valores:
                if ham(v, w) <= TOL:
                    cerca |= sh[w]
            if len(cerca) >= COMUN:
                out.add(v)
        return out
    for k, s in claves:
        vistos[k].add(s)
    return {k for k, s in vistos.items() if len(s) >= COMUN}


def emparejar(h, pa, pb, com):
    """Para cada página de A, índice de la primera página de B que coincide."""
    out = []
    for p in pa:
        if p[h] is None or p[h] in com:
            out.append(None)
            continue
        j = next((q["pag"] for q in pb if coincide(h, p[h], q[h])), None)
        out.append(j)
    return out


def relacion(h, pa, pb, com):
    if len(pa) > len(pb):
        pa, pb = pb, pa
    m = emparejar(h, pa, pb, com)
    hit = sum(x is not None for x in m)
    if hit == 0:
        return "nada", m
    if hit == len(pa):
        return ("mismo" if len(pa) == len(pb) else "dentro"), m
    return "parcial", m


def main():
    docs = comun.documentos()
    pags = comun.paginas()
    # un representante por contenido (sha): las copias idénticas no cuentan
    rep = {}
    for n in sorted(pags):
        rep.setdefault(docs[n]["sha"], n)
    ns = sorted(rep.values())
    com = {h: comunes(pags, docs, h) for h in HUELLAS}
    metr = {"tolerancia_hamming": TOL, "pagina_comun_desde_contenidos": COMUN,
            "pdfs": len(pags), "contenidos": len(ns),
            "paginas": sum(len(pags[n]) for n in ns),
            "paginas_sin_texto": sum(1 for n in ns for p in pags[n] if p["texto"] is None),
            "paginas_comunes": {h: sum(1 for n in ns for p in pags[n] if p[h] in com[h]) for h in HUELLAS}}

    rel = {h: {} for h in HUELLAS}
    for a, b in combinations(ns, 2):
        for h in HUELLAS:
            r, m = relacion(h, pags[a], pags[b], com[h])
            if r != "nada":
                rel[h][(a, b)] = (r, m)
    json.dump({h: [[a, b, r] for (a, b), (r, m) in v.items()] for h, v in rel.items()},
              open(os.path.join(comun.SALIDA, "relaciones-huellas.json"), "w"))

    # 1. documentos sueltos que aparecen dentro de otro PDF
    dentro = {}
    for h in HUELLAS:
        c = Counter()
        pares = Counter()
        for (a, b), (r, m) in rel[h].items():
            if r != "dentro":
                continue
            chico, grande = (a, b) if len(pags[a]) < len(pags[b]) else (b, a)
            c[docs[chico]["grupo"]] += 1
            pares[f'{docs[chico]["grupo"]} -> {docs[grande]["grupo"]}'] += 1
        dentro[h] = {"pares_dentro": sum(c.values()), "por_grupo_del_suelto": dict(c), "por_grupos": dict(pares),
                     "pares_mismo": sum(1 for r, _ in rel[h].values() if r == "mismo"),
                     "pares_parcial": sum(1 for r, _ in rel[h].values() if r == "parcial")}
    metr["dentro_de_otro"] = dentro

    # caso-ps frente a expedientes-pjn: cuántos contenidos de cada lado tienen par
    cruces = {}
    for h in HUELLAS:
        lados = defaultdict(set)
        for (a, b), (r, m) in rel[h].items():
            ga, gb = docs[a]["grupo"], docs[b]["grupo"]
            if {ga, gb} == {"caso-ps", "expedientes-pjn"} and r in ("mismo", "dentro"):
                lados[ga].add(a)
                lados[gb].add(b)
        cruces[h] = {g: len(v) for g, v in lados.items()}
    # copias idénticas byte a byte entre las dos carpetas, como referencia
    sh = defaultdict(set)
    for n in pags:
        sh[docs[n]["sha"]].add(docs[n]["grupo"])
    cruces["identicos_byte_a_byte"] = sum(1 for g in sh.values() if {"caso-ps", "expedientes-pjn"} <= g)
    cruces["contenidos_caso_ps"] = len({docs[n]["sha"] for n in pags if docs[n]["grupo"] == "caso-ps"})
    cruces["contenidos_pjn"] = len({docs[n]["sha"] for n in pags if docs[n]["grupo"] == "expedientes-pjn"})
    metr["caso_ps_vs_pjn"] = cruces

    # 2. partes.json: documentos incluidos con aparición suelta, y cortes
    partes = comun.hoja("partes.json")
    por_sha = {}
    for n in pags:
        por_sha.setdefault(docs[n]["sha"], n)
    res_partes = {}
    for h in HUELLAS:
        incluidos = con_suelto = cortes_ok = 0
        for f in partes["files"]:
            c = por_sha.get(f["sha256"])
            if c is None:
                continue
            for d in f["documents"]:
                incluidos += 1
                rango = set(range(d["first_page"] - 1, d["last_page"]))
                mejor = None
                for s in ns:
                    if docs[s]["sha"] == f["sha256"] or len(pags[s]) >= len(pags[c]):
                        continue
                    m = emparejar(h, pags[s], pags[c], com[h])
                    if m and all(x is not None for x in m):
                        hit = set(m)
                        if hit & rango:
                            mejor = hit if mejor is None or len(hit) > len(mejor) else mejor
                if mejor is not None:
                    con_suelto += 1
                    if mejor == rango:
                        cortes_ok += 1
        res_partes[h] = {"incluidos": incluidos, "con_aparicion_suelta": con_suelto, "corte_exacto": cortes_ok}
    metr["partes"] = res_partes

    # 3. respuestas.json
    resp = comun.hoja("respuestas.json")
    rr = comun.por_ruta(docs)
    positivas = {"mismo-documento", "practicamente-el-mismo", "vista-previa", "contenido-en"}
    res_resp = {}
    for h in HUELLAS:
        por_rel = defaultdict(Counter)
        for p in resp["pairs"]:
            a, b = rr.get(p["a"]), rr.get(p["b"])
            if a is None or b is None or a not in pags or b not in pags:
                por_rel[p["relation"]]["sin_pdf"] += 1
                continue
            if docs[a]["sha"] == docs[b]["sha"]:
                por_rel[p["relation"]]["identicos"] += 1
                continue
            ra, rb = rep[docs[a]["sha"]], rep[docs[b]["sha"]]
            r = rel[h].get((min(ra, rb), max(ra, rb)), ("nada",))[0]
            por_rel[p["relation"]][r] += 1
        tp = sum(por_rel[x][k] for x in positivas for k in ("mismo", "dentro"))
        pos = sum(sum(v.values()) - v["sin_pdf"] - v["identicos"] for x, v in por_rel.items() if x in positivas)
        falsos = sum(por_rel[x][k] for x in por_rel if x not in positivas | {"ejemplar-firmado", "version"} for k in ("mismo", "dentro"))
        res_resp[h] = {"por_relacion": {k: dict(v) for k, v in por_rel.items()},
                       "positivos_encontrados": tp, "positivos_medibles": pos, "falsos_en_la_hoja": falsos}
    metr["respuestas"] = res_resp

    json.dump(metr, open(os.path.join(comun.SALIDA, "metricas-a.json"), "w"), indent=1, ensure_ascii=False)
    print(json.dumps({k: metr[k] for k in ("pdfs", "contenidos", "paginas", "paginas_sin_texto", "paginas_comunes")}))
    for h in HUELLAS:
        print(h, json.dumps(dentro[h]), json.dumps(cruces[h]), json.dumps(res_partes[h]),
              "resp:", res_resp[h]["positivos_encontrados"], "/", res_resp[h]["positivos_medibles"], "falsos", res_resp[h]["falsos_en_la_hoja"])
    print("cruces extra", {k: cruces[k] for k in ("identicos_byte_a_byte", "contenidos_caso_ps", "contenidos_pjn")})


if __name__ == "__main__":
    main()
