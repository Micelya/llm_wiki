import { renderToStaticMarkup } from "react-dom/server"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { DocumentDetail, DocumentEntities, EntityDetail, KnowledgeArticle, MatterDetail } from "@/commands/docbase"
import { DocumentDetailView } from "@/components/docbase/document-detail"
import { useAiServicesStore } from "@/stores/ai-services-store"
import { NO_SERVICES } from "@/lib/ai-services"
import type { AiServices } from "@/commands/docbase"
import { EntityDetailView } from "./entity-detail"
import { MatterDetailView } from "./matters-view"

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

// What the base answers, by the key of each query (or the part before
// its first colon); every case below is made up.
const base = vi.hoisted(() => ({ answers: {} as Record<string, unknown> }))

vi.mock("@/components/docbase/use-docbase-query", () => ({
  useDocbaseQuery: (key: string) => ({
    data: base.answers[key] ?? base.answers[key.split(":")[0]] ?? null,
    error: null,
    loading: false,
    reload: () => {},
  }),
}))

/** The visible text, without tags, with spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/&#x27;/g, "'")
    .replace(/\s+/g, " ")
    .trim()
}

/** The text of the tab that is shown. */
function selectedTab(html: string): string | null {
  const found = /<button[^>]*role="tab"[^>]*aria-selected="true"[^>]*>(.*?)<\/button>/.exec(html)
  return found ? text(found[1]) : null
}

function article(objectId: string, body: string): KnowledgeArticle {
  return {
    objectId,
    kind: objectId.startsWith("mat-") ? "matter" : "document",
    path: `wiki/ejemplo/${objectId}.md`,
    title: "Ejemplo",
    body,
    state: "projected",
    writtenAt: Date.UTC(2026, 9, 20, 12),
    notes: null,
  }
}

const PAGE_WORDS = { docLocations: "Ubicaciones", matterFacts: "Lo que dicen los documentos del caso" }

function documentDetail(overrides: Partial<DocumentDetail> = {}): DocumentDetail {
  return {
    locationId: 10,
    originId: "o-1",
    originName: "Carpeta de prueba",
    originReachable: true,
    relPath: "Contratos/Locación de ejemplo.pdf",
    folder: "Contratos",
    name: "Locación de ejemplo.pdf",
    ext: "pdf",
    size: 2048,
    mtimeMs: Date.UTC(2026, 9, 1, 12),
    state: "processed",
    contentId: 5,
    copies: 1,
    labels: [],
    docType: null,
    docStage: null,
    docTypeProposed: false,
    docTypeName: null,
    path: "C:/prueba/Contratos/Locación de ejemplo.pdf",
    fileHash: null,
    error: null,
    wordCount: 6,
    processedAt: Date.UTC(2026, 9, 1, 13),
    firstSeenAt: Date.UTC(2026, 9, 1, 12),
    text: "El locador inventado alquila un local inventado.",
    textEngine: "extraction",
    textAt: Date.UTC(2026, 9, 1, 13),
    locations: [],
    generated: null,
    ...overrides,
  }
}

const DOCUMENT_ARTICLE = article(
  "doc-1",
  "---\ntype: source\n---\n\n# Locación de ejemplo\n\nTema: el alquiler de un local de prueba.\n\n## Lo esencial\n\n- El plazo es de dos años.\n\n## Ubicaciones\n\n- Carpeta de prueba › Contratos\n",
)

const DOCUMENT_ENTITIES: DocumentEntities = {
  contentId: 5,
  state: "done",
  older: false,
  reread: false,
  error: null,
  entities: [{ entityId: "ent-1", kind: "organizacion", name: "Inmobiliaria Ficticia SA", written: ["Inmobiliaria Ficticia SA"], roles: ["parte"], pagePath: null }],
  references: [],
  matters: [{ contentId: 5, matterId: "mat-1", name: "Caso de prueba", proposed: false, pagePath: null }],
  discarded: 0,
}

function matterDetail(overrides: Partial<MatterDetail> = {}): MatterDetail {
  return {
    id: "mat-1",
    name: "Reclamo de Ficticia SA a Inventada SRL",
    state: "active",
    status: "open",
    origin: "proposed",
    confirmation: "proposed",
    pagePath: "wiki/entities/reclamo-de-ficticia.md",
    documents: [
      { document: { contentId: 5, locationId: 10, name: "Carta documento de prueba.pdf" }, state: "firm", reasons: [], proposalReason: null },
      { document: { contentId: 6, locationId: 11, name: "Recibo inventado.pdf" }, state: "proposed", reasons: [], proposalReason: "habla del mismo local" },
    ],
    parties: [{ entityId: "ent-1", kind: "organizacion", name: "Ficticia SA", roles: ["parte"], documents: 2 }],
    decisions: [],
    ...overrides,
  }
}

