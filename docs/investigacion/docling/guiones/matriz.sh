#!/bin/bash
# Run every Docling configuration on every document, one process each (so peak memory is per run).
# Engines: RapidOCR, Tesseract (system "fast" Spanish model and tessdata_best), EasyOCR, and Docling's automatic choice;
# OCR modes: default (Docling decides the regions) and full_page. Skips runs that already have run.json.
# Expects: ./venv (Docling with CPU PyTorch), ./documentos/*.pdf, ./tessdata-best/spa.traineddata
cd "$(dirname "$0")/.." && . venv/bin/activate
for cfg in rapidocr:default tesserocr:default easyocr:default rapidocr:full_page tesserocr:full_page easyocr:full_page auto:default \
           tessbest:full_page tessbest:default; do
  eng=${cfg%%:*}; mode=${cfg##*:}
  if [ "$eng" = tessbest ]; then export TESSDATA_PREFIX=$PWD/tessdata-best; run_eng=tesserocr
  else export TESSDATA_PREFIX=/usr/share/tesseract-ocr/5/tessdata; run_eng=$eng; fi
  for f in documentos/0{1,3,5,6,2,4}*.pdf; do
    b=$(basename "$f" .pdf); o=resultados/docling/$eng-$mode/$b; mkdir -p "resultados/docling/$eng-$mode"
    [ -f "$o/run.json" ] && continue
    timeout 3600 python -I guiones/docling_run.py "$f" "$o" "$run_eng" "$mode" >> resultados/docling/matriz.log 2>"$o.err" \
      || echo "FAIL $eng $mode $b" >> resultados/docling/matriz.log
  done
done
echo DONE >> resultados/docling/matriz.log
