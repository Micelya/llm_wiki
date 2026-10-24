"""Cold start: import Docling, build the converter and load its models (layout, OCR, tables), then convert one page.
Run with HF_HUB_OFFLINE=1 to prove it works without network once the models are on disk.
Usage: python -I arranque.py <torch|onnx> <pdf>"""
import json, resource, sys, time
t0 = time.time()
kind, pdf = sys.argv[1], sys.argv[2]
if kind == "onnx":
    import types
    from docling.models.stages.table_structure.table_structure_model import TableStructureModel as _TS
    for _mod, _cls in (("docling.models.stages.table_structure.table_structure_model_v2", "TableStructureModelV2"),
                       ("docling.models.stages.table_structure.table_structure_model_granite_vision", "GraniteVisionTableStructureModel")):
        _m = types.ModuleType(_mod); setattr(_m, _cls, type(_cls, (_TS,), {})); sys.modules[_mod] = _m
    import docling.utils.accelerator_utils as _acc
    def _cpu_only(accelerator_device, supported_devices=None):
        return "cpu"
    _acc.decide_device.__code__ = _cpu_only.__code__
    import docling.models.inference_engines.common.hf_vision_base as _hv
    def _pil(self, model_folder):
        from transformers.models.rt_detr.image_processing_pil_rt_detr import RTDetrImageProcessorPil
        return RTDetrImageProcessorPil.from_pretrained(str(model_folder))
    _hv.HfVisionModelMixin._load_preprocessor = _pil
from docling.datamodel.base_models import InputFormat
from docling.datamodel.pipeline_options import PdfPipelineOptions, RapidOcrOptions, OcrMode
from docling.document_converter import DocumentConverter, PdfFormatOption
t_import = time.time() - t0
opts = PdfPipelineOptions(do_table_structure=(kind == "torch"))
opts.ocr_options = RapidOcrOptions(lang=["iso:es"], mode=OcrMode.FULL_PAGE)
if kind == "onnx":
    from docling.datamodel.pipeline_options import LayoutObjectDetectionOptions
    from docling.datamodel.object_detection_engine_options import OnnxRuntimeObjectDetectionEngineOptions
    opts.layout_options = LayoutObjectDetectionOptions(engine_options=OnnxRuntimeObjectDetectionEngineOptions())
conv = DocumentConverter(format_options={InputFormat.PDF: PdfFormatOption(pipeline_options=opts)})
t = time.time(); conv.initialize_pipeline(InputFormat.PDF); t_models = time.time() - t
t = time.time(); r = conv.convert(pdf, page_range=(1, 1)); t_page = time.time() - t
print(json.dumps({"tipo": kind, "importar_s": round(t_import, 2), "cargar_modelos_s": round(t_models, 2),
                  "primera_pagina_s": round(t_page, 2), "total_s": round(time.time() - t0, 2), "estado": str(r.status),
                  "memoria_max_mb": round(resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1024)}))
