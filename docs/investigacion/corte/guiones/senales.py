"""Cheap, model-free signals of every page of the PDFs in a folder (pdfium only).

Usage: CORTE_PYLIB=<folder with pypdfium2> python -I senales.py <pdf folder> <out.json>

For each page: size, the page shape of the app (page_shape.rs, thresholds of
page-shapes.json, copied below), the main image (pixels, dpi, color, filter)
and the cut patterns of the earlier research ("página x de y", document codes,
notarial sheet serials, MRZ). The output has the page texts' patterns, so it
stays outside the repository.
"""
import collections
import json
import os
import re
import sys

if os.environ.get("CORTE_PYLIB"):
    sys.path.insert(0, os.environ["CORTE_PYLIB"])
import pypdfium2 as pdfium  # noqa: E402
import pypdfium2.raw as raw  # noqa: E402

# page-shapes.json (catalog of the app), version 1.
SHAPES = {
    "repeated": {"share": 0.6, "minPages": 3, "minLineChars": 8},
    "scan": {"imageCover": 0.85, "layerMinChars": 80, "noisySingleLetters": 0.3},
    "picture": {"imageCover": 0.25, "maxOwnChars": 80},
    "drawn": {"minObjects": 200},
    "blank": {"maxOwnChars": 20},
    "form": {"shortLineChars": 40, "shortLineShare": 0.6, "minLines": 5, "minFields": 3,
             "flattenedMinValues": 3, "signingServices": ["docusign", "adobe sign", "acrobat sign", "echosign"]},
}

PATTERNS = {
    # "Página 2 de 3", "pág. 2/3"
    "page_of": re.compile(r"p[áa]g(?:ina)?\.?\s*(\d{1,3})\s*(?:de|/)\s*(\d{1,3})", re.I),
    # Document codes of an electronic records system: KIND-YYYY-NUMBER-...
    "doc_code": re.compile(r"\b([A-Z]{2,4}-\d{4}-\d{6,}-[A-Z0-9-]*[A-Z])"),
    # Serial of a notarial sheet: one or two letters and 8-9 digits.
    "sheet_serial": re.compile(r"\b([A-Z]{1,2})\s?(\d{8,9})\b"),
    # Machine readable zone of an identity document.
    "mrz": re.compile(r"[A-Z0-9<]{20,}<<"),
}


def visible(s):
    return sum(1 for c in s if not c.isspace())


def norm_line(line):
    return re.sub(r"\d", "#", " ".join(line.split()))


