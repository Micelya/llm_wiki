import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it, vi } from "vitest"
import { ProgramsSection } from "./programs-section"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(() => Promise.reject(new Error("sin app"))) }))

/** The visible text, without tags, with spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/&#x27;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/\s+/g, " ")
    .trim()
}

describe("Ajustes → Programas conectados", () => {
  it("says what it is for, the server, how to add a program and how to connect an assistant, in Spanish", () => {
    const shown = text(renderToStaticMarkup(<ProgramsSection />))
    expect(shown).toContain("Programas conectados")
    expect(shown).toContain("Por defecto solo lee")
    expect(shown).toContain("espera en Atención a que una persona lo confirme")
    expect(shown).toContain("El servidor de Micelya")
    expect(shown).toContain("http://127.0.0.1:19827")
    expect(shown).toContain("El recortador web (Micelya Clipper) guarda páginas siempre, sin token")
    expect(shown).toContain("Agregar un programa")
    expect(shown).toContain("Solo leer")
    expect(shown).toContain("Leer y pedir cambios")
    expect(shown).toContain("Cómo conectar un asistente por MCP")
    expect(shown).toContain('claude mcp add --transport http micelya http://127.0.0.1:19827/mcp --header "Authorization: Bearer EL-TOKEN"')
    expect(shown).toContain("[mcp_servers.micelya]")
    expect(shown).toContain("/api/v1/operaciones/<nombre>")
    expect(shown).toContain("Claude Desktop")
  })
})
