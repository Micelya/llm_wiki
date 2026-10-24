"""Collect numbers only (no text from the documents) for every tool and document into one JSON.
Usage: python -I juntar.py <results_dir> <answers_01.json> <out.json>
Expects: results/pdfium/<doc>/pNN.{plain,pos}.txt, results/docling/<engine-mode>/<doc>/{run.json,pp/}, results/rapid/<doc>/,
results/granite/. Uses calidad.py, pares01.py and mrz2.py from the same folder."""
import json, os, re, subprocess, sys
from pathlib import Path

here = Path(__file__).parent
R, gold, outp = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
sys.path.insert(0, str(here))
from calidad import measure  # noqa: E402

def mrz(path):
    out = subprocess.run([sys.executable, "-I", str(here / "mrz2.py"), str(path)], capture_output=True, text=True).stdout
    d = json.loads(out); d.pop("archivo"); return d

def cer(key, path):
    out = subprocess.run([sys.executable, "-I", str(here / "cer.py"), str(R.parent / "privado" / "oro-impreso.json"), key, str(path)],
                         capture_output=True, text=True).stdout
    d = json.loads(out); d.pop("archivo"); return d

def hand(key, path):
    out = subprocess.run([sys.executable, "-I", str(here / "manuscrito.py"), str(R.parent / "privado" / "oro-manuscrito.json"), key, str(path)],
                         capture_output=True, text=True).stdout
    d = json.loads(out); d.pop("archivo"); return d

def checks(get):
    """Printed passages (character error) and handwritten values, for any tool, given a page-file getter."""
    c = {}
    for key, doc, page in (("04-p14", "04-digitalizacion-capa-vieja-93-paginas", 14), ("02-p01", "02-compuesto-23-paginas", 1)):
        f = get(doc, page)
        if f and f.exists(): c.setdefault("error_de_caracteres_impreso", {})[key] = cer(key, f)
    for key, page in (("02-p09", 9), ("02-p15", 15)):
        f = get("02-compuesto-23-paginas", page)
        if f and f.exists(): c.setdefault("manuscrito", {})[key] = hand(key, f)
    return c

def pairs(path):
    out = subprocess.run([sys.executable, "-I", str(here / "pares01.py"), gold, str(path)], capture_output=True, text=True).stdout
    v = list(json.loads(out).values())[0]; return {"correctos": v["ok"], "de": v["total"], "fallados": v["wrong"]}

DOCS = sorted(p.name for p in (R / "pdfium").iterdir() if p.is_dir())
HAND = {"02-compuesto-23-paginas": [5, 6, 7, 9, 11, 13, 15, 17]}            # pages with handwriting (visual review)
CURSIVE = {"02-compuesto-23-paginas": [6, 7]}
PRINTED_SCAN = {"02-compuesto-23-paginas": [1, 2, 3, 4], "03-oficio-pie-pjn": [6, 7, 8]}
BLANK = {"03-oficio-pie-pjn": [3, 9]}
res = {"herramientas": {}}

def page_metrics(texts):
    return {p: measure(t) for p, t in texts.items()}

# a) pdfium
pd = {}
for d in DOCS:
    files = sorted((R / "pdfium" / d).glob("p*.plain.txt"))
    st = json.load(open(R / "pdfium" / d / "stats.json"))
    pd[d] = {"paginas": st["pages"], "segundos_texto_total": round(sum(x["text_s"] for x in st["per_page"]), 3),
             "por_pagina": page_metrics({int(f.name[1:3]): f.read_text(encoding="utf-8") for f in files})}
pd["01-formulario-docusign"]["pares_texto_simple"] = pairs(R / "pdfium/01-formulario-docusign/p01.plain.txt")
pd["01-formulario-docusign"]["pares_por_posicion"] = pairs(R / "pdfium/01-formulario-docusign/p01.pos.txt")
pd["controles"] = checks(lambda d, p: R / "pdfium" / d / f"p{p:02d}.plain.txt")
res["herramientas"]["pdfium"] = pd

# b) RapidOCR alone
if (R / "rapid").exists():
    rp = {}
    for d in sorted(x.name for x in (R / "rapid").iterdir() if x.is_dir()):
        run = json.load(open(R / "rapid" / d / "run.json"))
        texts = {int(f.name[1:3]): f.read_text(encoding="utf-8") for f in sorted((R / "rapid" / d).glob("p*.txt"))}
        e = {"lado_largo_px": run["long_side"], "inicio_s": run["init_s"], "memoria_max_mb": run["peak_rss_mb"],
             "segundos_por_pagina": round(sum(x["ocr_s"] + x["render_s"] for x in run["per_page"]) / len(run["per_page"]), 2),
             "por_pagina": page_metrics(texts)}
        if d.startswith("02"):
            e["zona_lectura"] = {p: mrz(R / "rapid" / d / f"p{p:02d}.txt") for p in (19, 21) if (R / "rapid" / d / f"p{p:02d}.txt").exists()}
        rp[d] = e
    rp["controles"] = checks(lambda d, p: R / "rapid" / d / f"p{p:02d}.txt")
    res["herramientas"]["rapidocr_solo"] = rp

