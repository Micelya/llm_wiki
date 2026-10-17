/**
 * Ingest in the archive's wiki, where the knowledge layer writes the
 * pages of people, organizations and matters (D-200): ingest writes the
 * page of the document and the concepts, never an entity page, and its
 * instructions are those of a law firm. Invented names only (R-24).
 */
import { describe, it, expect, beforeEach, afterEach, vi } from "vitest"
import path from "node:path"
import fs from "node:fs/promises"
import { realFs, createTempProject, fileExists } from "@/test-helpers/fs-temp"

vi.mock("@/commands/fs", () => realFs)

let archive = true
vi.mock("@/lib/knowledge-layer", async () => {
  const actual = await vi.importActual<typeof import("@/lib/knowledge-layer")>("@/lib/knowledge-layer")
  return {
    ...actual,
    usesKnowledgeLayer: vi.fn(async () => archive),
    tellWikiLanguage: vi.fn(),
  }
})

let pendingResponses: string[] = []
const prompts: string[] = []
vi.mock("./llm-client", () => ({
  streamChat: vi.fn(async (_cfg, messages: { role: string; content: string }[], cb) => {
    prompts.push(messages.map((m) => m.content).join("\n"))
    cb.onToken(pendingResponses.shift() ?? "")
    cb.onDone()
  }),
}))

import { autoIngest, buildAnalysisPrompt, buildChunkAnalysisSystemPrompt, buildGenerationPrompt } from "./ingest"
import { tellWikiLanguage } from "@/lib/knowledge-layer"
import { useWikiStore } from "@/stores/wiki-store"
import { useReviewStore } from "@/stores/review-store"
import { useActivityStore } from "@/stores/activity-store"

const GENERATION = [
  "---FILE: wiki/sources/contrato.md---",
  "---",
  "type: source",
  "title: Contrato de locación",
  "sources: [\"contrato.txt\"]",
  "---",
  "",
  "# Contrato de locación",
  "",
  "Ejemplo Uno S.A. alquila a Persona Dos por $ 100.000 mensuales.",
  "---END FILE---",
  "",
  "---FILE: wiki/entities/persona-dos.md---",
  "---",
  "type: entity",
  "title: Persona Dos",
  "sources: [\"contrato.txt\"]",
  "---",
  "",
  "# Persona Dos",
  "---END FILE---",
  "",
  "---FILE: wiki/people/ejemplo-uno.md---",
  "---",
  "type: entity",
  "title: Ejemplo Uno S.A.",
  "sources: [\"contrato.txt\"]",
  "---",
  "",
  "# Ejemplo Uno S.A.",
  "---END FILE---",
  "",
  "---FILE: wiki/concepts/locacion.md---",
  "---",
  "type: concept",
  "title: Locación",
  "sources: [\"contrato.txt\"]",
  "---",
  "",
  "# Locación",
  "---END FILE---",
].join("\n")

let tmp: { path: string; cleanup: () => Promise<void> } | undefined

beforeEach(async () => {
  archive = true
  pendingResponses = []
  prompts.length = 0
  useReviewStore.setState({ items: [] })
  useActivityStore.setState({ items: [] })
  tmp = await createTempProject("ingest-knowledge-layer")
  await fs.mkdir(path.join(tmp.path, "wiki", "entities"), { recursive: true })
  await fs.mkdir(path.join(tmp.path, "raw", "sources"), { recursive: true })
  await fs.writeFile(path.join(tmp.path, "schema.md"), "# schema\n")
  await fs.writeFile(path.join(tmp.path, "wiki", "index.md"), "# Wiki Index\n\n## Entities\n\n## Concepts\n")
  await fs.writeFile(
    path.join(tmp.path, "raw", "sources", "contrato.txt"),
    "Contrato de locación entre Ejemplo Uno S.A. y Persona Dos por $ 100.000 mensuales.",
  )
  useWikiStore.setState({
    project: { name: "t", path: tmp.path, createdAt: 0, purposeText: "", fileTree: [] } as unknown as ReturnType<
      typeof useWikiStore.getState
    >["project"],
  })
  useWikiStore.getState().setOutputLanguage("Spanish")
  useWikiStore.getState().setLlmConfig({
    provider: "openai",
    apiKey: "k",
    model: "m",
    ollamaUrl: "",
    customEndpoint: "",
    maxContextSize: 128000,
  })
})