const MATTER_ARTICLE = article(
  "mat-1",
  "---\ntype: entity\n---\n\n# Reclamo de Ficticia SA a Inventada SRL\n\n*Caso*\n\n## Partes\n\n- [[ficticia-sa|Ficticia SA]] · Organización — parte (2 documentos)\n\n## Lo que dicen los documentos del caso\n\n- Ficticia SA reclama una deuda inventada.\n",
)

beforeEach(() => {
  for (const key of Object.keys(base.answers)) delete base.answers[key]
  base.answers["page-words"] = PAGE_WORDS
  base.answers.research = []
})

describe("the screen of a document", () => {
  beforeEach(() => {
    base.answers.document = documentDetail()
    base.answers["document-article"] = DOCUMENT_ARTICLE
    base.answers["document-entities"] = DOCUMENT_ENTITIES
  })

  it("has the two tabs of a person or organization, and opens on the article", () => {
    const html = renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} />)
    expect(html.match(/role="tab"/g)).toHaveLength(2)
    expect(selectedTab(html)).toBe("Artículo")
    expect(text(html)).toContain("Datos y correcciones")
  })

  it("shows in «Artículo» only the article, and the notes at its foot", () => {
    const shown = text(renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} />))
    expect(shown).toContain("Tema: el alquiler de un local de prueba.")
    expect(shown).toContain("El plazo es de dos años.")
    expect(shown).toContain("Notas")
    // Where it is and whom it names are in the other tab.
    expect(shown).not.toContain("Carpeta de prueba › Contratos")
    expect(shown).not.toContain("El locador inventado alquila un local inventado.")
    expect(shown).not.toContain("Inmobiliaria Ficticia SA")
    expect(shown).not.toContain("Etiquetas")
  })

  it("shows in «Datos y correcciones» everything else, and the same notes", () => {
    const html = renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} initialTab="data" />)
    const shown = text(html)
    expect(selectedTab(html)).toBe("Datos y correcciones")
    expect(shown).toContain("Inmobiliaria Ficticia SA")
    expect(shown).toContain("Caso de prueba")
    expect(shown).toContain("Agregar a un caso")
    expect(shown).toContain("Ubicaciones 1")
    expect(shown).toContain("Contratos/Locación de ejemplo.pdf")
    expect(shown).toContain("Texto")
    expect(shown).toContain("El locador inventado alquila un local inventado.")
    expect(shown).toContain("Etiquetas")
    expect(shown).toContain("Notas")
    expect(shown).not.toContain("El plazo es de dos años.")
    // The whole text goes last: after the places.
    expect(shown.indexOf("El locador inventado")).toBeGreaterThan(shown.indexOf("Ubicaciones 1"))
  })

  it("opens on «Datos y correcciones» when it has no article, and the article tab says so", () => {
    base.answers["document-article"] = null
    const html = renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} />)
    expect(selectedTab(html)).toBe("Datos y correcciones")
    const article = text(renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} initialTab="article" />))
    expect(article).toContain("Su artículo todavía no está escrito: la app lo escribe en su próxima pasada.")
    expect(article).toContain("Ver «Datos y correcciones»")
  })

  it("is titled by its article, and the name of its file stays in its data (P-182)", () => {
    base.answers["document-article"] = { ...DOCUMENT_ARTICLE, title: "Contrato de locación de Inmobiliaria Ficticia SA" }
    const article = renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} />)
    expect(article).toContain(">Contrato de locación de Inmobiliaria Ficticia SA</h1>")
    expect(text(article)).not.toContain("Locación de ejemplo.pdf")
    const data = text(renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} initialTab="data" />))
    expect(data).toContain("Locación de ejemplo.pdf")
  })

  it("offers to ask the chat about the document, once it has its text (P-243)", () => {
    const html = renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} />)
    expect(html).toMatch(/<button[^>]*title="Abre el chat con este documento adjunto"[^>]*>.*?Preguntar al chat sobre este documento<\/button>/)
    base.answers.document = documentDetail({ contentId: null, state: "unread", text: null })
    const unread = renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} />)
    expect(unread).toMatch(/<button[^>]*disabled=""[^>]*>.*?Preguntar al chat sobre este documento<\/button>/)
  })

  it("says a file not read yet has no article", () => {
    base.answers.document = documentDetail({ contentId: null, state: "unread", text: null })
    const html = renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} initialTab="article" />)
    expect(text(html)).toContain("Este archivo todavía no se leyó: no tiene artículo.")
  })
})


