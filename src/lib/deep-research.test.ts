import { describe, expect, it, vi } from "vitest"
const { saveResearch, streamChat, webSearch } = vi.hoisted(() => ({
  saveResearch: vi.fn(),
  streamChat: vi.fn(),
  webSearch: vi.fn(),
}))
// No real question to a model and no real search: both are simulated, and
// the base that keeps the research too.
vi.mock("@/commands/docbase", () => ({ saveResearch }))
vi.mock("./llm-client", () => ({ streamChat }))
vi.mock("./web-search", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./web-search")>()),
  webSearch,
}))

import {
  citedResearchSourceIndexes,
  citedResearchSources,
  collectResearchSources,
  filterResearchSourcesByRelevance,
  noResearchSourcesTaskPatch,
  queueResearch,
  rejectedResearchSourcesTaskPatch,
  researchRelevanceRequestOverrides,
  researchSystemPrompt,
  resolveReviewForSavedResearch,
  validateResearchSynthesis,
} from "./deep-research"
import type { SearchApiConfig } from "@/stores/wiki-store"
import type { WebSearchResult } from "./web-search"
import { useWikiStore } from "@/stores/wiki-store"
import { useResearchStore } from "@/stores/research-store"
import { useReviewStore, type ReviewItem } from "@/stores/review-store"

const webResult: WebSearchResult = {
  title: "Web",
  url: "https://example.com/web",
  snippet: "web snippet",
  source: "example.com",
}

const localResult: WebSearchResult = {
  title: "Local",
  url: "file:///C:/docs/local.md",
  snippet: "local snippet",
  source: "AnyTXT",
}

function config(patch: Partial<SearchApiConfig>): SearchApiConfig {
  return {
    provider: "none",
    apiKey: "",
    ...patch,
  }
}

describe("validateResearchSynthesis", () => {
  const substantiveChinese = [
    "解离是一类涉及记忆、身份、意识与环境感知连续性受扰的心理现象。[1]",
    "现有研究通常区分短暂的正常体验与造成显著痛苦或功能损害的临床表现。评估时需要结合持续时间、诱因、共病情况以及对日常生活的影响，不能仅凭单一症状下结论。[2]",
    "不同理论模型对其机制仍有分歧，因此研究结论需要结合样本来源、测量工具与临床背景解释。现有证据更适合支持分层评估，而不是把单项自评结果直接视为诊断。",
  ].join("\n\n")

  it("rejects empty, reasoning-only, and short output", () => {
    expect(validateResearchSynthesis("", 2).valid).toBe(false)
    expect(validateResearchSynthesis("<think>private reasoning only</think>", 2).valid).toBe(false)
    expect(validateResearchSynthesis("## 概述\n\n内容很短。[1]", 2).valid).toBe(false)
  })

  it("accepts substantive Unicode prose with a valid citation", () => {
    const result = validateResearchSynthesis(substantiveChinese, 2)

    expect(result.valid).toBe(true)
    expect(result.citedSourceIndexes).toEqual([1, 2])
  })

  it("rejects substantive output that cites no collected source", () => {
    const result = validateResearchSynthesis(substantiveChinese.replace(/\[\d+\]/g, ""), 2)

    expect(result.valid).toBe(false)
    expect(result.error).toContain("did not cite")
  })

  it("extracts valid citation groups and bounded ranges", () => {
    expect(citedResearchSourceIndexes("Evidence [3, 1] and comparison [4-5], ignore [0] [8].", 5))
      .toEqual([1, 3, 4, 5])
  })
})

describe("noResearchSourcesTaskPatch", () => {
  it("marks source failures as an error instead of completed", () => {
    expect(noResearchSourcesTaskPatch(["Firecrawl blocked this IP", "AnyTXT offline"])).toEqual({
      status: "error",
      synthesis: "",
      error: "Firecrawl blocked this IP\nAnyTXT offline",
    })
  })

  it("marks an empty successful search as done", () => {
    expect(noResearchSourcesTaskPatch([])).toEqual({
      status: "done",
      synthesis: "No research sources found.",
      error: null,
    })
  })
})

