"""RapidOCR alone (onnxruntime, the models bundled in the rapidocr wheel) on PDF pages rendered with pdfium.
Usage: python -I rapid_solo.py <pdf> <out_dir> <long_side_px> <pages: 1-23 | 1,4,6>
Writes out_dir/pNN.txt (one detected line per row, top to bottom) and run.json (seconds per page, peak memory)."""
import json, os, resource, sys, time
import pypdfium2 as pdfium
from rapidocr import RapidOCR

pdf_path, out, side, spec = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
pages = [p for part in spec.split(",") for p in (range(int(part.split("-")[0]), int(part.split("-")[1]) + 1) if "-" in part else [int(part)])]
os.makedirs(out, exist_ok=True)
t = time.time(); eng = RapidOCR(); t_init = time.time() - t
pdf = pdfium.PdfDocument(pdf_path)
per = []
for p in pages:
    page = pdf[p - 1]; w, h = page.get_size()
    t = time.time()
    img = page.render(scale=side / max(w, h)).to_numpy()
    t_r = time.time() - t; t = time.time()
    r = eng(img)
    t_o = time.time() - t
    lines = list(r.txts or [])
    open(f"{out}/p{p:02d}.txt", "w", encoding="utf-8").write("\n".join(lines) + "\n")
    per.append({"page": p, "render_s": round(t_r, 2), "ocr_s": round(t_o, 2), "lines": len(lines)})
    print(per[-1], flush=True)
json.dump({"pdf": os.path.basename(pdf_path), "long_side": side, "init_s": round(t_init, 2), "per_page": per,
           "peak_rss_mb": round(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024)}, open(f"{out}/run.json", "w"), indent=1)