/** The registry with «Reconocer escaneos» on Codex with this model (made up). */
const recognizing = (model: string): AiServices => ({
  ...NO_SERVICES,
  recognize: { engine: { engine: "codex-cli", connection: "codex-cli", model, reasoning: "" }, alternative: null },
})

describe("the recognition of a scan, on the screen of its document (P-242)", () => {
  const scan = () =>
    documentDetail({ state: "needs-recognition", text: null, textEngine: null, textAt: null, wordCount: null, name: "Escaneo de prueba.pdf" })

  beforeEach(() => {
    base.answers["document-article"] = null
    base.answers["document-entities"] = { ...DOCUMENT_ENTITIES, entities: [], matters: [] }
    useAiServicesStore.setState({ services: recognizing("modelo-de-prueba") })
  })

  it("offers «Reconocer el texto» first in «Datos y correcciones», with what it costs", () => {
    base.answers.document = scan()
    const html = renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} initialTab="data" />)
    const shown = text(html)
    expect(html).toContain('data-recognition="waiting"')
    expect(shown).toContain("El texto de este documento está en imagen")
    expect(shown).toContain("Cada página es una pregunta al modelo")
    expect(/<button[^>]*>(?:(?!<\/button>).)*Reconocer el texto<\/button>/.test(html)).toBe(true)
    expect(/<button[^>]*disabled=""[^>]*>(?:(?!<\/button>).)*Reconocer el texto/.test(html)).toBe(false)
    expect(shown.indexOf("El texto de este documento está en imagen")).toBeLessThan(shown.indexOf("Ubicaciones"))
  })

  it("says why the last try failed, in words, and offers to try again", () => {
    base.answers.document = { ...scan(), error: "Text recognition failed on page 3: You've hit your usage limit" }
    const shown = text(renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} initialTab="data" />))
    expect(shown).toContain("El último intento falló: El proveedor del modelo llegó a su límite de uso (página 3).")
    expect(shown).toContain("Reintentar")
  })

  it("says how to choose an engine when the model chosen cannot read images, and does not offer the button", () => {
    useAiServicesStore.setState({ services: NO_SERVICES })
    base.answers.document = scan()
    const html = renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} initialTab="data" />)
    expect(text(html)).toContain("Falta elegir el motor que reconoce escaneos")
    expect(/<button[^>]*disabled=""[^>]*>(?:(?!<\/button>).)*Reconocer el texto/.test(html)).toBe(true)
  })

  it("says which engine recognized the text and when, and the pages it could not read", () => {
    base.answers.document = documentDetail({
      textEngine: "codex-cli:modelo-de-prueba",
      textAt: new Date(2026, 9, 21, 12).getTime(),
      text: '## Page 1\n\n<!-- texto-reconocido motor="codex-cli:modelo-de-prueba" -->\nUn recibo [ilegible] inventado.\n\n## Page 2\n\n<!-- texto-reconocido motor="codex-cli:modelo-de-prueba" -->\nSegunda hoja.\n',
    })
    const html = renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} initialTab="data" />)
    const shown = text(html)
    expect(shown).toContain("Texto reconocido por Codex (modelo-de-prueba) el 21/10/2026.")
    expect(shown).toContain("El motor no pudo leer partes de la página 1")
    expect(shown).toContain("Reconocimiento: Codex (modelo-de-prueba), el 21/10/2026")
    // The marks of each page are not shown; the same engine does not read it again.
    expect(shown).not.toContain("texto-reconocido")
    expect(shown).not.toContain("Reconocer de nuevo")
  })

  it("offers to read it again only when another engine is chosen", () => {
    base.answers.document = documentDetail({ textEngine: "codex-cli:otro-modelo", text: "Texto de prueba." })
    const shown = text(renderToStaticMarkup(<DocumentDetailView locationId={10} onBack={() => {}} initialTab="data" />))
    expect(shown).toContain("Texto reconocido por Codex (otro-modelo)")
    expect(shown).toContain("Reconocer de nuevo con Codex (modelo-de-prueba)")
  })
})

