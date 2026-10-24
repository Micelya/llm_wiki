"""How much Docling's page structure helps to split document 02 into its documents.
For each step from page p to p+1 it computes simple Docling-based signals and scores two naive rules against the
known starts (from the visual review). Prints numbers only.
Usage: python -I cortes02.py <doc.json>"""
import json, re, sys
from docling_core.types.doc import DoclingDocument, ContentLayer, TextItem

STARTS = {1, 4, 5, 6, 8, 9, 11, 13, 15, 16, 17, 19, 21, 23}   # first page of each document inside 02
doc = DoclingDocument.load_from_json(sys.argv[1])
n = len(doc.pages)
info = {p: {"heads": [], "hdr": [], "ftr": [], "text": ""} for p in range(1, n + 1)}
for it, _ in doc.iterate_items(included_content_layers={ContentLayer.BODY, ContentLayer.FURNITURE}, traverse_pictures=True):
    if not isinstance(it, TextItem) or not it.prov:
        continue
    pv = it.prov[0]; p = pv.page_no; H = doc.pages[p].size.height
    top = (H - pv.bbox.t) / H if pv.bbox.coord_origin.name == "BOTTOMLEFT" else pv.bbox.t / H
    lab = str(it.label.value)
    if lab in ("title", "section_header") and top < 0.35:
        info[p]["heads"].append(it.text)
    if lab == "page_header": info[p]["hdr"].append(it.text)
    if lab == "page_footer": info[p]["ftr"].append(it.text)
    info[p]["text"] += " " + it.text

def norm(s): return re.sub(r"\d", "#", " ".join(s.lower().split()))
PAG = re.compile(r"p[áa]g(?:ina)?\.?\s*(\d+)\s*(?:de|/)\s*(\d+)", re.I)
rows = []
for p in range(2, n + 1):
    a, b = info[p - 1], info[p]
    same_hdr = bool(set(map(norm, a["hdr"])) & set(map(norm, b["hdr"])))
    same_ftr = bool(set(map(norm, a["ftr"])) & set(map(norm, b["ftr"])))
    m = PAG.search(b["text"]); cont_pag = bool(m and m.group(1) != "1")
    rows.append({"page": p, "start": p in STARTS, "head_on_top": bool(b["heads"]), "same_header": same_hdr,
                 "same_footer": same_ftr, "page_x_of_y_cont": cont_pag})
def score(pred):
    tp = sum(1 for r in rows if pred(r) and r["start"]); fp = sum(1 for r in rows if pred(r) and not r["start"])
    fn = sum(1 for r in rows if not pred(r) and r["start"]); tn = len(rows) - tp - fp - fn
    return {"aciertos": tp + tn, "de": len(rows), "cortes_bien": tp, "cortes_de_mas": fp, "cortes_perdidos": fn}
rule_a = lambda r: r["head_on_top"]
rule_b = lambda r: r["head_on_top"] and not r["same_header"] and not r["page_x_of_y_cont"]
decided = [r for r in rows if r["same_header"] or r["page_x_of_y_cont"]]
print(json.dumps({"regla_titulo_arriba": score(rule_a), "regla_titulo_y_sin_continuidad": score(rule_b),
                  "pasos_con_senal_de_continuidad": len(decided),
                  "de_esos_bien": sum(1 for r in decided if not r["start"]),
                  "por_pagina": [{k: v for k, v in r.items()} for r in rows]}, ensure_ascii=False))