def repeated_lines(texts):
    r = SHAPES["repeated"]
    n = len(texts)
    if n < r["minPages"]:
        return set()
    counts = collections.Counter()
    for t in texts:
        counts.update({norm_line(l) for l in t.splitlines() if len(norm_line(l)) >= r["minLineChars"]})
    need = max(-(-r["share"] * n // 1), r["minPages"])
    return {l for l, c in counts.items() if c >= need}


def classify(p, signed):
    """Port of page_shape.rs::classify."""
    s = SHAPES
    if p["images_cover"] >= s["scan"]["imageCover"]:
        if p["own_chars"] < s["scan"]["layerMinChars"]:
            return "scan"
        return "scan_old_ocr"
    picture = p["images_cover"] >= s["picture"]["imageCover"] and p["own_chars"] < s["picture"]["maxOwnChars"]
    drawn = p["own_chars"] < s["blank"]["maxOwnChars"] and p["drawn"] >= s["drawn"]["minObjects"]
    if picture:
        return "picture"
    if drawn:
        return "scan"
    if p["own_chars"] < s["blank"]["maxOwnChars"]:
        return "blank"
    f = s["form"]
    form = (p["own_lines"] >= f["minLines"] and p["short_lines"] >= f["shortLineShare"]) or p["fields"] >= f["minFields"]
    if form and (p["group_values"] >= f["flattenedMinValues"] or signed):
        return "flattened_form"
    return "digital_form" if form else "digital_prose"


def objects(page):
    """(object, type, inside_group) for every object, descending into groups."""
    def rec(handle, count, get, inside):
        for i in range(count(handle)):
            obj = get(handle, i)
            t = raw.FPDFPageObj_GetType(obj)
            yield obj, t, inside
            if t == raw.FPDF_PAGEOBJ_FORM:
                yield from rec(obj, raw.FPDFFormObj_CountObjects, raw.FPDFFormObj_GetObject, True)
    yield from rec(page.raw, raw.FPDFPage_CountObjects, raw.FPDFPage_GetObject, False)


def page_signals(pdf, index, text, repeated, signed):
    page = pdf[index]
    w, h = page.get_size()
    own = [l for l in text.splitlines() if visible(l) > 0 and norm_line(l) not in repeated]
    own_chars = sum(visible(l) for l in own)
    words = [m for l in own for m in re.findall(r"[^\W\d_]+", l)]
    p = {
        "page": index + 1, "w": round(w), "h": round(h), "rotation": page.get_rotation(),
        "chars": visible(text), "own_chars": own_chars, "own_lines": len(own),
        "short_lines": (sum(1 for l in own if visible(l) <= SHAPES["form"]["shortLineChars"]) / len(own)) if own else 0.0,
        "single_letters": (sum(1 for x in words if len(x) == 1) / len(words)) if words else 0.0,
        "lower_start": None,
    }
    # Whether its own text starts in lower case (a sentence that comes from the page before).
    for l in own:
        m = re.search(r"[^\W\d_]", l)
        if m:
            p["lower_start"] = m.group(0).islower()
            break
    images, cover, group_values, drawn = [], 0.0, 0, 0
    area = w * h
    for obj, t, inside in objects(page):
        if t == raw.FPDF_PAGEOBJ_IMAGE:
            img = pdfium.PdfImage(obj, page=page, pdf=pdf)
            l, b, r, tp = img.get_bounds()
            a = max(0, min(r, w) - max(l, 0)) * max(0, min(tp, h) - max(b, 0)) / area
            cover += a
            try:
                md = img.get_metadata()
                px_w, px_h, bpp, cs = md.width, md.height, md.bits_per_pixel, md.colorspace
            except Exception:  # noqa: BLE001
                px_w = px_h = bpp = cs = 0
            try:
                filters = img.get_filters()
            except Exception:  # noqa: BLE001
                filters = []
            dpi = round(px_w / ((r - l) / 72)) if r > l else 0
            images.append({"cover": round(a, 3), "px": [px_w, px_h], "dpi": dpi, "bpp": bpp, "cs": cs,
                           "filter": ",".join(filters)})
        elif t == raw.FPDF_PAGEOBJ_TEXT and inside:
            group_values += 1
        elif t == raw.FPDF_PAGEOBJ_PATH:
            drawn += 1
    fields = 0
    for k in range(raw.FPDFPage_GetAnnotCount(page.raw)):
        annot = raw.FPDFPage_GetAnnot(page.raw, k)
        if raw.FPDFAnnot_GetSubtype(annot) == raw.FPDF_ANNOT_WIDGET:
            fields += 1
        raw.FPDFPage_CloseAnnot(annot)
    images.sort(key=lambda i: -i["cover"])
    p.update({"images_cover": min(cover, 1.0), "largest_image": images[0]["cover"] if images else 0.0,
              "n_images": len(images), "main_image": images[0] if images else None,
              "group_values": group_values, "fields": fields, "drawn": drawn})
    p["shape"] = classify(p, signed)
    own_text = "\n".join(own)
    p["page_of"] = [[int(a), int(b)] for a, b in PATTERNS["page_of"].findall(own_text) if 0 < int(a) <= int(b)]
    p["doc_codes"] = sorted(set(PATTERNS["doc_code"].findall(own_text)))
    p["sheet_serials"] = sorted({f"{a}{n}" for a, n in PATTERNS["sheet_serial"].findall(own_text)})
    p["mrz"] = bool(PATTERNS["mrz"].search(text))
    # The first lines of its own text, for the opener vocabulary (kept outside the repository).
    p["head"] = " ".join(own[:6])[:400]
    p["text"] = own_text[:3000]
    return p


def main(folder, out):
    result = []
    for name in sorted(os.listdir(folder)):
        if not name.lower().endswith(".pdf"):
            continue
        pdf = pdfium.PdfDocument(os.path.join(folder, name))
        producer = (pdf.get_metadata_dict().get("Producer") or "").lower()
        signed = any(s in producer for s in SHAPES["form"]["signingServices"])
        texts = [pdf[i].get_textpage().get_text_range() for i in range(len(pdf))]
        rep = repeated_lines(texts)
        pages = [page_signals(pdf, i, texts[i], rep, signed) for i in range(len(pdf))]
        result.append({"file": name, "producer": producer, "pages": pages})
    with open(out, "w", encoding="utf-8") as f:
        json.dump(result, f, ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
