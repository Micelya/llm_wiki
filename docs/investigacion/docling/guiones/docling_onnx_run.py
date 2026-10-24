"""Docling without PyTorch: layout model on onnxruntime (docling-layout-heron-onnx), RapidOCR on onnxruntime,
table structure off (TableFormer needs PyTorch). Docling's device check is replaced by 'always CPU' because it imports PyTorch. Usage: python -I docling_onnx_run.py <pdf> <out_dir> <default|full_page>"""
import json, os, resource, sys, time
from pathlib import Path
t0 = time.time()
# Two table plugins (TableFormer V2 and Granite Vision) import torch when the plugin list is built, even with
# tables off. They are replaced by empty modules exposing a class of the same name that is never used.
import types
from docling.models.stages.table_structure.table_structure_model import TableStructureModel as _TS
for _mod, _cls in (("docling.models.stages.table_structure.table_structure_model_v2", "TableStructureModelV2"),
                   ("docling.models.stages.table_structure.table_structure_model_granite_vision", "GraniteVisionTableStructureModel")):
    _m = types.ModuleType(_mod); setattr(_m, _cls, type(_cls, (_TS,), {})); sys.modules[_mod] = _m
from docling.datamodel.base_models import InputFormat
from docling.datamodel.pipeline_options import PdfPipelineOptions, RapidOcrOptions, OcrMode, LayoutObjectDetectionOptions
from docling.datamodel.object_detection_engine_options import OnnxRuntimeObjectDetectionEngineOptions
from docling.datamodel.settings import settings
from docling.document_converter import DocumentConverter, PdfFormatOption
import docling.utils.accelerator_utils as _acc
def _cpu_only(accelerator_device, supported_devices=None):
    return "cpu"
# Docling's device check imports PyTorch even for onnxruntime models; without PyTorch it must be replaced.
_acc.decide_device.__code__ = _cpu_only.__code__
# The layout preprocessor is loaded with AutoImageProcessor, which picks the torchvision variant; force the PIL one.
import docling.models.inference_engines.common.hf_vision_base as _hv
def _pil_preprocessor(self, model_folder):
    from transformers.models.rt_detr.image_processing_pil_rt_detr import RTDetrImageProcessorPil
    return RTDetrImageProcessorPil.from_pretrained(str(model_folder))
_hv.HfVisionModelMixin._load_preprocessor = _pil_preprocessor
t_import = time.time() - t0
settings.debug.profile_pipeline_timings = True
pdf, out, mode = sys.argv[1:4]; out = Path(out); out.mkdir(parents=True, exist_ok=True)
opts = PdfPipelineOptions(do_table_structure=False)
opts.layout_options = LayoutObjectDetectionOptions(engine_options=OnnxRuntimeObjectDetectionEngineOptions())
opts.ocr_options = RapidOcrOptions(lang=["iso:es"], mode=OcrMode.FULL_PAGE if mode == "full_page" else OcrMode.DEFAULT)
t = time.time()
conv = DocumentConverter(format_options={InputFormat.PDF: PdfFormatOption(pipeline_options=opts)})
res = conv.convert(pdf, raises_on_error=False)
wall = time.time() - t
(out / "doc.md").write_text(res.document.export_to_markdown(), encoding="utf-8")
res.document.save_as_json(out / "doc.json")
import sys as _s
run = {"pdf": os.path.basename(pdf), "mode": mode, "status": str(res.status), "pages": len(res.document.pages), "import_s": round(t_import, 2),
       "wall_s": round(wall, 2), "peak_rss_mb": round(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024),
       "torch_imported": "torch" in _s.modules, "timings": {k: round(sum(v.times), 3) for k, v in res.timings.items()},
       "errors": [str(e) for e in res.errors]}
(out / "run.json").write_text(json.dumps(run, indent=1))
print(json.dumps({k: run[k] for k in ("pdf", "mode", "status", "pages", "wall_s", "peak_rss_mb", "torch_imported", "import_s")}))
