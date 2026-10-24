"""Handwritten values of a page versus a private list read by eye from the image. For each value it finds the most
similar stretch of the tool's text (letters and digits only, no accents) and counts it as exact (100 %) or close
(85 % or more). Prints counts only.  Usage: python -I manuscrito.py <answers.json> <page key> <text>..."""
import difflib, json, re, sys, unicodedata
def norm(s):
    s = "".join(c for c in unicodedata.normalize("NFKD", s) if not unicodedata.combining(c))
    return re.sub(r"[^a-z0-9]", "", s.lower())
vals = json.load(open(sys.argv[1], encoding="utf-8"))[sys.argv[2]]
for path in sys.argv[3:]:
    t = norm(open(path, encoding="utf-8").read())
    exact = close = 0
    for v in map(norm, vals):
        if v in t:
            exact += 1; close += 1; continue
        L = len(v); best = 0
        for i in range(0, max(1, len(t) - L + 1)):
            r = difflib.SequenceMatcher(None, v, t[i:i + L]).ratio()
            if r > best: best = r
        if best >= 0.85: close += 1
    print(json.dumps({"archivo": path, "valores": len(vals), "exactos": exact, "parecidos_85": close}))
