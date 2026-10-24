"""Text and page images of a PDF, for reading it by eye. Usage: python pdf-a-texto.py <pdf> <out_dir>"""
import os
import sys

import pypdfium2 as pdfium  # noqa: E402

sys.stdout.reconfigure(encoding="utf-8")
pdf_path, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
pdf = pdfium.PdfDocument(pdf_path)
print("páginas:", len(pdf))
for i, page in enumerate(pdf):
    text = page.get_textpage().get_text_range()
    print(f"--- página {i + 1} ({len(text)} caracteres)")
    print(text)
    page.render(scale=1.4).to_pil().convert("L").save(os.path.join(out, f"p{i + 1:02d}.png"))
