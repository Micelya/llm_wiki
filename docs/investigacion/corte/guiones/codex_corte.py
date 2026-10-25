"""Variants B and C of the cutter: Codex looks at two pages and says whether the
second continues the document of the first or starts another, and of what type.

Usage: CORTE_PYLIB=<folder with pypdfium2 and Pillow> python -I codex_corte.py
           <b|c> <variant A decisions.json> <pdf folder> <work folder> <out.json>

b: only the steps variant A left doubtful; b2: those and the steps decided
only by the shape of the page (WEAK_RULES: another resolution or another paper
size is a hint, not a proof); c: every step. One call at a time.
Answers are kept in <work folder>/codex/ and reused (C reuses B's), with the
seconds each one took. The images are small: the long side at LONG_SIDE pixels.
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

if os.environ.get("CORTE_PYLIB"):
    sys.path.insert(0, os.environ["CORTE_PYLIB"])
import pypdfium2 as pdfium  # noqa: E402

MODEL = "gpt-6-luna"
REASONING = "high"
LONG_SIDE = 1000
TIMEOUT = 420
WEAK_RULES = {"capture", "paper"}

# type_ids_in_use of the criteria of the answer sheets, with a short gloss.
TYPES = {
    "birth_record": "partida o acta de nacimiento",
    "boleta": "boleta de pago",
    "clerk_report": "informe del secretario o actuario en un expediente",
    "constancia-de-firma": "constancia de firma de un servicio de firma",
    "copy_certification": "certificación de copia fiel (suelta)",
    "court_filing": "escrito presentado por una parte en un expediente",
    "court_official_letter": "oficio o cédula librados en un expediente",
    "court_order": "resolución o sentencia con fundamentos",
    "death_record": "partida o acta de defunción",
    "email": "correo electrónico impreso",
    "family_booklet": "libreta de familia",
    "fee_stamp": "comprobante de timbre o tasa (suelto)",
    "ficha-de-cliente": "ficha o perfil de cliente",
    "heirs_declaration": "declaratoria de herederos",
    "identity_document": "documento de identidad",
    "marriage_record": "partida o acta de matrimonio",
    "medical_death_certificate": "certificado médico de defunción",
    "notarial_deed": "escritura o actuación notarial",
    "power_of_attorney": "poder",
    "prosecutor_opinion": "dictamen del fiscal o de un ministerio público",
    "recibo": "recibo",
    "registry_inscription": "nota de inscripción de un registro (suelta)",
    "registry_report": "informe de un registro",
    "registry_request_form": "formulario de pedido a un registro",
    "signature_sheet": "hoja de firmas (suelta)",
    "simple_court_order": "providencia simple de trámite",
    "tax_certificate": "certificado fiscal o de deuda",
    "vehicle_title": "título de un automotor",
}
OTHER = "other"

PROMPT = """Las dos imágenes son dos páginas seguidas de un mismo archivo PDF, que puede juntar varios documentos (por ejemplo, un escrito con su documentación adjunta).
La primera imagen es la página {a} y la segunda, la página {b}.

¿La segunda página sigue el documento de la primera, o empieza otro documento?
Criterios generales:
- Una certificación de copia fiel, un comprobante de timbre o tasa, una hoja de firmas, una nota de inscripción o el dorso de una partida son parte del documento al que se refieren: no empiezan otro.
- Una página en blanco, o con solo el pie, sigue el documento anterior.
- El frente y el dorso de una misma tarjeta o documento de identidad son un documento.
- Cada ejemplar completo de un documento es un documento aparte, aunque sea igual al anterior.
- En un expediente, cada escrito, providencia, oficio, cédula, informe o dictamen con su fecha y firma propias es un documento.

Decí también el tipo del documento de cada página, con uno de estos identificadores (o "other" si ninguno sirve):
{types}

