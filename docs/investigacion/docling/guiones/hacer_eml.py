"""Build an invented .eml (fictitious people and data) with a small text PDF attached. Usage: python -I hacer_eml.py <out_dir>"""
import sys, os
from email.message import EmailMessage
from email.utils import format_datetime
from datetime import datetime, timezone, timedelta

def tiny_pdf(lines):
    # minimal one-page PDF with Helvetica text, WinAnsi encoding so accents survive
    content = "BT /F1 12 Tf 72 760 Td 16 TL " + " ".join(f"({l}) '" for l in lines) + " ET"
    data = content.encode("cp1252")
    objs = [b"<< /Type /Catalog /Pages 2 0 R >>", b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
            b"<< /Length " + str(len(data)).encode() + b" >>\nstream\n" + data + b"\nendstream",
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"]
    out, offs = bytearray(b"%PDF-1.4\n"), []
    for i, o in enumerate(objs, 1):
        offs.append(len(out)); out += f"{i} 0 obj\n".encode() + o + b"\nendobj\n"
    x = len(out)
    out += f"xref\n0 {len(objs)+1}\n0000000000 65535 f \n".encode() + b"".join(f"{o:010d} 00000 n \n".encode() for o in offs)
    out += f"trailer\n<< /Size {len(objs)+1} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n".encode()
    return bytes(out)

out = sys.argv[1]; os.makedirs(out, exist_ok=True)
m = EmailMessage()
m["From"] = "Estudio Ejemplo <mesa@estudio-ejemplo.test>"
m["To"] = "Cliente Ficticio <cliente@ejemplo.test>"
m["Cc"] = "Socia Inventada <socia@estudio-ejemplo.test>"
m["Subject"] = "Envío de la cédula de notificación – expediente de prueba"
m["Date"] = format_datetime(datetime(2026, 3, 14, 10, 30, tzinfo=timezone(timedelta(hours=-3))))
m.set_content("Estimado cliente:\n\nLe adjuntamos la cédula de notificación del expediente de prueba. "
              "La audiencia está fijada para el miércoles; recuerde traer su documentación.\n\n"
              "Saludos cordiales,\nMesa de entradas – Estudio Ejemplo\n")
m.add_alternative("<p>Estimado cliente:</p><p>Le adjuntamos la <b>cédula de notificación</b> del expediente de prueba. "
                  "La audiencia está fijada para el miércoles; recuerde traer su documentación.</p>"
                  "<p>Saludos cordiales,<br>Mesa de entradas – Estudio Ejemplo</p>", subtype="html")
pdf = tiny_pdf(["CÉDULA DE NOTIFICACIÓN (documento de prueba)", "Expediente: 0000/2026 (ficticio)",
                "Se notifica a la parte la audiencia del día miércoles.", "Firmado: Secretaría de ejemplo"])
open(f"{out}/adjunto.pdf", "wb").write(pdf)
m.add_attachment(pdf, maintype="application", subtype="pdf", filename="cedula-de-prueba.pdf")
open(f"{out}/correo-inventado.eml", "wb").write(bytes(m))
print("ok")
