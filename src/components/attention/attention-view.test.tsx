import { renderToStaticMarkup } from "react-dom/server"
import { beforeEach, describe, expect, it, vi } from "vitest"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }))

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

// What the base answers, by the exact key of each query; every case below is made up.
const base = vi.hoisted(() => ({ answers: {} as Record<string, unknown> }))

vi.mock("@/components/docbase/use-docbase-query", () => ({
  useDocbaseQuery: (key: string) => ({ data: base.answers[key] ?? null, error: null, loading: false, reload: () => {} }),
}))

import type { Home, ModelError, OriginSummary, StageItems } from "@/commands/docbase"
import type { AttentionNotice } from "@/lib/attention/types"
import { progress, stage } from "@/lib/progress/progress.fixtures"
import { useAttentionStore } from "@/stores/attention-store"
import { useAttentionViewStore } from "@/stores/attention-view-store"
import { useProgressStore } from "@/stores/progress-store"
import { useAiServicesStore } from "@/stores/ai-services-store"
import { NO_SERVICES } from "@/lib/ai-services"
import type { AiServices } from "@/commands/docbase"

/** The registry with «Reconocer escaneos» on Codex with this model (made up). */
const recognizing = (model: string): AiServices => ({
  ...NO_SERVICES,
  recognize: { engine: { engine: "codex-cli", connection: "codex-cli", model, reasoning: "" }, alternative: null },
})
import { vocabularyFor } from "@/vocabulary"
import { HomeView } from "@/components/docbase/home-view"
import { AutoQuestionsLine, autoQuestionsText, itemTarget, StageItemsList, StageLine } from "@/components/progress/ongoing-tab"
import { decideAutoQuestions, type AutoQuestions } from "@/commands/docbase"
import { invoke } from "@tauri-apps/api/core"
import { ProgressLine } from "@/components/progress/progress-line"
import { AttentionView, shownTab } from "./attention-view"

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

function tabs(html: string): { title: string; selected: boolean }[] {
  return [...html.matchAll(/<button role="tab" aria-selected="(true|false)"[^>]*>(.*?)<\/button>/g)].map((m) => ({
    title: text(m[2]),
    selected: m[1] === "true",
  }))
}

function stages(html: string): [string, string][] {
  return [...html.matchAll(/data-stage="([^"]+)" data-state="([^"]+)"/g)].map((m) => [m[1], m[2]])
}

function notice(id: string, kind: AttentionNotice["kind"], title: string): AttentionNotice {
  return {
    id: `prueba:${id}`,
    source: "prueba",
    kind,
    topic: id,
    priority: "normal",
    count: 1,
    title,
    explanation: "Una explicación inventada.",
    todo: "Algo para hacer.",
    resolvedWhen: "Desaparece sola.",
    actions: [],
  }
}

const limit: ModelError = {
  stage: "reading",
  kind: "limit",
  message: "You've hit your usage limit",
  at: new Date(2026, 9, 21, 14, 5).getTime(),
  retryAt: new Date(2026, 9, 21, 14, 15).getTime(),
}

/** The new folder of 68 documents half read: what the owner wants to follow. */
const halfway = progress(
  {
    text: { done: 68, total: 68 },
    recognition: {
      state: "running",
      done: 1,
      total: 4,
      pending: 3,
      extra: { pages: 9, uncounted: 0, requested: 0 },
      current: {
        status: "pending",
        locationId: 8,
        contentId: 4,
        originId: "o1",
        pagePath: null,
        name: "escaneo de prueba.pdf",
        place: "Carpeta de prueba › nueva",
        detail: null,
        page: 2,
        pages: 5,
      },
    },
    types: { done: 68, total: 68 },
    reading: {
      state: "running",
      done: 12,
      total: 68,
      pending: 56,
      current: {
        status: "pending",
        locationId: 7,
        contentId: 3,
        originId: "o1",
        pagePath: null,
        name: "nota de prueba.pdf",
        place: "Carpeta de prueba › nueva",
        detail: null,
      },
    },
    resolve: { state: "waiting", done: 0, total: 12, pending: 12 },
    matters: { state: "error", done: 0, total: 12, pending: 12, error: limit },
    pages: { state: "paused", done: 4, total: 6, pending: 2 },
  },
  { inProcess: 59, paused: 2 },
)

