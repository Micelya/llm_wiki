"""Contact sheets of a PDF (several small pages per image) plus each page's text length and first words.

Usage: python hojas.py <pdf> <out_dir> [per_sheet=6] [width=520]
"""
import os
import sys

import pypdfium2 as pdfium  # noqa: E402
from PIL import Image, ImageDraw  # noqa: E402

sys.stdout.reconfigure(encoding="utf-8")
pdf_path, out = sys.argv[1], sys.argv[2]
per = int(sys.argv[3]) if len(sys.argv) > 3 else 6
width = int(sys.argv[4]) if len(sys.argv) > 4 else 520
os.makedirs(out, exist_ok=True)
pdf = pdfium.PdfDocument(pdf_path)
n = len(pdf)
print("páginas:", n)
thumbs = []
for i, page in enumerate(pdf):
    text = page.get_textpage().get_text_range()
    chars = len([c for c in text if not c.isspace()])
    first = " ".join(text.split())[:110]
    print(f"p{i + 1}: {chars} caracteres | {first}")
    w, h = page.get_size()
    img = page.render(scale=width / w).to_pil().convert("L")
    img.save(os.path.join(out, f"p{i + 1:02d}.png"))
    thumbs.append(img)
cols = 3
for s in range(0, n, per):
    group = thumbs[s : s + per]
    rows = (len(group) + cols - 1) // cols
    cell_h = max(t.height for t in group)
    sheet = Image.new("L", (cols * width, rows * (cell_h + 22)), 255)
    draw = ImageDraw.Draw(sheet)
    for k, t in enumerate(group):
        x, y = (k % cols) * width, (k // cols) * (cell_h + 22)
        sheet.paste(t, (x, y + 22))
        draw.text((x + 6, y + 4), f"p{s + k + 1}", fill=0)
    sheet.save(os.path.join(out, f"hoja-{s // per + 1:02d}.png"))
