import { articleBody } from "@/components/knowledge/knowledge"

/** One name of a list of the overview: its link, its kind when it says it, and in how many documents. */
export interface OverviewItem {
  /** The name of its page (`[[stem|…]]`): following it opens its screen. */
  page: string
  name: string
  /** «Persona», «Organización»…: only the most named say it. */
  kind: string | null
  documents: number
}

/** The overview of the archive, read from what the projection wrote (D-240, D-407). */
export interface Overview {
  /** The figures of its first list, in order: «Documentos leídos: 1284», «Documentos cuya lectura falló: 3». */
  figures: { label: string; value: number }[]
  /** Who has a page, by kind: «Persona: 212 · Organización: 64». */
  withPage: { kind: string; count: number }[]
  /** The most named, by the documents where they have a role of substance. */
  mostNamed: OverviewItem[]
  /** Every matter with its page, the one with more documents first. */
  matters: OverviewItem[]
}

const ITEM = /^- \[\[([^|\]]+)\|([^\]]+)\]\](?: · (.+?))? — (\d+) \S/
const FIGURE = /^- ([^:]+): (\d+)\s*$/
const PAIRS = /^- ([^:]+):\s*(.*)$/
const PAIR = /^(.+?): (\d+)$/

/**
 * Reads the overview the app writes (`pages.rs`, `overview`): its figures,
 * who has a page by kind, and its two lists, the most named (whose items
 * say their kind) and the matters (whose items do not). `null` when the
 * text is not of that shape, so that the screen shows it as an article.
 */
export function parseOverview(text: string): Overview | null {
  const lines = articleBody(text).split("\n")
  const overview: Overview = { figures: [], withPage: [], mostNamed: [], matters: [] }
  const lists: OverviewItem[][] = []
  let current: OverviewItem[] | null = null
  for (const raw of lines) {
    const line = raw.trimEnd()
    if (line.startsWith("## ")) {
      current = []
      lists.push(current)
      continue
    }
    if (!line.startsWith("- ")) continue
    if (current) {
      const item = ITEM.exec(line)
      if (!item) return null
      current.push({ page: item[1].trim(), name: item[2].trim(), kind: item[3]?.trim() || null, documents: Number(item[4]) })
      continue
    }
    const figure = FIGURE.exec(line)
    if (figure) {
      overview.figures.push({ label: figure[1].trim(), value: Number(figure[2]) })
      continue
    }
    const pairs = PAIRS.exec(line)
    if (!pairs) return null
    for (const part of pairs[2].split(" · ").filter((part) => part.trim())) {
      const pair = PAIR.exec(part.trim())
      if (pair) overview.withPage.push({ kind: pair[1].trim(), count: Number(pair[2]) })
    }
  }
  if (overview.figures.length === 0) return null
  for (const list of lists) {
    if (list.length === 0) continue
    if (list.every((item) => item.kind !== null)) overview.mostNamed = list
    else overview.matters = list
  }
  return overview
}