describe("the screen of a matter", () => {
  beforeEach(() => {
    base.answers.matter = matterDetail()
    base.answers.article = MATTER_ARTICLE
  })

  it("has the same two tabs, and opens on its article", () => {
    const html = renderToStaticMarkup(<MatterDetailView id="mat-1" onBack={() => {}} />)
    expect(html.match(/role="tab"/g)).toHaveLength(2)
    expect(selectedTab(html)).toBe("Artículo")
  })

  it("shows in «Artículo» the article the base writes, the research on internet and the notes", () => {
    const html = renderToStaticMarkup(<MatterDetailView id="mat-1" onBack={() => {}} />)
    const shown = text(html)
    expect(shown).toContain("Partes")
    expect(shown).toContain("Ficticia SA reclama una deuda inventada.")
    expect(shown).toContain("Investigación en internet")
    expect(shown).toContain("Notas")
    // A link of the article leads to the screen of what it names.
    expect(html).toContain('href="#ficticia-sa"')
    // Deciding is in the other tab.
    expect(shown).not.toContain("Agregar documentos")
    expect(shown).not.toContain("Carta documento de prueba.pdf")
    expect(shown).not.toContain("Descartar")
  })

  it("shows in «Datos y correcciones» what a person decides of it, and the same notes", () => {
    const html = renderToStaticMarkup(<MatterDetailView id="mat-1" onBack={() => {}} initialTab="data" />)
    const shown = text(html)
    expect(selectedTab(html)).toBe("Datos y correcciones 2")
    expect(shown).toContain("Confirmar")
    expect(shown).toContain("Descartar")
    expect(shown).toContain("Documentos 1")
    expect(shown).toContain("Carta documento de prueba.pdf")
    expect(shown).toContain("Agregar documentos")
    expect(shown).toContain("Posiblemente de este caso")
    expect(shown).toContain("Recibo inventado.pdf")
    expect(shown).toContain("Partes y personas")
    expect(shown).toContain("Notas")
    expect(shown).not.toContain("Ficticia SA reclama una deuda inventada.")
    expect(shown).not.toContain("Investigación en internet")
    // Closing or archiving it (D-475), by the contract of operations.
    expect(shown).toContain("Cerrar")
    expect(shown).toContain("Archivar")
    expect(shown).not.toContain("Reabrir")
  })

  it("says a closed matter is out of the list and Atención, and offers to reopen it", () => {
    base.answers.matter = matterDetail({ status: "closed", confirmation: "confirmed" })
    const shown = text(renderToStaticMarkup(<MatterDetailView id="mat-1" onBack={() => {}} initialTab="data" />))
    expect(shown).toContain("Reabrir")
    expect(shown).not.toContain("Archivar")
    expect(shown).toContain("Este caso está cerrado: no aparece en la lista ni en Atención, sigue en la búsqueda y no recibe propuestas nuevas.")
    expect(shown).toContain("Cerrado")
    base.answers.matter = matterDetail()
  })

  it("says a matter without article has none, and opens on «Datos y correcciones»", () => {
    base.answers.matter = matterDetail({
      origin: "person",
      confirmation: "confirmed",
      pagePath: null,
      documents: [matterDetail().documents[0]],
    })
    base.answers.article = null
    const html = renderToStaticMarkup(<MatterDetailView id="mat-1" onBack={() => {}} />)
    expect(selectedTab(html)).toBe("Datos y correcciones")
    const article = text(renderToStaticMarkup(<MatterDetailView id="mat-1" onBack={() => {}} initialTab="article" />))
    expect(article).toContain("Este caso todavía no tiene artículo")
    expect(article).toContain("Ver «Datos y correcciones»")
    expect(article).toContain("Notas")
  })
})