beforeEach(() => {
  base.answers = { "background-reading": true }
  useAttentionStore.setState({ bySource: {} })
  useAttentionViewStore.setState({ tab: null, scope: {} })
  useProgressStore.setState({ progress: null, error: null })
})

describe("the attention view in tabs", () => {
  it("has three tabs, each with how many things it has, and opens on the first with something", () => {
    useAttentionStore.setState({
      bySource: {
        prueba: [
          notice("a", "problem", "Un problema inventado"),
          notice("b", "problem", "Otro problema inventado"),
          notice("c", "decision", "Una decisión inventada"),
        ],
      },
    })
    useProgressStore.setState({ progress: halfway })
    const html = renderToStaticMarkup(<AttentionView />)
    expect(tabs(html)).toEqual([
      { title: "Problemas 2", selected: true },
      { title: "Decisiones 1", selected: false },
      // The stages not done: reading, resolve, matters, pages and the scans.
      { title: "En curso 5", selected: false },
    ])
    expect(text(html)).toContain("Un problema inventado")
    expect(text(html)).not.toContain("Una decisión inventada")
  })

  it("counts in «En curso» over the origin or folder the tab is narrowed to", () => {
    useProgressStore.setState({ progress: halfway })
    const ongoing = () => tabs(renderToStaticMarkup(<AttentionView />)).find((tab) => tab.title.startsWith("En curso"))?.title
    expect(ongoing()).toBe("En curso 5")
    useAttentionViewStore.setState({ scope: { originId: "o1", folder: "nueva" } })
    base.answers["progress:o1/nueva"] = progress({ reading: { state: "running", done: 3, total: 68, pending: 65 } })
    expect(ongoing()).toBe("En curso 1")
    // Everything done in that folder: no figure.
    base.answers["progress:o1/nueva"] = progress()
    expect(ongoing()).toBe("En curso")
  })

  it("keeps the tab the person chose", () => {
    useAttentionViewStore.setState({ tab: "decision" })
    const html = renderToStaticMarkup(<AttentionView />)
    expect(tabs(html).find((tab) => tab.selected)?.title).toBe("Decisiones")
    expect(text(html)).toContain("Nada espera una decisión.")
    expect(shownTab(null, { problem: 0, decision: 0, ongoing: 2 })).toBe("ongoing")
    expect(shownTab(null, { problem: 0, decision: 0, ongoing: 0 })).toBe("problem")
  })
})

