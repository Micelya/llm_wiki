import { readdirSync, readFileSync } from "node:fs"
import { join } from "node:path"
import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it, vi } from "vitest"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn(() => Promise.resolve()) }))

import i18n from "@/i18n"
import { CHANGELOG } from "@/lib/changelog"
import { LICENSE_TEXT } from "@/lib/license"
import { AboutSection } from "@/components/settings/sections/about-section"
import { LEGAL_NOTICE, PRODUCT_NAME, SITE_URL, SOURCE_URL } from "./app-identity"

/** How the original project is named, by name or by its repository. */
const ORIGINAL = /LLM[ -]Wiki|nashsu/

/** The only files that may name it: the legal notice and this test. */
const ALLOWED = ["lib/app-identity.ts", "lib/app-identity.test.tsx"]

/** Every source and language file of the interface, relative to `src`. */
function interfaceFiles(): string[] {
  const root = join(process.cwd(), "src")
  return (readdirSync(root, { recursive: true }) as string[])
    .map((path) => path.replace(/\\/g, "/"))
    .filter((path) => /\.(ts|tsx|json)$/.test(path))
}

/** The visible text, without tags, with spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/&#x27;/g, "'")
    .replace(/&amp;/g, "&")
    .replace(/\s+/g, " ")
    .trim()
}

describe("the app's own identity (D-485)", () => {
  it("names the original project only in the legal notice", () => {
    const root = join(process.cwd(), "src")
    const naming = interfaceFiles().filter(
      (path) => !ALLOWED.includes(path) && ORIGINAL.test(readFileSync(join(root, path), "utf8")),
    )
    expect(naming).toEqual([])
  })

  it("names this app in the translated texts", () => {
    // The screens are in Spanish (P-058); the other languages name it too.
    expect(i18n.t("settings.sections.general.autostartHint")).toBe(
      "Micelya Desktop se abre sola cuando entrás en esta computadora.",
    )
    expect(i18n.t("settings.sections.general.autostartHint", { lng: "en" })).toBe(
      "Starts Micelya Desktop automatically after you sign in to this computer.",
    )
    expect(i18n.t("settings.sections.general.closeExitHint")).toContain(PRODUCT_NAME)
  })

  it("shows only this app's releases in the changelog", () => {
    expect(CHANGELOG.map((entry) => entry.version)).toEqual(["0.1.0"])
  })

  it("keeps in the legal notice what the licence asks for", () => {
    expect(LEGAL_NOTICE).toContain("LLM Wiki (© 2024-2026 Yong Su)")
    expect(LEGAL_NOTICE).toContain("GNU GPL versión 3")
    expect(LEGAL_NOTICE).toContain("modificada por Micelya")
    expect(LEGAL_NOTICE).toContain("sin ninguna garantía")
    expect(LEGAL_NOTICE).toContain("github.com/Micelya/llm_wiki")
  })

  it("ships the full text of the licence with the app", () => {
    expect(LICENSE_TEXT).toContain("GNU GENERAL PUBLIC LICENSE")
    expect(LICENSE_TEXT).toContain("Version 3, 29 June 2007")
  })

  it("shows in Acerca de the app, its version, its links and the legal notice", () => {
    const html = renderToStaticMarkup(<AboutSection />)
    const shown = text(html)
    expect(shown).toContain(`Micelya Desktop Versión v${__APP_VERSION__}`)
    expect(html).toContain(`href="${SITE_URL}"`)
    expect(html).toContain(`href="${SOURCE_URL}"`)
    expect(SOURCE_URL).toBe("https://github.com/Micelya/llm_wiki")
    expect(shown).toContain("Sitio: micelya.com")
    expect(shown).toContain("Código fuente: github.com/Micelya/llm_wiki")
    expect(shown).toContain(`Avisos legales ${LEGAL_NOTICE}`)
    expect(shown).toContain("Ver el texto completo de la licencia")
    expect(shown).not.toContain("GNU GENERAL PUBLIC LICENSE")
  })

  it("shows the licence when asked", () => {
    const shown = text(renderToStaticMarkup(<AboutSection licenseOpen />))
    expect(shown).toContain("Ocultar la licencia")
    expect(shown).toContain("GNU GENERAL PUBLIC LICENSE")
  })
})
