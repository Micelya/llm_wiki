"""Rough 'does this read like words' score of a text, without a dictionary. Usage: python -I basura.py <txt>..."""
import re, sys
sys.stdout.reconfigure(encoding="utf-8")
V = set("aeiouáéíóúü")
for p in sys.argv[1:]:
    t = open(p, encoding="utf-8-sig").read().split("\n", 1)[-1]
    toks = re.findall(r"[^\W\d_]+", t)
    if not toks: print(p.split("/")[-1], "sin palabras"); continue
    long_ = sum(1 for w in toks if len(w) >= 15)
    novow = sum(1 for w in toks if len(w) >= 3 and not (set(w.lower()) & V))
    single = sum(1 for w in toks if len(w) == 1)
    bad = (long_ + novow + single) / len(toks)
    print(f"{p.split('/')[-1]}: {len(toks)} palabras | largas {long_} | sin vocal {novow} | sueltas {single} | dudosas {bad:.0%}")