describe("«En curso»", () => {
  beforeEach(() => {
    useAttentionViewStore.setState({ tab: "ongoing" })
  })

  it("shows every stage in order, the finished ones too, each with how it stands", () => {
    useProgressStore.setState({ progress: halfway })
    const html = renderToStaticMarkup(<AttentionView />)
    expect(stages(html)).toEqual([
      ["origins", "done"],
      ["text", "done"],
      ["recognition", "running"],
      ["similar", "done"],
      ["types", "done"],
      ["templates", "done"],
      ["reading", "running"],
      ["resolve", "waiting"],
      ["matters", "error"],
      ["pages", "paused"],
    ])
    const shown = text(html)
    expect(shown).toContain("Plantillas: nombres de los campos Hecha")
    // A finished stage stays, with its figures.
    expect(shown).toContain("Tipo y datos de cada documento Hecha")
    expect(shown).toContain("68 de 68 documentos")
    expect(shown).toContain("Lectura con el modelo: a quiénes nombran En curso")
    expect(shown).toContain("12 de 68 documentos")
    expect(shown).toContain("faltan 56")
    expect(shown).toContain("Ahora: «nota de prueba.pdf» · Carpeta de prueba › nueva")
    expect(shown).toContain("Quién es quién Esperando")
    // The scans: how many, the pages that wait and the page being read.
    expect(shown).toContain("Texto de los escaneos En curso")
    expect(shown).toContain("1 de 4 documentos")
    expect(shown).toContain("3 esperan · 9 páginas por reconocer")
    expect(shown).toContain("Ahora: «escaneo de prueba.pdf» · Carpeta de prueba › nueva · página 2 de 5")
    // The domain word comes from the vocabulary.
    expect(shown).toContain("Casos Con errores")
    expect(shown).toContain("Último error del modelo: El proveedor del modelo llegó a su límite de uso.")
    expect(shown).toContain("Vuelve a intentar a las 14:15.")
    expect(shown).toContain("Páginas (artículos) En pausa")
    // Each stage opens.
    expect([...html.matchAll(/aria-expanded="false"/g)].length).toBe(10)
  })

  it("says whether the automatic reading is on, with the switch to pause it", () => {
    useProgressStore.setState({ progress: halfway })
    const on = text(renderToStaticMarkup(<AttentionView />))
    expect(on).toContain("Lectura automática Activa")
    expect(on).toContain("Pausar")
    base.answers["background-reading"] = false
    const off = text(renderToStaticMarkup(<AttentionView />))
    expect(off).toContain("Lectura automática En pausa")
    expect(off).toContain("Activar")
  })

  it("narrows the figures to the origin or folder chosen", () => {
    useProgressStore.setState({ progress: halfway })
    useAttentionViewStore.setState({ scope: { originId: "o1", folder: "nueva" } })
    const origin: Partial<OriginSummary> = { id: "o1", name: "Carpeta de prueba" }
    base.answers["progress:origins"] = [origin]
    base.answers["progress:folders:o1"] = [
      { folder: "nueva", files: 68 },
      { folder: "nueva/anexos", files: 5 },
    ]
    base.answers["progress:o1/nueva"] = progress({ reading: { state: "running", done: 3, total: 68, pending: 65 } })
    const html = renderToStaticMarkup(<AttentionView />)
    expect(text(html)).toContain("3 de 68 documentos")
    expect(text(html)).not.toContain("12 de 68 documentos")
    expect(html).toMatch(/<option value="o1" selected="">Carpeta de prueba<\/option>/)
    expect(html).toMatch(/<option value="nueva" selected="">nueva \(68\)<\/option>/)
    expect(text(html)).toContain("anexos (5)")
    // That folder can be read again from its reading; the whole archive, always (D-465).
    expect(text(html)).toContain("Volver a leer esta carpeta")
    expect(html).toContain('data-reread="all"')
  })

  it("always has the option to read the whole archive again, with what it costs said first (D-465)", () => {
    // Whatever the figures: even before they come.
    for (const shown of [halfway, null]) {
      useProgressStore.setState({ progress: shown })
      const html = renderToStaticMarkup(<AttentionView />)
      const all = text(html.slice(html.indexOf('data-reread="all"')))
      expect(all).toContain("Volver a leer todo el archivo Nada se vuelve a leer solo")
      expect(all).toContain("antes te dice cuántas preguntas son.")
      // The reading of the whole archive does not repeat it.
      expect(text(html).match(/Volver a leer todo/g)?.length).toBe(2)
    }
  })

  it("opens a stage into its documents, each with a button to open it", () => {
    const words = vocabularyFor("legal", "es")
    const items: StageItems = {
      stage: "reading",
      total: 12,
      items: [
        { status: "failed", locationId: 1, contentId: 1, originId: "o1", pagePath: null, name: "escrito.pdf", place: "Carpeta de prueba › nueva", detail: "La respuesta no se pudo usar" },
        { status: "pending", locationId: 2, contentId: 2, originId: "o1", pagePath: null, name: "carta.docx", place: "Carpeta de prueba › nueva", detail: null },
      ],
    }
    const reading = stage("reading", { state: "running", done: 10, total: 12, pending: 1, failed: 1 })
    const html = renderToStaticMarkup(
      <StageLine stage={reading} scope={{}} engineChosen initialOpen initialItems={items} />,
    )
    expect(html).toContain('aria-expanded="true"')
    const shown = text(html)
    expect(shown).toContain("escrito.pdf falló La respuesta no se pudo usar · Carpeta de prueba › nueva Abrir")
    expect(shown).toContain("carta.docx falta Carpeta de prueba › nueva Abrir")
    expect(shown).toContain("Y 10 más.")
    // What failed can be read again from its stage.
    expect(shown).toContain("Volver a leer los que fallaron")
    // The matter of each document, in the words of the vertical.
    const matters = text(
      renderToStaticMarkup(
        <StageItemsList
          stage={stage("matters")}
          words={words}
          items={{
            stage: "matters",
            total: 2,
            items: [
              { status: "done", locationId: 3, contentId: 3, originId: "o1", pagePath: null, name: "a.pdf", place: "", detail: "Reclamo de ejemplo" },
              { status: "done", locationId: 4, contentId: 4, originId: "o1", pagePath: null, name: "b.pdf", place: "", detail: null },
            ],
          }}
        />,
      ),
    )
    expect(matters).toContain("a.pdf hecho Caso: Reclamo de ejemplo")
    expect(matters).toContain("b.pdf hecho Sin caso identificado")
    expect(itemTarget(items.items[0])).toEqual({ view: "document", locationId: 1 })
    expect(itemTarget({ ...items.items[0], locationId: null, pagePath: "wiki/personas/ejemplo.md" })).toEqual({
      view: "wiki-page",
      path: "wiki/personas/ejemplo.md",
    })
    expect(itemTarget({ ...items.items[0], locationId: null, contentId: null })).toEqual({ view: "origins" })
  })
})

