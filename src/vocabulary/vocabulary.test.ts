import { readFileSync } from "node:fs"
import { join } from "node:path"
import { describe, expect, it } from "vitest"
import { relationName, resolveVocabulary, vocabularyFor, type Vertical } from "./index"
import { legal } from "./legal"

/** The relations the catalog of the engine lists, by id. */
function catalogRelations(): { id: string; directed: boolean }[] {
  const file = join(process.cwd(), "src-tauri", "resources", "catalog", "document-types.json")
  return JSON.parse(readFileSync(file, "utf8")).relations
}

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

describe("names of the relations between similar documents", () => {
  it("names every relation of the catalog in every language, and the direction of the directed ones", () => {
    for (const language of Object.values(legal.languages)) {
      for (const relation of catalogRelations()) {
        expect(language.relations[relation.id]?.name, relation.id).toBeTruthy()
        expect(Boolean(language.relations[relation.id]?.of), relation.id).toBe(relation.directed)
      }
    }
  })

  it("shows a relation nobody named yet by its id", () => {
    const words = vocabularyFor("legal", "es")
    expect(relationName("signed-copy", words)).toBe("Ejemplar firmado")
    expect(relationName("some-new-relation", words)).toBe("some-new-relation")
  })
})

describe("vocabulary by country", () => {
  // An invented vertical: one country changes one relation's name.
  const invented: Vertical = {
    default: "es",
    languages: { es: legal.languages.es!, en: legal.languages.en! },
    countries: {
      "es-XX": { relations: { ...legal.languages.es!.relations, "same-template": { name: "Mismo modelo, otros datos" } } },
      "en-XX": { suggestedGroups: ["Customer"] },
    },
  }

  it("takes the country's terms over its language's", () => {
    const words = resolveVocabulary(invented, "es-XX")
    expect(words.relations["same-template"].name).toBe("Mismo modelo, otros datos")
    expect(words.relations.version.name).toBe("Versión")
    expect(words.archive.name).toBe("Archivo")
  })

  it("keeps the language's terms for another country and for the bare language", () => {
    expect(resolveVocabulary(invented, "es-YY").relations["same-template"].name).toBe("Misma plantilla, otros datos")
    expect(resolveVocabulary(invented, "es").relations["same-template"].name).toBe("Misma plantilla, otros datos")
  })

  it("does not apply a country's terms to a language the vertical lacks", () => {
    const onlySpanish: Vertical = { default: "es", languages: { es: legal.languages.es! }, countries: invented.countries }
    expect(resolveVocabulary(onlySpanish, "en-XX").suggestedGroups).toEqual(["Cliente", "Caso"])
    expect(resolveVocabulary(invented, "en-XX").suggestedGroups).toEqual(["Customer"])
  })
})
