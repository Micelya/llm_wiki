import { renderToStaticMarkup } from "react-dom/server"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { KnowledgeArticle, PageObject } from "@/commands/docbase"
import { ArticleText } from "@/components/knowledge/article-text"
import { FloatingPreviewsSetting } from "@/components/settings/sections/interface-section"
import { forgetLinkPreviews, loadLinkPreview, previewOfArticle } from "@/lib/link-preview"
import { usePreviewCardsStore } from "@/stores/preview-cards-store"
import { vocabulary } from "@/vocabulary"
import { LinkCard } from "./page-link"

// Rendered once to text, a store shows the state its first render would
// read (`getInitialState`); here, the state each test sets.
vi.mock("zustand", async (importOriginal) => {
  const actual = await importOriginal<typeof import("zustand")>()
  const vanilla = await import("zustand/vanilla")
  const make = (createState: any) => {
    const api = vanilla.createStore(createState)
    api.getInitialState = api.getState
    const hook = (selector?: any) => actual.useStore(api, selector)
    return Object.assign(hook, api)
  }
  return { ...actual, create: (createState?: any) => (createState ? make(createState) : make) }
})

// What the base answers; every case below is made up.
const base = vi.hoisted(() => ({
  objects: {} as Record<string, unknown>,
  articles: {} as Record<string, unknown>,
}))

vi.mock("@/commands/docbase", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/commands/docbase")>()
  return {
    ...actual,
    objectOfPage: async (reference: string) => base.objects[reference] ?? null,
    getKnowledgeArticle: async (id: string) => base.articles[id] ?? null,
    getDocumentArticle: async () => null,
    getKnowledgeOverview: async () => null,
    onDocbaseChanged: async () => () => {},
  }
})

/** The visible text, without tags, with spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/&#x27;/g, "'")
    .replace(/\s+/g, " ")
    .trim()
}

const PAGE = `# Persona de Ejemplo

Firmó con [[organizacion-ficticia|Organización Ficticia S.A.]].[^1]

[^1]: Firma — «firma el presente» ([[doc-1|contrato.pdf]])
`

function object(objectId: string, kind: PageObject["kind"]): PageObject {
  return { objectId, kind, path: `wiki/ejemplo/${objectId}.md`, contentId: null, locationId: null }
}

function article(objectId: string, kind: KnowledgeArticle["kind"], title: string, body: string): KnowledgeArticle {
  return { objectId, kind, path: `wiki/ejemplo/${objectId}.md`, title, body, state: "projected", writtenAt: 0, notes: null }
}

const COMPANY = article(
  "ent-2",
  "entity",
  "Organización Ficticia S.A.",
  "---\ntype: entity\nentity_kind: organizacion\n---\n\n# Organización Ficticia S.A.\n\n*Organización* · CUIT 30-00000000-0\n\n## Lo que dicen\n\nAlquila un local inventado.\n",
)

beforeEach(() => {
  usePreviewCardsStore.setState({ enabled: true })
  forgetLinkPreviews()
  base.objects = { "organizacion-ficticia": object("ent-2", "entity") }
  base.articles = { "ent-2": COMPANY }
})

describe("the floating previews of an article (P-246)", () => {
  it("each note is a button that carries its label; each link to a page keeps its address", () => {
    const html = renderToStaticMarkup(<ArticleText body={PAGE} />)
    expect(html).toMatch(/<button[^>]*type="button"[^>]*data-note="1"[^>]*aria-label="Nota 1: ver la cita"[^>]*aria-expanded="false"[^>]*>\[1\]<\/button>/)
    expect(html).toMatch(/<a[^>]*href="#organizacion-ficticia"[^>]*>Organización Ficticia S.A.<\/a>/)
    // The note is not written at the foot: only in its card.
    expect(text(html)).not.toContain("firma el presente")
  })

  it("turned off, the note is still a button and the link a plain link", () => {
    usePreviewCardsStore.setState({ enabled: false })
    const html = renderToStaticMarkup(<ArticleText body={PAGE} />)
    expect(html).toMatch(/<button[^>]*data-note="1"/)
    expect(html).toMatch(/<a href="#organizacion-ficticia" class="[^"]*">Organización Ficticia S.A.<\/a>/)
  })

  it("the card of a link waits at a fixed size, then shows type, name, summary and a button", async () => {
    const waiting = renderToStaticMarkup(<LinkCard reference="organizacion-ficticia" onOpen={() => {}} />)
    expect(waiting).toContain('aria-busy="true"')
    expect(waiting).toContain("h-36")
    await loadLinkPreview("organizacion-ficticia")
    const shown = text(renderToStaticMarkup(<LinkCard reference="organizacion-ficticia" onOpen={() => {}} />))
    expect(shown).toBe("Organización Organización Ficticia S.A. Alquila un local inventado. Abrir")
  })

  it("a link to nothing the base knows says so", async () => {
    expect(await loadLinkPreview("no-existe")).toBeNull()
    const shown = text(renderToStaticMarkup(<LinkCard reference="no-existe" onOpen={() => {}} />))
    expect(shown).toBe("La base no tiene una ficha con este nombre.")
  })

  it("names the type in the words of the profession", () => {
    expect(previewOfArticle(object("mat-1", "matter"), null).typeLabel).toBe(vocabulary().knowledge.matter)
    expect(previewOfArticle(object("doc-1", "document"), null).typeLabel).toBe("Documento")
    const person = article("ent-1", "entity", "Persona de Ejemplo", "---\nentity_kind: persona\n---\n\n# Persona de Ejemplo\n\nSin más.\n")
    expect(previewOfArticle(object("ent-1", "entity"), person)).toMatchObject({ typeLabel: "Persona", title: "Persona de Ejemplo", summary: "Sin más." })
  })

  it("Settings → Interface can turn them off", () => {
    expect(renderToStaticMarkup(<FloatingPreviewsSetting />)).toMatch(/<input type="checkbox"[^>]*checked=""/)
    usePreviewCardsStore.setState({ enabled: false })
    const html = renderToStaticMarkup(<FloatingPreviewsSetting />)
    expect(html).not.toMatch(/checked=""/)
    expect(text(html)).toContain("Vistas previas flotantes")
  })
})
