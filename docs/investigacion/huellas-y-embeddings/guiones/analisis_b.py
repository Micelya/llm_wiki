"""Parte B y combinación: qué encuentran los embeddings.

Uso: python analisis_b.py <nombre_corto> [<nombre_corto> ...]

Vector de un documento: promedio normalizado de los vectores de sus páginas.
Umbrales elegidos antes de medir, por razones generales:
- K = 5 vecinos;
- datos clave: el conjunto de palabras con dígitos (fechas, montos, números);
  dos textos "tienen los mismos datos clave" si su Jaccard es >= 0,8.
"""
import json
import os
import re
import sys
from collections import Counter, defaultdict

import numpy as np
from rank_bm25 import BM25Okapi

import comun
from huellas import norm

K = 5
JACCARD_CLAVE = 0.8
POSITIVAS = ["mismo-documento", "practicamente-el-mismo", "vista-previa", "contenido-en",
             "version", "ejemplar-firmado"]


def unit(v):
    return v / (np.linalg.norm(v, axis=-1, keepdims=True) + 1e-9)


def texto_doc(n, pags, desde=None, hasta=None):
    from embeddings import texto_pagina
    ps = pags[n][desde:hasta]
    return " ".join(texto_pagina(n, p) for p in ps)


def claves(t):
    return {w for w in norm(t).split() if any(c.isdigit() for c in w)}


def jac(a, b):
    return len(a & b) / len(a | b) if a | b else 1.0


def cargar(corto, docs, pags):
    z = np.load(os.path.join(comun.SALIDA, f"emb-{corto}.npz"))
    pv = {k: v for k, v in zip(z["claves"], unit(z["emb"]))}
    reps = sorted({int(k.split(":")[0]) for k in z["claves"]})
    pagv = {n: np.array([pv[f"{n}:{p['pag']}"] for p in pags[n]]) for n in reps}
    dv = {n: unit(pagv[n].mean(0)) for n in reps}
    return reps, pagv, dv, unit(z["preguntas"])


