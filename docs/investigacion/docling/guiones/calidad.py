"""Text quality numbers without a reference transcription (prints numbers only, never text).
- conocidas: share of words (3+ letters) found in a 200k Spanish word list (wordfreq, Apache-2.0)
- sueltas / largas / sin_vocal: same idea as basura.py (single letters, 15+ letters, 3+ letters with no vowel)
- con_tilde: words carrying á é í ó ú ñ ü that are real words; sin_tilde: words written without their accent
  where the accented spelling is a much more common word (e.g. 'defuncion' for 'defunción')
Usage: python -I calidad.py <file>...  -> one JSON object per line"""
import json, re, sys, unicodedata
from wordfreq import top_n_list, zipf_frequency

def strip(s):
    return "".join(c for c in unicodedata.normalize("NFKD", s) if not unicodedata.combining(c))

WORDS = top_n_list("es", 200000)
KNOWN = set(WORDS)
ACCENTED = {}
for w in WORDS:
    if w != strip(w):
        ACCENTED.setdefault(strip(w), w)
VOW = set("aeiouáéíóúü")

def measure(t):
    t = re.sub(r"<!--.*?-->", " ", t)
    t = re.sub(r"[|#*_`]", " ", t)
    toks = re.findall(r"[^\W\d_]+", t)
    if not toks:
        return {"palabras": 0}
    low = [w.lower() for w in toks]
    longer = [w for w in low if len(w) >= 3]
    known = sum(1 for w in longer if w in KNOWN)
    with_acc = [w for w in low if w != strip(w)]
    acc_ok = sum(1 for w in with_acc if w in KNOWN)
    missing = 0
    for w in low:
        if w == strip(w) and w in ACCENTED:
            a = ACCENTED[w]
            if zipf_frequency(a, "es") - zipf_frequency(w, "es") > 1.0:
                missing += 1
    n = len(toks)
    return {"palabras": n, "conocidas_pct": round(100 * known / max(1, len(longer)), 1),
            "sueltas_pct": round(100 * sum(1 for w in toks if len(w) == 1) / n, 1),
            "largas": sum(1 for w in toks if len(w) >= 15),
            "sin_vocal": sum(1 for w in longer if not (set(w) & VOW)),
            "con_tilde_ok": acc_ok, "con_tilde_raras": len(with_acc) - acc_ok, "tilde_perdida": missing}

if __name__ == "__main__":
    for p in sys.argv[1:]:
        print(json.dumps({"archivo": p, **measure(open(p, encoding="utf-8").read())}))