describe("rejectedResearchSourcesTaskPatch", () => {
  it("reports rejected candidates and preserves source failures", () => {
    expect(rejectedResearchSourcesTaskPatch(2, ["Web provider timed out"])).toEqual({
      status: "error",
      synthesis: "",
      error: "2 candidate source(s) were found, but all were rejected by the relevance and evidence-quality gate. Refine the research topic or queries and retry.\nWeb provider timed out",
    })
  })
})

describe("review-linked research", () => {
  const review: ReviewItem = {
    id: "review-1",
    type: "suggestion",
    title: "Research this",
    description: "Needs evidence",
    options: [],
    resolved: false,
    createdAt: 1,
  }

  it("resolves the source review only after a saved result exists", () => {
    useWikiStore.setState({ project: { id: "p1", name: "Project", path: "/project" } })
    useReviewStore.setState({ items: [review] })
    useResearchStore.setState({
      tasks: [{
        id: "research-1",
        topic: "topic",
        sourceReviewId: review.id,
        status: "done",
        webResults: [],
        synthesis: "This completed synthesis contains enough substantive analysis to explain the evidence, its limitations, and the conclusions that can reasonably be drawn. It also distinguishes established findings from open questions so the saved research is useful for later review.",
        savedId: 7,
        error: null,
        createdAt: 1,
      }],
    })

    expect(resolveReviewForSavedResearch(
      "research-1",
      7,
    )).toBe(true)
    expect(useReviewStore.getState().items[0]).toMatchObject({
      resolved: true,
      resolvedAction: "Investigación guardada en la base (7)",
    })
  })

  it("rejects callbacks before the linked task reaches the matching saved state", () => {
    useWikiStore.setState({ project: { id: "p1", name: "Project", path: "/project" } })
    useReviewStore.setState({ items: [review] })
    useResearchStore.setState({
      tasks: [{
        id: "research-1",
        topic: "topic",
        sourceReviewId: review.id,
        status: "saving",
        webResults: [],
        synthesis: "answer",
        savedId: null,
        error: null,
        createdAt: 1,
      }],
    })

    expect(resolveReviewForSavedResearch(
      "research-1",
      7,
    )).toBe(false)
    expect(useReviewStore.getState().items[0].resolved).toBe(false)
  })

  it("does not resolve a saved task with incomplete synthesis", () => {
    useWikiStore.setState({ project: { id: "p1", name: "Project", path: "/project" } })
    useReviewStore.setState({ items: [review] })
    useResearchStore.setState({
      tasks: [{
        id: "research-1",
        topic: "topic",
        sourceReviewId: review.id,
        status: "done",
        webResults: [webResult],
        synthesis: "[1]",
        savedId: 7,
        error: null,
        createdAt: 1,
      }],
    })

    expect(resolveReviewForSavedResearch(
      "research-1",
      7,
    )).toBe(false)
    expect(useReviewStore.getState().items[0].resolved).toBe(false)
  })
})

