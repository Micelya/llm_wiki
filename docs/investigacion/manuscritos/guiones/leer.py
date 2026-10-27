"""Lee las páginas de prueba con un modelo y guarda el texto, el tiempo y la memoria.

Uso: python leer.py <rapidocr|paddleocr-vl|qwen3-vl> <clave>...
Imágenes en img/<clave>.png (200 ppp, lado mayor <= 2200 px). Los modelos de
visión reciben la imagen con el lado mayor <= 1600 px, para acotar el cómputo.
Salida: txt/<modelo>/<clave>.txt y txt/<modelo>/<clave>.json. Si ya existe, la saltea.
"""
import json
import os
import resource
import sys
import time

from PIL import Image

BASE = os.path.expanduser("~/prueba-ms")
PROMPT = ("Transcribe all the text in this image exactly as written, including the "
          "handwritten parts, in reading order. Output only the text.")
MAX_VLM = 1600


def imagen(clave, max_lado=None):
    im = Image.open(os.path.join(BASE, "img", f"{clave}.png")).convert("RGB")
    if max_lado and max(im.size) > max_lado:
        s = max_lado / max(im.size)
        im = im.resize((round(im.width * s), round(im.height * s)), Image.LANCZOS)
    return im


def cargar(nombre):
    if nombre == "rapidocr":
        from rapidocr import RapidOCR
        eng = RapidOCR()
        return lambda clave: "\n".join(eng(os.path.join(BASE, "img", f"{clave}.png")).txts or [])
    import torch
    from transformers import AutoModelForCausalLM, AutoModelForImageTextToText, AutoProcessor
    torch.set_num_threads(4)
    repo = {"paddleocr-vl": "PaddlePaddle/PaddleOCR-VL", "qwen3-vl": "Qwen/Qwen3-VL-2B-Instruct"}[nombre]
    remoto = nombre == "paddleocr-vl"  # código del repositorio del modelo, autorizado por el dueño
    proc = AutoProcessor.from_pretrained(repo, trust_remote_code=remoto)
    clase = AutoModelForCausalLM if remoto else AutoModelForImageTextToText
    model = clase.from_pretrained(repo, torch_dtype=torch.float32, trust_remote_code=remoto).eval()
    texto = "OCR:" if nombre == "paddleocr-vl" else PROMPT

    def leer(clave):
        msgs = [{"role": "user", "content": [{"type": "image", "image": imagen(clave, MAX_VLM)},
                                             {"type": "text", "text": texto}]}]
        inp = proc.apply_chat_template(msgs, add_generation_prompt=True, tokenize=True,
                                       return_dict=True, return_tensors="pt")
        with torch.inference_mode():
            out = model.generate(**inp, max_new_tokens=2048, do_sample=False)
        return proc.batch_decode(out[:, inp["input_ids"].shape[1]:], skip_special_tokens=True)[0]
    return leer


def main():
    nombre, claves = sys.argv[1], sys.argv[2:]
    d = os.path.join(BASE, "txt", nombre)
    os.makedirs(d, exist_ok=True)
    t = time.time()
    leer = cargar(nombre)
    carga = time.time() - t
    for clave in claves:
        if os.path.exists(os.path.join(d, f"{clave}.txt")):
            continue
        t = time.time()
        txt = leer(clave)
        dt = time.time() - t
        open(os.path.join(d, f"{clave}.txt"), "w", encoding="utf-8").write(txt)
        json.dump({"segundos": round(dt, 1), "carga_s": round(carga, 1), "chars": len(txt),
                   "memoria_MB": resource.getrusage(resource.RUSAGE_SELF).ru_maxrss // 1024},
                  open(os.path.join(d, f"{clave}.json"), "w"))
        print(nombre, clave, f"segundos={dt:.0f} chars={len(txt)}", flush=True)


if __name__ == "__main__":
    main()
