"""A page's text rebuilt from each character's position (rows by height, left to right), versus the plain extraction.

Usage: python texto-por-posicion.py <pdf> <page number>
"""
import os
import sys

import pypdfium2 as pdfium  # noqa: E402

sys.stdout.reconfigure(encoding="utf-8")
pdf = pdfium.PdfDocument(sys.argv[1])
page = pdf[int(sys.argv[2]) - 1]
tp = page.get_textpage()
n = tp.count_chars()
chars = []
for i in range(n):
    ch = tp.get_text_range(i, 1)
    left, bottom, right, top = tp.get_charbox(i)
    if ch.strip() == "" and right - left <= 0:
        continue
    chars.append((round((top + bottom) / 2), left, right, ch))
rows = {}
for y, left, right, ch in chars:
    key = next((k for k in rows if abs(k - y) <= 3), y)
    rows.setdefault(key, []).append((left, right, ch))
print("== plana (como hoy):")
print(" ".join(tp.get_text_range().split())[:400])
print("== por posición:")
for y in sorted(rows, reverse=True):
    line, last = "", None
    for left, right, ch in sorted(rows[y]):
        if last is not None and left - last > 6:
            line += "  "
        line += ch
        last = right
    line = " ".join(line.split("  ")).strip()
    if line:
        print(line)