describe("collectResearchSources", () => {
  it("uses only Web Search when source mode is web", async () => {
    const webSearch = vi.fn().mockResolvedValue([webResult])
    const anyTxtSearch = vi.fn().mockResolvedValue([localResult])

    const out = await collectResearchSources(
      ["alpha"],
      config({ deepResearchSource: "web", provider: "tavily", apiKey: "tvly" }),
      "/project",
      { webSearch, anyTxtSearch },
    )

    expect(webSearch).toHaveBeenCalledTimes(1)
    expect(anyTxtSearch).not.toHaveBeenCalled()
    expect(out.results).toEqual([webResult])
  })

  it("uses only AnyTXT when source mode is anytxt", async () => {
    const webSearch = vi.fn().mockResolvedValue([webResult])
    const anyTxtSearch = vi.fn().mockResolvedValue([localResult])

    const out = await collectResearchSources(
      ["alpha"],
      config({
        deepResearchSource: "anytxt",
        provider: "tavily",
        apiKey: "tvly",
        anyTxt: { endpoint: "http://127.0.0.1:9920" },
      }),
      "/project",
      { webSearch, anyTxtSearch },
    )

    expect(webSearch).not.toHaveBeenCalled()
    expect(anyTxtSearch).toHaveBeenCalledTimes(1)
    expect(anyTxtSearch.mock.calls[0][0]).toEqual(["alpha"])
    expect(out.results).toEqual([localResult])
  })

  it("uses both sources concurrently and deduplicates by URL", async () => {
    const duplicate = { ...localResult, url: webResult.url }
    const webSearch = vi.fn().mockResolvedValue([webResult])
    const anyTxtSearch = vi.fn().mockResolvedValue([duplicate, localResult])

    const out = await collectResearchSources(
      ["alpha"],
      config({
        deepResearchSource: "both",
        provider: "tavily",
        apiKey: "tvly",
        anyTxt: { endpoint: "http://127.0.0.1:9920" },
      }),
      "/project",
      { webSearch, anyTxtSearch },
    )

    expect(webSearch).toHaveBeenCalledTimes(1)
    expect(anyTxtSearch).toHaveBeenCalledTimes(1)
    expect(out.results).toEqual([webResult, localResult])
  })

  it("keeps web results when AnyTXT fails and exposes the source error", async () => {
    const webSearch = vi.fn().mockResolvedValue([webResult])
    const anyTxtSearch = vi.fn().mockRejectedValue(new Error("Check that ATGUI.exe is running"))

    const out = await collectResearchSources(
      ["alpha"],
      config({
        deepResearchSource: "both",
        provider: "tavily",
        apiKey: "tvly",
        anyTxt: { endpoint: "http://127.0.0.1:9920" },
      }),
      "/project",
      { webSearch, anyTxtSearch },
    )

    expect(out.results).toEqual([webResult])
    expect(out.errors).toEqual(["Check that ATGUI.exe is running"])
  })

  it("skips Web Search in both mode when no web provider is configured", async () => {
    const webSearch = vi.fn().mockResolvedValue([webResult])
    const anyTxtSearch = vi.fn().mockResolvedValue([localResult])

    const out = await collectResearchSources(
      ["alpha"],
      config({
        deepResearchSource: "both",
        provider: "none",
        anyTxt: { endpoint: "http://127.0.0.1:9920" },
      }),
      "/project",
      { webSearch, anyTxtSearch },
    )

    expect(webSearch).not.toHaveBeenCalled()
    expect(anyTxtSearch).toHaveBeenCalledTimes(1)
    expect(out.results).toEqual([localResult])
  })

  it("returns no results for blank queries", async () => {
    const webSearch = vi.fn().mockResolvedValue([webResult])
    const anyTxtSearch = vi.fn().mockResolvedValue([localResult])

    const out = await collectResearchSources(
      [" ", ""],
      config({ deepResearchSource: "both", provider: "tavily", apiKey: "tvly" }),
      "/project",
      { webSearch, anyTxtSearch },
    )

    expect(webSearch).not.toHaveBeenCalled()
    expect(anyTxtSearch).not.toHaveBeenCalled()
    expect(out.results).toEqual([])
  })

  it("logs once when research sources are capped", async () => {
    const infoSpy = vi.spyOn(console, "info").mockImplementation(() => {})
    const webSearch = vi.fn().mockResolvedValue(
      Array.from({ length: 25 }, (_, index) => ({
        title: `Result ${index}`,
        url: `https://example.com/${index}`,
        snippet: "snippet",
        source: "example.com",
      })),
    )
    const anyTxtSearch = vi.fn().mockResolvedValue([])

    const out = await collectResearchSources(
      ["alpha", "beta"],
      config({ deepResearchSource: "web", provider: "tavily", apiKey: "tvly" }),
      "/project",
      { webSearch, anyTxtSearch },
    )

    expect(out.results).toHaveLength(20)
    expect(infoSpy).toHaveBeenCalledTimes(1)
    infoSpy.mockRestore()
  })

  it("applies the relevance filter after deduplication and capping", async () => {
    const lowQuality = {
      title: "Unrelated homework answers",
      url: "https://documents.test/homework",
      snippet: "Exam answers unrelated to the research topic",
      source: "documents.test",
    }
    const relevanceFilter = vi.fn().mockImplementation(async (_queries, results) => [results[0]])
    const webSearch = vi.fn().mockResolvedValue([webResult, lowQuality])
    const anyTxtSearch = vi.fn().mockResolvedValue([])

    const out = await collectResearchSources(
      ["alpha"],
      config({ deepResearchSource: "web", provider: "tavily", apiKey: "tvly" }),
      "/project",
      { webSearch, anyTxtSearch },
      { llmConfig: {} as never, researchTopic: "Alpha topic", relevanceFilter },
    )

    expect(relevanceFilter).toHaveBeenCalledWith(
      ["alpha"],
      [webResult, lowQuality],
      expect.anything(),
      "Alpha topic",
    )
    expect(out.results).toEqual([webResult])
    expect(out.candidateCount).toBe(2)
    expect(out.rejectedCount).toBe(1)
  })
})