describe("the screen of a person or organization, on the same tabs", () => {
  const entity: EntityDetail = {
    id: "ent-1",
    kind: "organizacion",
    name: "Ficticia SA",
    state: "active",
    mergedInto: null,
    aliases: [{ written: "Ficticia S.A.", mentions: 2 }],
    identifiers: [],
    mentions: [],
    claims: [],
    doubts: [],
    pagePath: "wiki/entities/ficticia-sa.md",
    matters: [],
    decisions: [],
    incoming: [],
  }

  beforeEach(() => {
    base.answers.entity = entity
    base.answers.article = article("ent-1", "---\ntype: entity\n---\n\n# Ficticia SA\n\nUna sociedad inventada para las pruebas.\n")
  })

  it("keeps its article, the research and the notes in «Artículo»", () => {
    const html = renderToStaticMarkup(<EntityDetailView id="ent-1" onBack={() => {}} />)
    const shown = text(html)
    expect(selectedTab(html)).toBe("Artículo")
    expect(shown).toContain("Una sociedad inventada para las pruebas.")
    expect(shown).toContain("Investigación en internet")
    expect(shown).toContain("Notas")
    expect(shown).not.toContain("Se escribe")
  })

  it("shows the quotes of its article as numbered notes, not as a list at its foot (D-441)", () => {
    base.answers.article = article(
      "ent-1",
      "---\ntype: entity\n---\n\n# Ficticia SA\n\n## Lo que dicen los documentos\n\nTiene domicilio en Calle Inventada 100.[^1]\n\n[^1]: Domicilio: Calle Inventada 100 — «con domicilio en Calle Inventada 100» ([[contrato-de-prueba|Contrato de prueba.pdf]])\n",
    )
    const html = renderToStaticMarkup(<EntityDetailView id="ent-1" onBack={() => {}} />)
    expect(html).toMatch(/<button[^>]*data-note="1"[^>]*>\[1\]<\/button>/)
    const shown = text(html)
    expect(shown).toContain("Tiene domicilio en Calle Inventada 100.")
    // The quote shows when the number is touched, not before.
    expect(shown).not.toContain("con domicilio en Calle Inventada 100")
    expect(shown).not.toContain("[^1]")
  })

  it("lays its article out as a page of an encyclopedia (D-493): where you are, its documents in a table, the doubtful marked, an index", () => {
    base.answers["page-words"] = {
      ...PAGE_WORDS,
      says: "Lo que dicen los documentos",
      documents: "Documentos",
      conflicts: "Datos en los que los documentos difieren",
      certainDoubtful: "sin confirmar",
      proposed: "propuesto",
    }
    base.answers.entity = { ...entity, identifiers: [{ kind: "cuit", value: "30-00000000-0", documents: 2 }] }
    base.answers.article = article(
      "ent-1",
      [
        "---\ntype: entity\n---\n\n# Ficticia SA\n\n*Organización* · CUIT 30-00000000-0 (2 documentos)\n",
        "## Lo que dicen los documentos\n\nTiene un local inventado *(sin confirmar)*.\n",
        "**Datos en los que los documentos difieren**\n\n- Domicilio: «Calle 1» · «Calle 2»\n",
        "## Documentos (1)\n\n- [[contrato-de-prueba|Contrato de prueba]] — parte · contrato\n",
      ].join("\n"),
    )
    const html = renderToStaticMarkup(<EntityDetailView id="ent-1" onBack={() => {}} />)
    const shown = text(html)
    expect(html).toMatch(/<nav aria-label="Dónde estás"[^>]*>.*Personas y organizaciones.*Ficticia SA/)
    expect(shown).toContain("Organización Ficticia SA")
    expect(html).toContain('data-mark="doubt"')
    expect(html).toContain('data-tone="warn"')
    expect(shown).toContain("«Calle 1» o «Calle 2»")
    expect(html).toMatch(/<th[^>]*>Su papel<\/th>/)
    expect(html).toContain('href="#contrato-de-prueba"')
    expect(html).toContain('data-infobox=""')
    expect(shown).toContain("30-00000000-0")
    expect(html).toContain('aria-label="En esta página"')
    // The text of the page is not shown as written: its marks are the screen's.
    expect(shown).not.toContain("*(sin confirmar)*")
    expect(shown).not.toContain("**Datos")
  })

  it("keeps its data in «Datos y correcciones», now with the notes at the foot too", () => {
    const html = renderToStaticMarkup(<EntityDetailView id="ent-1" onBack={() => {}} initialTab="data" />)
    const shown = text(html)
    expect(selectedTab(html)).toBe("Datos y correcciones")
    expect(shown).toContain("Lo que dicen los documentos")
    expect(shown).toContain("Se escribe")
    expect(shown).toContain("Ficticia S.A.")
    expect(shown).toContain("Unir con otra")
    expect(shown).toContain("Notas")
    expect(shown).not.toContain("Una sociedad inventada para las pruebas.")
  })
})
