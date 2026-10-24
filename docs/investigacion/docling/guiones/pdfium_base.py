"""Baseline with pdfium only: plain text and position-rebuilt text per page, page renders, timings.
Usage: python -I pdfium_base.py <pdf> <out_dir> [render_dpi=200]"""
import json, os, sys, time
import pypdfium2 as pdfium

def by_position(tp):
    chars = []
    for i in range(tp.count_chars()):
        ch = tp.get_text_range(i, 1)
        l, b, r, t = tp.get_charbox(i)
        if ch.strip() == "" and r - l <= 0:
            continue
        chars.append((round((t + b) / 2), l, r, ch))
    rows = {}
    for y, l, r, ch in chars:
        key = next((k for k in rows if abs(k - y) <= 3), y)
        rows.setdefault(key, []).append((l, r, ch))
    out = []
    for y in sorted(rows, reverse=True):
        line, last = "", None
        for l, r, ch in sorted(rows[y]):
            if last is not None and l - last > 6:
                line += "  "
            line += ch
            last = r
        line = " ".join(line.split("  ")).strip()
        if line:
            out.append(line)
    return "\n".join(out)

pdf_path, out = sys.argv[1], sys.argv[2]
dpi = int(sys.argv[3]) if len(sys.argv) > 3 else 200
os.makedirs(out, exist_ok=True)
t0 = time.time()
pdf = pdfium.PdfDocument(pdf_path)
stats = []
for i, page in enumerate(pdf):
    t = time.time()
    tp = page.get_textpage()
    plain = tp.get_text_range()
    pos = by_position(tp)
    te = time.time() - t
    open(f"{out}/p{i+1:02d}.plain.txt", "w", encoding="utf-8").write(plain)
    open(f"{out}/p{i+1:02d}.pos.txt", "w", encoding="utf-8").write(pos)
    t = time.time()
    page.render(scale=dpi / 72).to_pil().save(f"{out}/p{i+1:02d}.png")
    stats.append({"page": i + 1, "text_s": round(te, 4), "render_s": round(time.time() - t, 3)})
json.dump({"pages": len(pdf), "total_s": round(time.time() - t0, 3), "per_page": stats}, open(f"{out}/stats.json", "w"))
print(pdf_path, len(pdf), "páginas", round(time.time() - t0, 2), "s")
