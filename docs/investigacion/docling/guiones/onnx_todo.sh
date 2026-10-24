#!/bin/bash
# Docling without PyTorch on the six documents (full-page OCR and default), one process each
cd "$(dirname "$0")/.." && . venv-onnx/bin/activate
for mode in full_page default; do
  for f in documentos/0{1,3,5,6,2,4}*.pdf; do
    b=$(basename $f .pdf); o=resultados/docling-onnx/$mode/$b; mkdir -p $o
    [ -f $o/run.json ] || timeout 3600 python -I guiones/docling_onnx_run.py $f $o $mode >> resultados/docling-onnx.log 2>$o.err || echo "FAIL $mode $b" >> resultados/docling-onnx.log
  done
done
echo DONE >> resultados/docling-onnx.log
