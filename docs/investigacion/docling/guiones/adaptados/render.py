"""Render pages to grayscale PNG with a given long side. Usage: python -I render.py <pdf> <prefix> <long_side> <pages...>"""
import os, sys
import pypdfium2 as pdfium
pdf = pdfium.PdfDocument(sys.argv[1]); prefix = sys.argv[2]; side = int(sys.argv[3])
for p in map(int, sys.argv[4:]):
    page = pdf[p - 1]; w, h = page.get_size()
    page.render(scale=side / max(w, h)).to_pil().convert("L").save(f"{prefix}-p{p:02d}.png")
    print("ok", p)
