"""Per-page text from a Docling JSON, split by each item's provenance (a paragraph that runs across two pages
is split at its charspan). Body and furniture (page header/footer) are kept apart; tables are flattened row by row.
Text that Docling read inside a region it classed as a picture is not in its default Markdown; it is kept here
(pNN.txt) and also counted apart (labels.json, "in_picture"), so both views can be measured.
Usage: python -I por_pagina.py <doc.json> <out_dir>  -> out_dir/pNN.txt (body), pNN.furniture.txt, labels.json"""
import json, os, sys
from collections import defaultdict
from docling_core.types.doc import DoclingDocument, TableItem, TextItem, ContentLayer

doc = DoclingDocument.load_from_json(sys.argv[1])
out = sys.argv[2]; os.makedirs(out, exist_ok=True)
body, furn, labels = defaultdict(list), defaultdict(list), defaultdict(list)
in_pic = set()
def mark(node):
    for ch in node.children:
        in_pic.add(ch.cref); mark(ch.resolve(doc))
for pic in doc.pictures:
    mark(pic)
for item, _lvl in doc.iterate_items(included_content_layers={ContentLayer.BODY, ContentLayer.FURNITURE}, traverse_pictures=True):
    is_furn = getattr(item, "content_layer", None) == ContentLayer.FURNITURE
    if isinstance(item, TableItem):
        rows = []
        for r in item.data.grid:
            rows.append(" | ".join(c.text for c in r))
        text_by_page = {item.prov[0].page_no: "\n".join(rows)} if item.prov else {}
    elif isinstance(item, TextItem):
        text_by_page = {}
        for pv in item.prov:
            s, e = pv.charspan
            part = item.text[s:e] if len(item.prov) > 1 else item.text
            text_by_page[pv.page_no] = text_by_page.get(pv.page_no, "") + part
    else:
        continue
    for p, t in text_by_page.items():
        (furn if is_furn else body)[p].append(t)
        labels[p].append({"label": str(item.label.value if hasattr(item.label, "value") else item.label), "furniture": is_furn, "in_picture": item.self_ref in in_pic,
                          "chars": len(t), "y_top": round(item.prov[0].bbox.t, 1) if item.prov else None})
for p in doc.pages:
    open(f"{out}/p{p:02d}.txt", "w", encoding="utf-8").write("\n".join(body.get(p, [])))
    open(f"{out}/p{p:02d}.furniture.txt", "w", encoding="utf-8").write("\n".join(furn.get(p, [])))
json.dump({str(k): v for k, v in labels.items()}, open(f"{out}/labels.json", "w"), indent=0)
print(out, len(doc.pages), "páginas")
