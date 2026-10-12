import { describe, expect, it } from "vitest"
import { vocabularyFor } from "./index"
import { legal } from "./legal"

describe("vocabulary by vertical and language", () => {
  it("speaks the language asked for when the vertical has it", () => {
    expect(vocabularyFor("legal", "es").archive.name).toBe("Archivo")
    expect(vocabularyFor("legal", "en-US").archive.name).toBe("Archive")
  })

  it("falls back to the vertical's own language for the rest", () => {
    expect(vocabularyFor("legal", "zh").archive.name).toBe("Archivo")
  })

  it("has every term in every language of a vertical", () => {
    const shape = (value: unknown): unknown =>
      Array.isArray(value)
        ? "list"
        : value && typeof value === "object"
          ? Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, shape(inner)]))
          : typeof value
    const languages = Object.values(legal.languages)
    for (const language of languages) {
      expect(shape(language)).toEqual(shape(languages[0]))
    }
  })
})