describe("filterResearchSourcesByRelevance", () => {
  it("uses a bounded non-reasoning request for the quality gate", () => {
    const overrides = researchRelevanceRequestOverrides()

    expect(overrides).toMatchObject({
      temperature: 0,
      max_tokens: 512,
      reasoning: { mode: "off" },
    })
  })

  it("keeps only indexes approved by the quality judge", async () => {
    const noisy: WebSearchResult = {
      title: "Courseware and exam answers",
      url: "https://documents.test/exam",
      snippet: "Unrelated homework material",
      source: "documents.test",
    }
    const judge = vi.fn().mockResolvedValue("{\"keep\":[1]}")

    const filtered = await filterResearchSourcesByRelevance(
      ["alpha project mechanism"],
      [webResult, noisy],
      {} as never,
      "Alpha project mechanism",
      judge,
    )

    expect(filtered).toEqual([webResult])
    const messages = judge.mock.calls[0][1]
    expect(messages[0].content).toContain("document-sharing farms")
    expect(messages[0].content).toContain("unrelated meanings of ambiguous terms")
    expect(messages[0].content).toContain("contradictory evidence")
    expect(JSON.parse(messages[1].content).researchTopic).toBe("Alpha project mechanism")
  })

  it("fails open when the quality judge response is malformed", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    const results = [webResult]
    const filtered = await filterResearchSourcesByRelevance(
      ["alpha"],
      results,
      {} as never,
      "Alpha",
      vi.fn().mockResolvedValue("not json"),
    )
    expect(filtered).toEqual(results)
    expect(warn).toHaveBeenCalled()
    warn.mockRestore()
  })

  it("allows the judge to reject every irrelevant candidate", async () => {
    const filtered = await filterResearchSourcesByRelevance(
      ["alpha"],
      [webResult],
      {} as never,
      "Alpha",
      vi.fn().mockResolvedValue("```json\n{\"keep\":[]}\n```"),
    )
    expect(filtered).toEqual([])
  })

  it("accepts numeric string indexes and fails open on zero-based output", async () => {
    const second = { ...webResult, title: "Second", url: "https://example.com/second" }
    await expect(filterResearchSourcesByRelevance(
      ["alpha"],
      [webResult, second],
      {} as never,
      "Alpha",
      vi.fn().mockResolvedValue("{\"keep\":[\"1\"]}"),
    )).resolves.toEqual([webResult])
    await expect(filterResearchSourcesByRelevance(
      ["alpha"],
      [webResult, second],
      {} as never,
      "Alpha",
      vi.fn().mockResolvedValue("{\"keep\":[0]}"),
    )).resolves.toEqual([webResult, second])
  })

  it("fails open when non-empty indexes contain no valid candidate", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    const filtered = await filterResearchSourcesByRelevance(
      ["alpha"],
      [webResult],
      {} as never,
      "Alpha",
      vi.fn().mockResolvedValue("{\"keep\":[99]}"),
    )
    expect(filtered).toEqual([webResult])
    expect(warn).toHaveBeenCalledWith(
      expect.stringContaining("relevance gate failed"),
      expect.anything(),
    )
    warn.mockRestore()
  })

  it("parses the last compact keep object around prose and reasoning", async () => {
    const second = { ...webResult, title: "Second", url: "https://example.com/second" }
    const filtered = await filterResearchSourcesByRelevance(
      ["alpha"],
      [webResult, second],
      {} as never,
      "Alpha",
      vi.fn().mockResolvedValue("<think>{draft}</think>Result: {\"keep\":[2]} done"),
    )
    expect(filtered).toEqual([second])
  })
})

