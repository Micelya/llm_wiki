"""Variant A of the cutter: rules only, no model.

Usage: python -I cortar.py <senales.json> <out.json>

For every step from one page to the next it says "continue", "new" or
"doubtful" and the rule that decided; for every document it starts, the type
the vocabulary finds in its first page (or none). Rules, in order (the first
that applies decides):
  1. blank: the next page is blank or has only the footer -> continue.
  2. page_of: "página x de y" on the next page with x > 1 -> continue;
     x = 1, or the page before was the last ("y de y") -> new.
  3. doc_code: both pages carry the same document code -> continue;
     both carry codes and share none -> new.
  4. serial: correlative notarial sheet serials (same letters, number + 1) -> continue.
  5. photo_pair: two photographed pages (less than 100 dpi) with the same page
     size and the same image pixels: the two sides of a card -> continue.
  6. making: one page is a scan and the other is digital -> new.
  7. capture: two scans with a different resolution (more than 10 %) or a
     different color space -> new (another capture, another document).
  8. paper: two scans of the same resolution whose page differs in
     orientation or by more than 4 % in width or height -> new (at the same
     resolution the page is the paper, and another paper is another document).
  9. closer / opener: the page before ends with a closing formula, or the
     next one starts with an opening formula (vocabulario.json) -> new.
 10. otherwise -> doubtful (counted as continue in variant A).
"""
import json
import os
import sys
import unicodedata

HERE = os.path.dirname(os.path.abspath(__file__))
VOCAB = json.load(open(os.path.join(HERE, "vocabulario.json"), encoding="utf-8"))
SCANS = {"scan", "scan_old_ocr", "picture"}
DIGITAL = {"digital_prose", "digital_form", "flattened_form"}


def fold(s):
    s = unicodedata.normalize("NFD", s.lower())
    return "".join(c for c in s if unicodedata.category(c) != "Mn")


def lines(p):
    return [l for l in fold(p.get("text", "")).splitlines() if l.strip()]


def has_any(text, phrases):
    return next((x for x in phrases if x in text), None)


def type_of(p):
    if p.get("mrz"):
        return "identity_document"
    text = fold(p.get("text", ""))
    for type_id, phrases in VOCAB["types"]["rules"]:
        if has_any(text, phrases):
            return type_id
    return None


def serial_next(a, b):
    def parse(s):
        letters = s.rstrip("0123456789")
        return letters, int(s[len(letters):])
    return any(la == lb and nb == na + 1 for la, na in map(parse, a) for lb, nb in map(parse, b))


def differs(a, b, share):
    return abs(a - b) > share * max(a, b)


def step(prev, nxt):
    """(decision, rule) for the step from page prev to page nxt."""
    if nxt["shape"] == "blank":
        return "continue", "blank"
    if nxt["page_of"]:
        x, _ = nxt["page_of"][0]
        if x > 1:
            return "continue", "page_of"
        return "new", "page_of"
    if prev["page_of"] and prev["page_of"][-1][0] == prev["page_of"][-1][1] and prev["page_of"][-1][1] > 1:
        return "new", "page_of"
    if prev["doc_codes"] and nxt["doc_codes"]:
        if set(prev["doc_codes"]) & set(nxt["doc_codes"]):
            return "continue", "doc_code"
        return "new", "doc_code"
    if serial_next(prev["sheet_serials"], nxt["sheet_serials"]):
        return "continue", "serial"
    a, b = prev.get("main_image") or {}, nxt.get("main_image") or {}
    both_scans = prev["shape"] in SCANS and nxt["shape"] in SCANS and a and b
    if both_scans and a["dpi"] < 100 and b["dpi"] < 100 and a["px"] == b["px"] \
            and (prev["w"], prev["h"]) == (nxt["w"], nxt["h"]):
        return "continue", "photo_pair"
    if (prev["shape"] in SCANS and nxt["shape"] in DIGITAL) or (prev["shape"] in DIGITAL and nxt["shape"] in SCANS):
        return "new", "making"
    if both_scans:
        if differs(a["dpi"], b["dpi"], 0.10) or a["cs"] != b["cs"]:
            return "new", "capture"
        landscape = (prev["w"] > prev["h"]) != (nxt["w"] > nxt["h"])
        if landscape or differs(prev["w"], nxt["w"], 0.04) or differs(prev["h"], nxt["h"], 0.04):
            return "new", "paper"
    tail, head = lines(prev)[-4:], lines(nxt)[:4]
    if prev["shape"] != "blank" and has_any(" ".join(tail), VOCAB["closers"]["phrases"]):
        return "new", "closer"
    if has_any(" ".join(head), VOCAB["openers"]["phrases"]):
        return "new", "opener"
    return "doubtful", "none"


def cut_file(f):
    pages = f["pages"]
    steps = []
    for i in range(len(pages) - 1):
        d, rule = step(pages[i], pages[i + 1])
        steps.append({"after": i + 1, "decision": d, "rule": rule})
    starts = [1] + [s["after"] + 1 for s in steps if s["decision"] == "new"]
    types = {str(s): type_of(pages[s - 1]) for s in starts}
    return {"file": f["file"], "pages": len(pages), "steps": steps, "types": types}


def main(src, out):
    files = json.load(open(src, encoding="utf-8"))
    json.dump([cut_file(f) for f in files], open(out, "w", encoding="utf-8"), ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