describe("the questions that go by themselves in «En curso» (D-484)", () => {
  const round = (patch: Partial<AutoQuestions> = {}): AutoQuestions => ({
    pending: 0,
    asked: 0,
    cap: 50,
    held: null,
    decision: null,
    ...patch,
  })

  it("says how many it is going to ask, and nothing when there are none", () => {
    expect(autoQuestionsText(round())).toBeNull()
    expect(renderToStaticMarkup(<AutoQuestionsLine round={round()} />)).toBe("")
    const going = text(renderToStaticMarkup(<AutoQuestionsLine round={round({ pending: 17, asked: 8 })} />))
    expect(going).toContain("La app va a hacer 17 preguntas de quién es quién y de resumen de las fichas (ya hizo 8 de 25)")
    expect(going).toContain("Si en total pasan de 50, la app pide permiso antes de seguir.")
    expect(going).not.toContain("Seguir")
    // Shown at the top of the tab.
    useAttentionViewStore.setState({ tab: "ongoing" })
    useProgressStore.setState({ progress: progress({}, { autoQuestions: round({ pending: 8 }) }) })
    expect(text(renderToStaticMarkup(<AttentionView />))).toContain("La app va a hacer 8 preguntas de quién es quién")
  })

  it("past the cap, asks for permission to go on or to leave them for later", async () => {
    const html = renderToStaticMarkup(<AutoQuestionsLine round={round({ pending: 60, held: "resolve" })} />)
    expect(html).toContain('data-auto-questions="held"')
    const held = text(html)
    expect(held).toContain("La app espera tu permiso para hacer 60 preguntas al modelo")
    expect(held).toContain("Pasan de 50, así que la app no sigue sin tu permiso.")
    expect(held).toContain("Usan la cuota de tu proveedor.")
    expect(held).toContain("Seguir con las 60")
    expect(held).toContain("Dejarlas para después")
    // Left for later: only the button to do them now.
    const later = text(renderToStaticMarkup(<AutoQuestionsLine round={round({ pending: 60, held: "pages", decision: "later" })} />))
    expect(later).toContain("Dejaste para después 60 preguntas de quién es quién y de resumen de las fichas")
    expect(later).toContain("Hacerlas ahora")
    expect(later).not.toContain("Dejarlas para después")
    // The stage that waits says why.
    const resolve = stage("resolve", { state: "paused", pending: 2, extra: { held: 1 } })
    expect(text(renderToStaticMarkup(<StageLine stage={resolve} scope={{}} engineChosen />))).toContain(
      "En pausa: sus preguntas al modelo esperan tu permiso (más arriba).",
    )
    // What a person says goes to the base.
    vi.mocked(invoke).mockClear()
    await decideAutoQuestions(false)
    expect(invoke).toHaveBeenCalledWith("docbase_auto_questions_decide", { go: false })
  })
})

