"""Run Docling's standard PDF pipeline on one PDF with one OCR configuration, and record what it decided to OCR.
Usage: python -I docling_run.py <pdf> <out_dir> <engine: auto|rapidocr|tesserocr|easyocr|none> <mode: default|full_page> [pages e.g. 1-5]
Writes out_dir/doc.md, doc.json, pNN.md per page, run.json (timings, OCR areas per page, peak memory)."""
import json, os, resource, sys, time
from pathlib import Path

pdf, out, engine, mode = sys.argv[1:5]
prange = tuple(map(int, sys.argv[5].split("-"))) if len(sys.argv) > 5 else None
out = Path(out); out.mkdir(parents=True, exist_ok=True)
t_import = time.time()
from docling.datamodel.base_models import InputFormat
from docling.datamodel.pipeline_options import (PdfPipelineOptions, OcrMode, RapidOcrOptions, TesseractOcrOptions,
                                                EasyOcrOptions, OcrAutoOptions)
from docling.datamodel.settings import settings
from docling.document_converter import DocumentConverter, PdfFormatOption
from docling.models.base_ocr_model import BaseOcrModel
t_import = time.time() - t_import
settings.debug.profile_pipeline_timings = True

ocr_mode = OcrMode.FULL_PAGE if mode == "full_page" else OcrMode.DEFAULT
opts = PdfPipelineOptions()
if engine == "none":
    opts.do_ocr = False
else:
    cls = {"auto": OcrAutoOptions, "rapidocr": RapidOcrOptions, "tesserocr": TesseractOcrOptions, "easyocr": EasyOcrOptions}[engine]
    kw = {"mode": ocr_mode}
    if engine != "auto":
        kw["lang"] = {"rapidocr": ["iso:es"], "tesserocr": ["spa"], "easyocr": ["es"]}[engine]
    opts.ocr_options = cls(**kw)

# record the regions sent to OCR, per page, as a share of the page area
ocr_log = {}
_orig = BaseOcrModel.get_ocr_rects
def logged(self, page):
    rects = _orig(self, page)
    area = page.size.width * page.size.height
    ocr_log[page.page_no] = {"n_rects": len(rects), "area_share": round(sum(r.area() for r in rects) / area, 3),
                                 "engine": type(self).__name__}
    return rects
BaseOcrModel.get_ocr_rects = logged

t0 = time.time()
conv = DocumentConverter(format_options={InputFormat.PDF: PdfFormatOption(pipeline_options=opts)})
kw = {"page_range": prange} if prange else {}
res = conv.convert(pdf, raises_on_error=False, **kw)
t_init_conv = time.time() - t0
doc = res.document
(out / "doc.md").write_text(doc.export_to_markdown(), encoding="utf-8")
doc.save_as_json(out / "doc.json")
pages = sorted(doc.pages.keys())
for p in pages:
    (out / f"p{p:02d}.md").write_text(doc.export_to_markdown(page_no=p), encoding="utf-8")
timings = {k: {"total_s": round(sum(v.times), 3), "count": len(v.times)} for k, v in res.timings.items()}
used = [type(m).__name__ for m in getattr(conv._get_pipeline(InputFormat.PDF), "build_pipe", [])] if hasattr(conv, "_get_pipeline") else []
run = {"pdf": os.path.basename(pdf), "engine": engine, "mode": mode, "status": str(res.status), "pages": len(pages),
       "import_s": round(t_import, 2), "wall_s": round(t_init_conv, 2), "peak_rss_mb": round(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024),
       "timings": timings, "ocr_regions": ocr_log, "errors": [str(e) for e in res.errors]}
(out / "run.json").write_text(json.dumps(run, indent=1, ensure_ascii=False), encoding="utf-8")
print(json.dumps({k: run[k] for k in ("pdf", "engine", "mode", "status", "pages", "wall_s", "peak_rss_mb")}))
