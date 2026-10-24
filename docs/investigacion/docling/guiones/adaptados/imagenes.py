"""Per page: size, and the biggest image's pixel size, dpi, colour space and filter. Usage: python -I imagenes.py <pdf>"""
import os, sys
import pypdfium2 as pdfium, pypdfium2.raw as raw
sys.stdout.reconfigure(encoding="utf-8")
CS = {1: "gris", 2: "RGB", 3: "CMYK", 4: "calgris", 5: "calRGB", 6: "lab", 7: "icc", 11: "indexado", 0: "?"}
pdf = pdfium.PdfDocument(sys.argv[1])
for i, page in enumerate(pdf):
    w, h = page.get_size()
    best = None
    for obj in page.get_objects(filter=[raw.FPDF_PAGEOBJ_IMAGE], max_depth=3):
        try:
            md = obj.get_metadata(); px = md.width * md.height
            if not best or px > best[0]:
                best = (px, md.width, md.height, round(md.horizontal_dpi), CS.get(md.colorspace, md.colorspace), md.bits_per_pixel, obj.get_filters())
        except Exception as e:
            pass
    print(i + 1, f"{w:.0f}x{h:.0f}", best[1:] if best else "-")