afterEach(async () => {
  await tmp?.cleanup()
  tmp = undefined
  useWikiStore.getState().setOutputLanguage("auto")
})

async function ingest(): Promise<void> {
  pendingResponses = ["## Document\nUn contrato.", GENERATION]
  await autoIngest(tmp!.path, path.join(tmp!.path, "raw", "sources", "contrato.txt"), useWikiStore.getState().llmConfig)
}

describe("ingest in the archive's wiki", () => {
  it("writes the page of the document and the concepts, never an entity page", async () => {
    await ingest()
    expect(await fileExists(path.join(tmp!.path, "wiki", "sources", "contrato.md"))).toBe(true)
    expect(await fileExists(path.join(tmp!.path, "wiki", "concepts", "locacion.md"))).toBe(true)
    expect(await fileExists(path.join(tmp!.path, "wiki", "entities", "persona-dos.md"))).toBe(false)
    expect(await fileExists(path.join(tmp!.path, "wiki", "people", "ejemplo-uno.md"))).toBe(false)
    // The instructions said so, and the language went to the base.
    expect(prompts[1]).toContain("do not write any file under wiki/entities/")
    expect(prompts[0]).toContain("Do not decide or recommend pages for them")
    expect(tellWikiLanguage).toHaveBeenCalledWith("Spanish")
  })

  it("lists every page in the index, by type, in the wiki's language", async () => {
    await fs.writeFile(
      path.join(tmp!.path, "wiki", "entities", "persona-dos-0123abcd.md"),
      "---\ntype: entity\ntitle: \"Persona Dos\"\nentity_kind: persona\ngenerated_by: micelya-conocimiento\n---\n\n# Persona Dos\n",
    )
    await ingest()
    const index = await fs.readFile(path.join(tmp!.path, "wiki", "index.md"), "utf8")
    expect(index).toContain("# Índice del wiki")
    expect(index).toContain("## Documentos (1)\n\n- [[sources/contrato|Contrato de locación]]")
    expect(index).toContain("## Personas, organizaciones y asuntos (1)\n\n### Personas (1)\n\n- [[entities/persona-dos-0123abcd|Persona Dos]]")
    expect(index).toContain("## Conceptos (1)")
    expect(index).not.toContain("## Entities")
  })

  it("keeps writing entity pages in any other project", async () => {
    archive = false
    await ingest()
    expect(await fileExists(path.join(tmp!.path, "wiki", "entities", "persona-dos.md"))).toBe(true)
    expect(prompts[1]).not.toContain("do not write any file under wiki/entities/")
  })
})

describe("instructions of a law firm", () => {
  it("speak of parties, roles, amounts and deadlines, and forbid outside knowledge", () => {
    const analysis = buildAnalysisPrompt("", "", "")
    const generation = buildGenerationPrompt("", "", "", "x.pdf")
    const chunk = buildChunkAnalysisSystemPrompt("", "", "", "x")
    for (const prompt of [analysis, generation, chunk]) {
      expect(prompt).not.toMatch(/research analyst|benchmark|GPT-5|personal wiki/i)
    }
    expect(analysis).toContain("You assist a law firm")
    expect(analysis).toContain("Dates, amounts, deadlines, obligations")
    expect(analysis).toContain("do not add public or general knowledge")
    expect(generation).toContain("never add public or general knowledge")
    expect(generation).not.toContain("high-confidence public knowledge")
    expect(generation).toContain("Never add public or general knowledge about a person")
  })
})
