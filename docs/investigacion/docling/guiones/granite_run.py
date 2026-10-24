"""Docling VLM pipeline with granite-docling-258M (transformers, CPU) on single pages.
Usage: python -I granite_run.py <out_dir> <pdf>:<page> ...  -> out_dir/<pdf stem>-pNN.{md,json,run.json}"""
import json, os, resource, sys, time
from pathlib import Path
from docling.datamodel.base_models import InputFormat
from docling.datamodel.pipeline_options import VlmPipelineOptions, VlmConvertOptions
from docling.document_converter import DocumentConverter, PdfFormatOption
from docling.pipeline.vlm_pipeline import VlmPipeline

out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
opts = VlmPipelineOptions(vlm_options=VlmConvertOptions.from_preset("granite_docling"))
conv = DocumentConverter(format_options={InputFormat.PDF: PdfFormatOption(pipeline_cls=VlmPipeline, pipeline_options=opts)})
for spec in sys.argv[2:]:
    pdf, page = spec.rsplit(":", 1); page = int(page)
    stem = f"{Path(pdf).stem[:2]}-p{page:02d}"
    t = time.time()
    res = conv.convert(pdf, page_range=(page, page), raises_on_error=False)
    dt = time.time() - t
    md = res.document.export_to_markdown()
    (out / f"{stem}.md").write_text(md, encoding="utf-8")
    res.document.save_as_json(out / f"{stem}.json")
    run = {"page": stem, "status": str(res.status), "seconds": round(dt, 1), "chars": len(md),
           "peak_rss_mb": round(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024)}
    (out / f"{stem}.run.json").write_text(json.dumps(run))
    print(json.dumps(run), flush=True)
