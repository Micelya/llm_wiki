import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it } from "vitest"
import { LongName, namePieces } from "./long-name"

describe("a long name", () => {
  it("breaks after its underscores and slashes, and keeps every character", () => {
    const name = "EXP-1_fs2_Escrito_inventado.pdf"
    expect(namePieces(name)).toEqual(["EXP-1_", "fs2_", "Escrito_", "inventado.pdf"])
    expect(namePieces("Clientes/Ejemplar\\Despido")).toEqual(["Clientes/", "Ejemplar\\", "Despido"])
    expect(namePieces(name).join("")).toBe(name)
  })

  it("is the name as it is when it has nowhere to break", () => {
    expect(namePieces("Recibo de sueldo.pdf")).toEqual(["Recibo de sueldo.pdf"])
    expect(namePieces("")).toEqual([])
    expect(namePieces("a_")).toEqual(["a_"])
  })

  it("is shown with a place to break between its pieces, and may break anywhere", () => {
    const html = renderToStaticMarkup(<LongName title="Escrito_inventado.pdf">Escrito_inventado.pdf</LongName>)
    expect(html).toBe('<span class="wrap-anywhere" title="Escrito_inventado.pdf">Escrito_<wbr/>inventado.pdf</span>')
  })
})
