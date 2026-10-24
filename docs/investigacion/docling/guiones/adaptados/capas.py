"""Top-level objects of a page and what text each top-level form XObject holds. Usage: python -I capas.py <pdf> <page>"""
import collections, ctypes, os, sys
import pypdfium2 as pdfium, pypdfium2.raw as raw
sys.stdout.reconfigure(encoding="utf-8")
pdf = pdfium.PdfDocument(sys.argv[1]); page = pdf[int(sys.argv[2]) - 1]
tp = page.get_textpage()
def text_of(obj):
    n = raw.FPDFTextObj_GetText(obj, tp.raw, None, 0)
    buf = (ctypes.c_ushort * (n // 2 + 1))()
    raw.FPDFTextObj_GetText(obj, tp.raw, buf, n)
    return bytes(buf)[: max(0, n - 2)].decode("utf-16-le", "replace")
kinds = collections.Counter()
for i in range(raw.FPDFPage_CountObjects(page.raw)):
    obj = raw.FPDFPage_GetObject(page.raw, i); t = raw.FPDFPageObj_GetType(obj); kinds[t] += 1
    if t == raw.FPDF_PAGEOBJ_FORM:
        texts = []
        def rec(h):
            for j in range(raw.FPDFFormObj_CountObjects(h)):
                o = raw.FPDFFormObj_GetObject(h, j); tt = raw.FPDFPageObj_GetType(o)
                if tt == raw.FPDF_PAGEOBJ_TEXT: texts.append(text_of(o))
                elif tt == raw.FPDF_PAGEOBJ_FORM: rec(o)
        rec(obj)
        print(f"objeto {i}: grupo (form XObject) con {raw.FPDFFormObj_CountObjects(obj)} objetos, {len(texts)} de texto; muestra: {[t[:25] for t in texts[:8]]}")
print("objetos de primer nivel por tipo (1 texto, 2 trazo, 3 imagen, 4 sombreado, 5 grupo):", dict(kinds))
