"""Lista recursivamente una carpeta compartida de Drive y baja sus archivos.

Uso: python bajar.py <id_carpeta_raiz> <carpeta_destino>
Escribe <destino>/lista.tsv (id, ruta relativa, bytes). No imprime nombres.
"""
import html
import os
import re
import subprocess
import sys
import urllib.request

ENTRY = re.compile(
    r'<div class="flip-entry" id="entry-([A-Za-z0-9_-]+)".*?'
    r'href="https://drive\.google\.com/(file/d|drive/folders)/.*?'
    r'<div class="flip-entry-title">(.*?)</div>',
    re.S,
)


def listar(folder_id):
    url = f"https://drive.google.com/embeddedfolderview?id={folder_id}"
    with urllib.request.urlopen(url, timeout=60) as r:
        h = r.read().decode("utf-8")
    for fid, kind, title in ENTRY.findall(h):
        yield fid, kind == "drive/folders", html.unescape(title).replace("/", "_")


def recorrer(folder_id, ruta=""):
    for fid, es_carpeta, nombre in listar(folder_id):
        rel = os.path.join(ruta, nombre)
        if es_carpeta:
            yield fid, rel, True
            yield from recorrer(fid, rel)
        else:
            yield fid, rel, False


def main():
    raiz, destino = sys.argv[1], sys.argv[2]
    os.makedirs(destino, exist_ok=True)
    carpetas = archivos = errores = 0
    total = 0
    with open(os.path.join(destino, "lista.tsv"), "w", encoding="utf-8") as out:
        for fid, rel, es_carpeta in recorrer(raiz):
            path = os.path.join(destino, "datos", rel)
            if es_carpeta:
                os.makedirs(path, exist_ok=True)
                carpetas += 1
                continue
            os.makedirs(os.path.dirname(path), exist_ok=True)
            if not os.path.exists(path):
                url = ("https://drive.usercontent.google.com/download"
                       f"?id={fid}&export=download&confirm=t")
                r = subprocess.run(["curl", "-sS", "-L", "-o", path, "-w",
                                    "%{http_code}", url],
                                   capture_output=True, text=True)
                if r.stdout != "200":
                    errores += 1
                    continue
            n = os.path.getsize(path)
            total += n
            archivos += 1
            out.write(f"{fid}\t{rel}\t{n}\n")
    print(f"carpetas={carpetas} archivos={archivos} bytes={total} errores={errores}")


if __name__ == "__main__":
    main()