def analizar(corto, docs, pags, rel_ph):
    reps, pagv, dv, qv = cargar(corto, docs, pags)
    rep_de = {}
    for n in sorted(pags):
        rep_de.setdefault(docs[n]["sha"], n)
    M = np.array([dv[n] for n in reps])
    S = M @ M.T
    np.fill_diagonal(S, -1)
    idx = {n: i for i, n in enumerate(reps)}
    orden = np.argsort(-S, axis=1)

    def rango(a, b):
        return int(np.where(orden[idx[a]] == idx[b])[0][0]) + 1

    out = {}
    # 1. copias y versiones
    resp = comun.hoja("respuestas.json")
    rr = comun.por_ruta(docs)
    por_rel = defaultdict(lambda: {"pares": 0, "en_top5": 0, "cos": []})
    hoja_pares = {}
    for p in resp["pairs"]:
        a, b = rr.get(p["a"]), rr.get(p["b"])
        if a not in pags or b not in pags or docs[a]["sha"] == docs[b]["sha"]:
            continue
        a, b = rep_de[docs[a]["sha"]], rep_de[docs[b]["sha"]]
        hoja_pares[(min(a, b), max(a, b))] = p["relation"]
        r = min(rango(a, b), rango(b, a))
        e = por_rel[p["relation"]]
        e["pares"] += 1
        e["en_top5"] += r <= K
        e["cos"].append(float(S[idx[a], idx[b]]))
    for e in por_rel.values():
        c = e.pop("cos")
        e["cos_mediana"] = round(float(np.median(c)), 3)
        e["cos_min"] = round(min(c), 3)
        e["cos_max"] = round(max(c), 3)
    out["copias_por_relacion"] = dict(por_rel)

    # vecinos cercanos de los documentos de la hoja: qué son
    en_hoja = {rep_de[docs[rr[d["path"]]]["sha"]] for d in resp["documents"]
               if rr.get(d["path"]) in pags}
    tipo_vecino = Counter()
    for a in en_hoja:
        for j in orden[idx[a]][:K]:
            b = reps[j]
            key = (min(a, b), max(a, b))
            r = hoja_pares.get(key)
            tipo_vecino["positivo" if r in POSITIVAS else ("otra-relacion:" + r if r else
                        ("sin-etiqueta-misma-hoja" if b in en_hoja else "fuera-de-la-hoja"))] += 1
    out["vecinos_top5_de_docs_de_la_hoja"] = dict(tipo_vecino)
    out["docs_de_la_hoja_con_pdf"] = len(en_hoja)

    # trampa jurídica: misma plantilla frente a mismo documento
    cl = {n: claves(texto_doc(n, pags)) for n in reps}
    pos_cos = [S[idx[a], idx[b]] for (a, b), r in hoja_pares.items() if r == "mismo-documento"]
    umbral = min(pos_cos) if pos_cos else 1.0
    trampa = {"umbral_cos_min_mismo_documento": round(float(umbral), 3)}
    for rel in ("misma-plantilla", "mismo-documento", "ejemplar-firmado", "sin-relacion"):
        ps = [(a, b) for (a, b), r in hoja_pares.items() if r == rel]
        altos = [(a, b) for a, b in ps if S[idx[a], idx[b]] >= umbral]
        trampa[rel] = {"pares": len(ps), "cos_ge_umbral": len(altos),
                       "mismos_datos_clave": sum(jac(cl[a], cl[b]) >= JACCARD_CLAVE for a, b in ps),
                       "cos_ge_umbral_y_mismos_datos": sum(jac(cl[a], cl[b]) >= JACCARD_CLAVE for a, b in altos)}
    out["trampa"] = trampa

    # 2. casos
    casos = {n: set(filter(None, (docs[n]["casos_app"] or "").split(";"))) for n in reps}
    con = [n for n in reps if casos[n]]
    hits = tot = 0
    for a in con:
        vec = [reps[j] for j in orden[idx[a]] if reps[j] in casos and casos[reps[j]]][:K]
        hits += sum(bool(casos[a] & casos[b]) for b in vec)
        tot += len(vec)
    azar = np.mean([bool(casos[a] & casos[b]) for a in con for b in con if a != b])
    out["casos"] = {"docs_con_caso": len(con), "casos_distintos": len(set().union(*casos.values())),
                    "precision_top5": round(hits / tot, 3), "azar": round(float(azar), 3)}

    # 3. tipos, dejando cada uno afuera
    tipos = {n: docs[n]["tipo_app"] for n in reps if docs[n]["tipo_app"]}
    ok1 = ok5 = 0
    for a in tipos:
        vec = [reps[j] for j in orden[idx[a]] if reps[j] in tipos][:K]
        ok1 += tipos[vec[0]] == tipos[a]
        votos = Counter()
        for b in vec:
            votos[tipos[b]] += S[idx[a], idx[b]]
        ok5 += votos.most_common(1)[0][0] == tipos[a]
    mayor = Counter(tipos.values()).most_common(1)[0][1]
    out["tipos"] = {"docs": len(tipos), "tipos_distintos": len(set(tipos.values())),
                    "acierto_1_vecino": round(ok1 / len(tipos), 3), "acierto_5_vecinos": round(ok5 / len(tipos), 3),
                    "acierto_tipo_mas_comun": round(mayor / len(tipos), 3)}

    # 4. búsqueda: embedding, BM25 y la mezcla (RRF), puntaje del doc = mejor página
    qs = comun.hoja("preguntas.json")["preguntas"]
    pag_tok, pag_doc = [], []
    for n in reps:
        for p in pags[n]:
            from embeddings import texto_pagina
            pag_tok.append(norm(texto_pagina(n, p)).split() or ["_"])
            pag_doc.append(n)
    bm = BM25Okapi(pag_tok)
    pag_doc = np.array(pag_doc)

    def por_doc(scores):
        best = defaultdict(lambda: -1e9)
        for n, s in zip(pag_doc, scores):
            best[n] = max(best[n], s)
        return sorted(best, key=lambda n: -best[n])

    allpv = np.concatenate([pagv[n] for n in reps])
    met = {m: {"recall5": [], "recall10": [], "rr": []} for m in ("embedding", "bm25", "mezcla")}
    sin_pdf = 0
    for qi, q in enumerate(qs):
        esp = set()
        for e in q["esperados"]:
            n = rr.get(e["ruta"])
            if n in pags:
                esp.add(rep_de[docs[n]["sha"]])
            else:
                sin_pdf += 1
        if not esp:
            continue
        r_emb = por_doc(allpv @ qv[qi])
        r_bm = por_doc(bm.get_scores(norm(q["pregunta"]).split()))
        rrf = Counter()
        for lst in (r_emb, r_bm):
            for i, n in enumerate(lst):
                rrf[n] += 1 / (60 + i)
        r_mix = [n for n, _ in rrf.most_common()]
        for m, lst in (("embedding", r_emb), ("bm25", r_bm), ("mezcla", r_mix)):
            met[m]["recall5"].append(len(esp & set(lst[:5])) / len(esp))
            met[m]["recall10"].append(len(esp & set(lst[:10])) / len(esp))
            met[m]["rr"].append(next((1 / (i + 1) for i, n in enumerate(lst) if n in esp), 0))
    out["busqueda"] = {m: {k: round(float(np.mean(v)), 3) for k, v in d.items()} for m, d in met.items()}
    out["busqueda"]["preguntas_medidas"] = len(met["bm25"]["rr"])
    out["busqueda"]["esperados_sin_pdf"] = sin_pdf

    # 5. combinación: el embedding propone (top 5), la huella phash confirma
    ph = {(a, b): r for a, b, r in rel_ph}
    cand = set()
    for a in reps:
        for j in orden[idx[a]][:K]:
            b = reps[j]
            cand.add((min(a, b), max(a, b)))
    conf = {k for k in cand if ph.get(k) in ("mismo", "dentro")}
    solo_ph = {k for k, r in ph.items() if r in ("mismo", "dentro")}

    def evaluar(conj):
        tp = sum(1 for k in conj if hoja_pares.get(k) in POSITIVAS)
        fp = sum(1 for k in conj if k in hoja_pares and hoja_pares[k] not in POSITIVAS)
        return {"pares": len(conj), "positivos_de_la_hoja": tp, "falsos_de_la_hoja": fp,
                "entre_clientes": sum(1 for a, b in conj if docs[a]["grupo"] != docs[b]["grupo"]
                                      and {docs[a]["grupo"], docs[b]["grupo"]} != {"caso-ps", "expedientes-pjn"})}
    out["combinacion"] = {"positivos_medibles": sum(1 for r in hoja_pares.values() if r in POSITIVAS),
                          "solo_phash": evaluar(solo_ph), "embedding_top5_y_phash": evaluar(conf),
                          "comparaciones_phash": {"todas": len(reps) * (len(reps) - 1) // 2, "con_candidatos": len(cand)}}

    # partes: vecino suelto más cercano de cada documento incluido
    partes = comun.hoja("partes.json")
    cos_inc = []
    for f in partes["files"]:
        c = rep_de.get(f["sha256"])
        if c is None:
            continue
        for d in f["documents"]:
            v = unit(pagv[c][d["first_page"] - 1:d["last_page"]].mean(0))
            otros = [n for n in reps if n != c]
            cs = np.array([dv[n] for n in otros]) @ v
            cos_inc.append(float(cs.max()))
    out["partes_vecino_suelto"] = {"incluidos": len(cos_inc),
                                   "cos_ge_0_95": sum(c >= 0.95 for c in cos_inc),
                                   "cos_ge_umbral_mismo_documento": sum(c >= umbral for c in cos_inc),
                                   "cos_mediana": round(float(np.median(cos_inc)), 3)}
    return out


def main():
    docs = comun.documentos()
    pags = comun.paginas()
    rel_ph = json.load(open(os.path.join(comun.SALIDA, "relaciones-huellas.json")))["phash"]
    todo = {c: analizar(c, docs, pags, rel_ph) for c in sys.argv[1:]}
    json.dump(todo, open(os.path.join(comun.SALIDA, "metricas-b.json"), "w"), indent=1, ensure_ascii=False, default=lambda o: o.item())
    for c, o in todo.items():
        print("##", c)
        for k, v in o.items():
            print(k, json.dumps(v, ensure_ascii=False, default=lambda o: o.item()))


if __name__ == "__main__":
    main()