# c) Docling configurations (with PyTorch) and Docling without PyTorch (layout on onnxruntime, no tables)
CFGS = [("docling_" + x.name, R / "docling" / x.name) for x in sorted((R / "docling").iterdir()) if x.is_dir()]
if (R / "docling-onnx").exists():
    CFGS += [("docling_sin_pytorch_rapidocr-" + x.name, x) for x in sorted((R / "docling-onnx").iterdir()) if x.is_dir()]
for name, root in CFGS:
    out = {}
    for d in DOCS:
        base = root / d
        if not (base / "run.json").exists():
            out[d] = {"no_corrio": True}; continue
        run = json.load(open(base / "run.json"))
        labels = json.load(open(base / "pp" / "labels.json"))
        texts = {int(f.name[1:3]): f.read_text(encoding="utf-8") for f in sorted((base / "pp").glob("p??.txt"))}
        n = run["pages"]
        e = {"segundos_total": run["wall_s"], "segundos_por_pagina": round(run["wall_s"] / n, 2), "memoria_max_mb": run["peak_rss_mb"],
             "etapas_s": {k: (v["total_s"] if isinstance(v, dict) else v) for k, v in run["timings"].items()},
             "zona_reconocida_por_pagina": {int(k): v["area_share"] for k, v in run.get("ocr_regions", {}).items()},
             "caracteres_por_pagina": {int(p): {"total": sum(x["chars"] for x in L if not x["furniture"]),
                                                 "dentro_de_imagen_no_va_al_markdown": sum(x["chars"] for x in L if x["in_picture"] and not x["furniture"]),
                                                 "pie_o_encabezado": sum(x["chars"] for x in L if x["furniture"])} for p, L in labels.items()},
             "por_pagina": page_metrics(texts)}
        if d.startswith("01"):
            e["pares_texto_por_pagina"] = pairs(base / "pp" / "p01.txt")
            if (base / "p01.md").exists():
                e["pares_markdown"] = pairs(base / "p01.md")
        if d in BLANK:
            e["paginas_en_blanco_detectadas"] = {p: len(texts.get(p, "").strip()) == 0 for p in BLANK[d]}
        if d.startswith("02"):
            e["zona_lectura"] = {p: mrz(base / "pp" / f"p{p:02d}.txt") for p in (19, 21)}
            c = subprocess.run([sys.executable, "-I", str(here / "cortes02.py"), str(base / "doc.json")], capture_output=True, text=True).stdout
            if c:
                c = json.loads(c); c.pop("por_pagina"); e["cortes_02"] = c
        if d.startswith(("05", "06")):
            dj = json.load(open(base / "doc.json"))
            e["tablas"] = [{"pagina": t["prov"][0]["page_no"], "filas": len(t["data"]["grid"]),
                            "columnas": len(t["data"]["grid"][0]) if t["data"]["grid"] else 0} for t in dj["tables"]]
        out[d] = e
    out["controles"] = checks(lambda d, p, root=root: root / d / "pp" / f"p{p:02d}.txt")
    res["herramientas"][name] = out

# d) granite-docling
if (R / "granite").exists():
    g = {}
    for f in sorted((R / "granite").glob("*.run.json")):
        run = json.load(open(f)); stem = f.name[:-9]
        md = (R / "granite" / f"{stem}.md").read_text(encoding="utf-8")
        g[stem] = {"segundos": run["seconds"], "memoria_max_mb": run["peak_rss_mb"], **measure(md)}
        if stem in ("02-p19", "02-p21"):
            g[stem]["zona_lectura"] = mrz(R / "granite" / f"{stem}.md")
    g["controles"] = checks(lambda d, p: R / "granite" / f"{d[:2]}-p{p:02d}.md")
    g["pares_01"] = pairs(R / "granite" / "01-p01.md")
    res["herramientas"]["granite_docling_258M"] = g
if (R / "arranque.json").exists():
    res["arranque"] = json.load(open(R / "arranque.json"))
if (R / "tamanos.json").exists():
    res["tamanos_mb"] = json.load(open(R / "tamanos.json"))
res["paginas_con_manuscrito"] = HAND; res["paginas_en_cursiva"] = CURSIVE; res["escaneos_impresos"] = PRINTED_SCAN
json.dump(res, open(outp, "w", encoding="utf-8"), ensure_ascii=False, indent=1)
print("ok", outp)