describe("the scans in «En curso» (P-242)", () => {
  const items: StageItems = {
    stage: "recognition",
    total: 2,
    items: [
      { status: "failed", locationId: 1, contentId: 1, originId: "o1", pagePath: null, name: "foto.jpg", place: "Carpeta de prueba", detail: "Codex did not answer within 300 seconds" },
      { status: "pending", locationId: 2, contentId: 2, originId: "o1", pagePath: null, name: "escaneo.pdf", place: "Carpeta de prueba", detail: null, pages: 7 },
    ],
  }
  const scans = stage("recognition", { state: "paused", done: 0, total: 2, pending: 1, failed: 1, extra: { pages: 8 } })

  beforeEach(() => {
    useAiServicesStore.setState({ services: recognizing("modelo-de-prueba") })
  })

  it("with the reading off, says it and offers to recognize them all, or one by one, and to retry", () => {
    const html = renderToStaticMarkup(
      <StageLine stage={scans} scope={{}} engineChosen backgroundReading={false} initialOpen initialItems={items} />,
    )
    const shown = text(html)
    expect(shown).toContain("Texto de los escaneos En pausa")
    expect(shown).toContain("1 espera · 1 falló · 8 páginas por reconocer")
    expect(shown).toContain("En pausa: la lectura automática está apagada. Podés reconocerlos a pedido")
    expect(shown).toContain("Reconocer todos")
    expect(shown).toContain("Reintentar los que fallaron")
    // Each one, with its pages or why it failed, in words.
    expect(shown).toContain("foto.jpg falló El motor no respondió a tiempo, o no hay conexión. · Carpeta de prueba Reintentar Abrir")
    expect(shown).toContain("escaneo.pdf espera 7 páginas · Carpeta de prueba Reconocer Abrir")
  })

  it("says how to choose the engine when there is none, and offers no button that works", () => {
    useAiServicesStore.setState({ services: NO_SERVICES })
    const html = renderToStaticMarkup(<StageLine stage={scans} scope={{}} engineChosen initialOpen initialItems={items} />)
    const shown = text(html)
    expect(shown).toContain("Falta elegir el motor que reconoce escaneos: en Ajustes → Servicios de IA, elegí uno para «Reconocer escaneos».")
    expect(html).toMatch(/<button[^>]*disabled=""[^>]*>Reconocer todos<\/button>/)
    // Without an engine, the documents have no button to recognize them.
    expect(shown).not.toContain("Reconocer Abrir")
  })

  it("says the line at the foot is waiting for the engine", () => {
    const waiting = progress({ recognition: { state: "paused", pending: 3, total: 3, done: 0 } }, { paused: 3, recognitionEngine: null })
    expect(text(renderToStaticMarkup(<ProgressLine progress={waiting} />))).toBe(
      "Falta elegir el motor que reconoce escaneos: 3 documentos esperan Ver «En curso»",
    )
  })
})

describe("the line at the foot of the window", () => {
  it("says what is being processed and how far it went", () => {
    const html = renderToStaticMarkup(<ProgressLine progress={halfway} />)
    expect(text(html)).toBe("Casos: El proveedor del modelo llegó a su límite de uso. Vuelve a intentar a las 14:15. Ver «En curso»")
    const busy = progress({ reading: { state: "running", done: 12, total: 68, pending: 56 } })
    expect(text(renderToStaticMarkup(<ProgressLine progress={busy} />))).toBe(
      "Procesando: Lectura con el modelo: a quiénes nombran, 12 de 68 Ver «En curso»",
    )
    expect(renderToStaticMarkup(<ProgressLine progress={busy} />)).toContain('data-tone="busy"')
    expect(text(renderToStaticMarkup(<ProgressLine progress={progress()} />))).toBe("Todo al día Ver «En curso»")
  })
})

describe("Inicio", () => {
  it("counts in «En curso» what is really being processed, and what waits paused", () => {
    const home: Home = {
      health: {
        origins: 1,
        unreachableOrigins: 0,
        locations: 68,
        documents: 68,
        // Every file has its text: the old figure said 0 while the model read.
        byState: { processed: 68 },
        copies: 0,
        words: 1000,
        lastScanAt: null,
      },
      origins: [],
      activity: [],
    }
    base.answers.home = home
    useProgressStore.setState({ progress: halfway })
    const shown = text(renderToStaticMarkup(<HomeView />))
    expect(shown).toContain("En curso 59 2 en pausa")
    expect(shown).toContain("Nada requiere tu atención. La app está trabajando")
    useAttentionStore.setState({ bySource: { prueba: [notice("a", "problem", "Un problema inventado")] } })
    expect(text(renderToStaticMarkup(<HomeView />))).toContain("Además, hay 59 documentos en proceso.")
  })
})