describe("collectResearchSources relevance origins", () => {
  it("filters web candidates but always preserves local AnyTXT results", async () => {
    const webSearch = vi.fn().mockResolvedValue([webResult])
    const anyTxtSearch = vi.fn().mockResolvedValue([localResult])
    const relevanceFilter = vi.fn().mockResolvedValue([])

    const out = await collectResearchSources(
      ["alpha"],
      config({
        deepResearchSource: "both",
        provider: "tavily",
        apiKey: "tvly",
        anyTxt: { endpoint: "http://127.0.0.1:9920" },
      }),
      "/project",
      { webSearch, anyTxtSearch },
      { llmConfig: {} as never, researchTopic: "Alpha", relevanceFilter },
    )

    expect(relevanceFilter).toHaveBeenCalledWith(["alpha"], [webResult], expect.anything(), "Alpha")
    expect(out.results).toEqual([localResult])
    expect(out.candidateCount).toBe(2)
    expect(out.rejectedCount).toBe(1)
  })
})

describe("research kept in the base (D-414)", () => {
  const substantive = "The public record shows the company was registered in the city and later changed its name [1]. A second note reports a dispute that was settled out of court, with no further filings found [2]."

  it("tells the model whom the research is of and that it is apart from the documents", () => {
    const prompt = researchSystemPrompt("Ana Prueba", "Ana Prueba")
    expect(prompt).toContain("about Ana Prueba")
    expect(prompt).toContain("apart from the user's own documents")
    expect(researchSystemPrompt("tema suelto")).not.toContain("## Subject")
  })

  it("keeps only the cited sources, with their numbers", () => {
    const results = [webResult, { ...webResult, title: "Otra", url: "https://example.com/2" }]
    expect(citedResearchSources([2], results)).toEqual([
      { index: 2, title: "Otra", url: "https://example.com/2", source: "example.com" },
    ])
    expect(citedResearchSources([5], results)).toEqual([])
  })

  it("saves the synthesis in the base, of its object, with the sources it cites", async () => {
    useResearchStore.setState({ tasks: [], panelOpen: false })
    webSearch.mockResolvedValue([
      webResult,
      { ...webResult, title: "Segunda", url: "https://example.com/2" },
    ])
    streamChat.mockImplementation(async (_config, messages, callbacks) => {
      const gate = String(messages[0].content).includes("quality gate")
      callbacks.onToken(gate ? '{"keep":[1,2]}' : substantive)
      callbacks.onDone()
    })
    saveResearch.mockResolvedValue({ id: 41, objectId: "ent-a", topic: "Ana Prueba", body: substantive, sources: [], createdAt: 1 })
    const searchConfig = { provider: "tavily", apiKey: "k" } as SearchApiConfig
    const llm = { provider: "openai", apiKey: "k", model: "m" } as never

    const id = queueResearch("Ana Prueba", llm, searchConfig, {
      target: { objectId: "ent-a", name: "Ana Prueba" },
      openPanel: false,
    })
    expect(useResearchStore.getState().panelOpen).toBe(false)
    await vi.waitFor(() => expect(useResearchStore.getState().tasks.find((t) => t.id === id)?.status).toBe("done"))

    expect(saveResearch).toHaveBeenCalledTimes(1)
    const [objectId, topic, body, sources] = saveResearch.mock.calls[0]
    expect([objectId, topic]).toEqual(["ent-a", "Ana Prueba"])
    expect(body).toBe(substantive)
    expect(sources.map((source: { index: number }) => source.index)).toEqual([1, 2])
    expect(useResearchStore.getState().tasks.find((t) => t.id === id)?.savedId).toBe(41)
    const system = streamChat.mock.calls[streamChat.mock.calls.length - 1][1][0].content as string
    expect(system).toContain("about Ana Prueba")
    expect(system).not.toContain("Wiki Index")
  })
})
