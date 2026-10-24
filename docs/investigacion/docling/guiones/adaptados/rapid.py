"""OCR with RapidOCR (PP-OCR models on onnxruntime), offline after the first model download.
Usage: python -I rapid.py <png> <out.txt>"""
import os, sys, time
from rapidocr import RapidOCR
eng = RapidOCR()
for src, dst in zip(sys.argv[1::2], sys.argv[2::2]):
    t = time.time(); r = eng(src)
    txts = list(r.txts or [])
    with open(dst, "w", encoding="utf-8") as f:
        f.write(f"# ms={int((time.time()-t)*1000)} lines={len(txts)}\n" + "\n".join(txts) + "\n")
    print(os.path.basename(src), len(txts), "renglones", int((time.time()-t)*1000), "ms")
