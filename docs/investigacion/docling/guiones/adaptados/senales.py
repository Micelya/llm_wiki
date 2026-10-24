"""Deterministic, cheap signals of the 'shape' of each page of a PDF (no model, no OCR).

Usage: python -I senales.py <pdf>
Prints document-level facts, then one line per page.
"""
import collections
import ctypes
import os
import re
import sys

import pypdfium2 as pdfium  # noqa: E402
import pypdfium2.raw as raw  # noqa: E402

sys.stdout.reconfigure(encoding="utf-8")

PATTERNS = {
    "pag_x_de_y": re.compile(r"p[áa]g(?:ina)?\.?\s*\d+\s*(?:de|/)\s*\d+", re.I),
    "gedo_IF": re.compile(r"\bIF-\d{4}-\d+-[A-Z]+", re.I),
    "mrz": re.compile(r"[A-Z0-9<]{20,}<<[A-Z0-9<]*"),
    "fojas": re.compile(r"\bfs\.?\s*\d+", re.I),
    "cuit": re.compile(r"\b\d{2}-\d{8}-\d\b"),
}


def font_name(textobj):
    font = raw.FPDFTextObj_GetFont(textobj)
    if not font:
        return "?"
    n = raw.FPDFFont_GetBaseFontName(font, None, 0)
    if n <= 0:
        return "?"
    buf = ctypes.create_string_buffer(n)
    raw.FPDFFont_GetBaseFontName(font, buf, n)
    return buf.value.decode("utf-8", "replace")


def walk(page_raw):
    """Yield (obj, depth, top_form_index) for every object, descending into form XObjects."""
    def rec(parent_is_form, handle, count_fn, get_fn, depth, top):
        for i in range(count_fn(handle)):
            obj = get_fn(handle, i)
            t = raw.FPDFPageObj_GetType(obj)
            yield obj, t, depth, (top if top is not None else (i if t == raw.FPDF_PAGEOBJ_FORM else None))
            if t == raw.FPDF_PAGEOBJ_FORM:
                yield from rec(True, obj, raw.FPDFFormObj_CountObjects, raw.FPDFFormObj_GetObject, depth + 1,
                               top if top is not None else i)
    yield from rec(False, page_raw, raw.FPDFPage_CountObjects, raw.FPDFPage_GetObject, 0, None)


def bounds(obj):
    l, b, r, t = (ctypes.c_float(), ctypes.c_float(), ctypes.c_float(), ctypes.c_float())
    raw.FPDFPageObj_GetBounds(obj, l, b, r, t)
    return l.value, b.value, r.value, t.value


def norm_line(line):
    return re.sub(r"\d", "#", " ".join(line.split())).strip()


def main(path):
    pdf = pdfium.PdfDocument(path)
    meta = pdf.get_metadata_dict()
    print("archivo:", os.path.basename(path))
    print("páginas:", len(pdf), "| productor:", meta.get("Producer"), "| creador:", meta.get("Creator"),
          "| tipo de formulario:", raw.FPDF_GetFormType(pdf.raw))
    pages_text = []
    for page in pdf:
        pages_text.append(page.get_textpage().get_text_range())
    # lines repeated on many pages (footers, stamps)
    n = len(pdf)
    counter = collections.Counter()
    for text in pages_text:
        counter.update({norm_line(l) for l in text.splitlines() if len(norm_line(l)) >= 8})
    repeated = {l for l, c in counter.items() if n >= 3 and c >= max(3, 0.6 * n)}
    if repeated:
        print("renglones repetidos en >=60% de las páginas:", [r[:70] for r in repeated])
    print("p | tamaño | chars | propios | invis | 1letra% | img% | nimg | forms(texto) | widgets | saltos↑ | fuentes | patrones")
    for i, page in enumerate(pdf):
        text = pages_text[i]
        w, h = page.get_size()
        chars = sum(1 for c in text if not c.isspace())
        own = sum(sum(1 for c in l if not c.isspace()) for l in text.splitlines() if norm_line(l) not in repeated)
        toks = re.findall(r"[^\W\d_]+", text)
        single = sum(1 for t in toks if len(t) == 1)
        single_pct = round(100 * single / len(toks)) if toks else 0
        invis = 0
        fonts = collections.Counter()
        img_area = 0.0
        biggest = 0.0
        nimg = 0
        forms_with_text = set()
        top_forms = 0
        for obj, t, depth, top in walk(page.raw):
            if t == raw.FPDF_PAGEOBJ_TEXT:
                fonts[font_name(obj)] += 1
                if raw.FPDFTextObj_GetTextRenderMode(obj) == raw.FPDF_TEXTRENDERMODE_INVISIBLE:
                    invis += 1
                if top is not None:
                    forms_with_text.add(top)
            elif t == raw.FPDF_PAGEOBJ_IMAGE:
                l, b, r, tp = bounds(obj)
                a = max(0, min(r, w) - max(l, 0)) * max(0, min(tp, h) - max(b, 0)) / (w * h)
                img_area += a
                biggest = max(biggest, a)
                nimg += 1
            elif t == raw.FPDF_PAGEOBJ_FORM and depth == 0:
                top_forms += 1
        widgets = 0
        for k in range(raw.FPDFPage_GetAnnotCount(page.raw)):
            annot = raw.FPDFPage_GetAnnot(page.raw, k)
            if raw.FPDFAnnot_GetSubtype(annot) == raw.FPDF_ANNOT_WIDGET:
                widgets += 1
            raw.FPDFPage_CloseAnnot(annot)
        # stream order vs position: count big upward jumps between consecutive characters
        tp_ = page.get_textpage()
        ups, last_y = 0, None
        for ci in range(tp_.count_chars()):
            ch = tp_.get_text_range(ci, 1)
            if ch.isspace():
                continue
            l, b, r, t = tp_.get_charbox(ci)
            y = (b + t) / 2
            if last_y is not None and y - last_y > 30:  # went back up more than ~2 lines
                ups += 1
            last_y = y
        pats = {k: len(p.findall(text)) for k, p in PATTERNS.items() if p.search(text)}
        top_fonts = ",".join(f"{f[:18]}" for f, _ in fonts.most_common(2))
        orient = "V" if h >= w else "H"
        print(f"{i + 1} | {w:.0f}x{h:.0f}{orient} | {chars} | {own} | {invis} | {single_pct} | {min(img_area, 1) * 100:.0f} (max {biggest * 100:.0f}) | {nimg} | {top_forms}({len(forms_with_text)}) | {widgets} | {ups} | {top_fonts} | {pats}")


if __name__ == "__main__":
    main(sys.argv[1])