Respondé solo con el JSON pedido."""


def schema():
    enum = sorted(TYPES) + [OTHER]
    return {
        "type": "object",
        "additionalProperties": False,
        "required": ["second_starts_new_document", "first_page_type", "second_page_type", "confidence"],
        "properties": {
            "second_starts_new_document": {"type": "boolean"},
            "first_page_type": {"type": "string", "enum": enum},
            "second_page_type": {"type": "string", "enum": enum},
            "confidence": {"type": "string", "enum": ["high", "medium", "low"]},
        },
    }


def render(pdf_path, page, out):
    if os.path.exists(out):
        return out
    pdf = pdfium.PdfDocument(pdf_path)
    p = pdf[page - 1]
    w, h = p.get_size()
    img = p.render(scale=LONG_SIDE / max(w, h)).to_pil().convert("RGB")
    img.save(out, "JPEG", quality=70)
    return out


def ask(codex, img_a, img_b, a, b):
    assert os.path.isabs(img_a) and os.path.exists(img_a) and os.path.exists(img_b)
    work = tempfile.mkdtemp(prefix="corte-codex-")
    try:
        schema_file = os.path.join(work, "schema.json")
        answer_file = os.path.join(work, "answer.json")
        json.dump(schema(), open(schema_file, "w", encoding="utf-8"))
        args = [codex, "-a", "never", "exec", "-i", img_a, img_b, "--skip-git-repo-check", "--sandbox", "read-only",
                "--ephemeral", "--output-schema", schema_file, "--model", MODEL,
                "-c", f"model_reasoning_effort={REASONING}", "-o", answer_file, "-"]
        types = "\n".join(f"- {k}: {v}" for k, v in sorted(TYPES.items()))
        prompt = PROMPT.format(a=a, b=b, types=types)
        start = time.time()
        try:
            subprocess.run(args, input=prompt.encode("utf-8"), cwd=work, capture_output=True, timeout=TIMEOUT)
        except subprocess.TimeoutExpired:
            return None, time.time() - start
        seconds = time.time() - start
        try:
            return json.load(open(answer_file, encoding="utf-8")), seconds
        except (OSError, ValueError):
            return None, seconds
    finally:
        shutil.rmtree(work, ignore_errors=True)


def asked(mode, s):
    if mode == "c":
        return True
    if mode == "b2" and s["rule"] in WEAK_RULES:
        return True
    return s["decision"] == "doubtful"


def main(mode, a_path, pdf_dir, work, out):
    codex = shutil.which("codex") or shutil.which("codex.cmd")
    # Codex runs in a temporary folder: the images need absolute paths. With a
    # path it cannot find, it does not fail: it answers without seeing them.
    work, pdf_dir = os.path.abspath(work), os.path.abspath(pdf_dir)
    os.makedirs(os.path.join(work, "img"), exist_ok=True)
    os.makedirs(os.path.join(work, "codex"), exist_ok=True)
    variant_a = json.load(open(a_path, encoding="utf-8"))
    result, calls, seconds, failed = [], 0, 0.0, 0
    for f in variant_a:
        stem = f["file"][:2]
        pdf_path = os.path.join(pdf_dir, f["file"])
        answers = {}
        for s in f["steps"]:
            if not asked(mode, s):
                continue
            a = s["after"]
            cache = os.path.join(work, "codex", f"{stem}-{a:03d}.json")
            if os.path.exists(cache):
                saved = json.load(open(cache, encoding="utf-8"))
            else:
                ia = render(pdf_path, a, os.path.join(work, "img", f"{stem}-{a:03d}.jpg"))
                ib = render(pdf_path, a + 1, os.path.join(work, "img", f"{stem}-{a + 1:03d}.jpg"))
                answer, secs = ask(codex, ia, ib, a, a + 1)
                saved = {"answer": answer, "seconds": round(secs, 1)}
                if answer is not None:
                    json.dump(saved, open(cache, "w", encoding="utf-8"), ensure_ascii=False)
                print(f"{stem} {a}->{a + 1} {secs:.0f}s {answer}", flush=True)
            calls += 1
            seconds += saved["seconds"]
            if saved["answer"] is None:
                failed += 1
            else:
                answers[a] = saved["answer"]
        steps, types = [], {}
        for s in f["steps"]:
            ans = answers.get(s["after"])
            if ans is not None and asked(mode, s):
                d = "new" if ans["second_starts_new_document"] else "continue"
                steps.append({"after": s["after"], "decision": d, "rule": "codex"})
            else:
                steps.append({"after": s["after"], "decision": "continue" if s["decision"] == "doubtful" else s["decision"],
                              "rule": s["rule"]})
        starts = [1] + [s["after"] + 1 for s in steps if s["decision"] == "new"]
        for st in starts:
            t = None
            if st - 1 in answers:
                t = answers[st - 1]["second_page_type"]
            elif st in answers:
                t = answers[st]["first_page_type"]
            if mode != "c" and (t is None or t == OTHER):
                t = f["types"].get(str(st)) or t
            types[str(st)] = None if t == OTHER else t
        result.append({"file": f["file"], "pages": f["pages"], "steps": steps, "types": types})
    meta = {"codex_calls": calls, "codex_seconds": seconds, "codex_failed": failed, "model": MODEL, "reasoning": REASONING}
    json.dump({"meta": meta, "files": result}, open(out, "w", encoding="utf-8"), ensure_ascii=False, indent=1)


if __name__ == "__main__":
    main(*sys.argv[1:])
