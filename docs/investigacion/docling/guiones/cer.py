"""Character error rate of a tool's page text against a private reference passage typed by eye from the image.
Spaces, hyphens and dashes are ignored (line breaks and hyphenation differ by layout); case and accents count.
The passage is located in the tool's text with a fuzzy alignment, then the Levenshtein distance is divided by
the passage length. Also counts accented letters (á é í ó ú ñ) of the passage that the tool kept. Prints numbers only.
Usage: python -I cer.py <answers.json> <passage key> <text>..."""
import json, re, sys
from rapidfuzz import fuzz
from rapidfuzz.distance import Levenshtein
def norm(s): return re.sub(r"[\s\-–—_|#*]", "", s)
ref = norm(json.load(open(sys.argv[1], encoding="utf-8"))[sys.argv[2]])
acc = [c for c in ref if c in "áéíóúñÁÉÍÓÚÑ"]
for path in sys.argv[3:]:
    t = norm(open(path, encoding="utf-8").read())
    if not t:
        print(json.dumps({"archivo": path, "cer_pct": 100.0})); continue
    al = fuzz.partial_ratio_alignment(ref, t)
    s, e = max(0, al.dest_start - 20), min(len(t), al.dest_end + 20)
    best = min((Levenshtein.distance(ref, t[i:j]), i, j) for i in range(s, min(s + 41, e)) for j in range(max(i + 1, e - 40), e + 1))
    win = t[best[1]:best[2]]
    kept = sum(1 for c in acc if c in win)
    print(json.dumps({"archivo": path, "caracteres": len(ref), "cer_pct": round(100 * best[0] / len(ref), 1),
                      "tildes_en_pasaje": len(acc), "tildes_presentes": min(kept, len(acc))}))
