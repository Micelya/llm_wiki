/**
 * The wiki's index (`wiki/index.md`): every page of the wiki, grouped by
 * the type its header says, written by the app (D-201).
 *
 * It used to keep only the 200 pages most recently written, under one
 * section, and left the type sections empty. Now it is built again from
 * the pages on disk whenever a page changes: what ingest writes and what
 * the knowledge layer projects. The headings are in the wiki's language,
 * from the vocabulary.
 */
import { listDirectory, readFile, writeFile } from "@/commands/fs"
import { parseFrontmatter } from "@/lib/frontmatter"
import { normalizePath } from "@/lib/path-utils"
import { useWikiStore } from "@/stores/wiki-store"
import { vocabulary, wikiVocabulary, type Vocabulary } from "@/vocabulary"
import type { FileNode } from "@/types/wiki"

/** A page as the index lists it. */
export interface IndexedPage {
  /** Path inside the project: `wiki/sources/carta.md`. */
  path: string
  title: string
  type: string
  /** For a page of an entity of the knowledge layer: persona, organizacion, asunto. */
  entityKind?: string
}

const AGGREGATES = new Set(["wiki/index.md", "wiki/overview.md", "wiki/log.md"])
/** The order of the sections; the types the vocabulary does not name come after, then the rest. */
const TYPE_ORDER = ["source", "entity", "concept", "query", "comparison", "synthesis"]

function target(path: string): string {
  return normalizePath(path).replace(/^wiki\//, "").replace(/\.md$/i, "")
}

function byTitle(a: IndexedPage, b: IndexedPage): number {
  return a.title.localeCompare(b.title, undefined, { sensitivity: "base" }) || a.path.localeCompare(b.path)
}

/** The index of these pages, every one under the heading of its type. */
export function buildWikiIndex(pages: IndexedPage[], words: Vocabulary["wikiIndex"]): string {
  const byType = new Map<string, IndexedPage[]>()
  for (const page of pages) {
    if (AGGREGATES.has(normalizePath(page.path).toLowerCase())) continue
    const type = page.type && words.types[page.type] ? page.type : ""
    const list = byType.get(type) ?? []
    list.push(page)
    byType.set(type, list)
  }
  const order = [
    ...TYPE_ORDER.filter((type) => words.types[type]),
    ...Object.keys(words.types).filter((type) => !TYPE_ORDER.includes(type)),
    "",
  ]
  const lines = [`# ${words.title}`, "", words.intro, ""]
  for (const type of order) {
    const list = byType.get(type)
    if (!list || list.length === 0) continue
    lines.push(`## ${type ? words.types[type] : words.other} (${list.length})`, "")
    const kinds = new Map<string, IndexedPage[]>()
    for (const page of list) {
      const kind = type === "entity" && page.entityKind && words.entityKinds[page.entityKind] ? page.entityKind : ""
      const group = kinds.get(kind) ?? []
      group.push(page)
      kinds.set(kind, group)
    }
    const kindOrder = [...Object.keys(words.entityKinds), ""]
    const grouped = kinds.size > 1 || !kinds.has("")
    for (const kind of kindOrder) {
      const group = kinds.get(kind)
      if (!group) continue
      if (grouped) lines.push(`### ${kind ? words.entityKinds[kind] : words.other} (${group.length})`, "")
      for (const page of group.sort(byTitle)) {
        lines.push(`- [[${target(page.path)}|${page.title.replace(/[[\]|]/g, " ").trim()}]]`)
      }
      lines.push("")
    }
  }
  return `${lines.join("\n").trimEnd()}\n`
}

function markdownFiles(nodes: FileNode[], out: string[] = []): string[] {
  for (const node of nodes) {
    if (node.is_dir) {
      if (!node.name.startsWith(".")) markdownFiles(node.children ?? [], out)
    } else if (node.name.toLowerCase().endsWith(".md")) {
      out.push(node.path)
    }
  }
  return out
}

/**
 * The language the wiki is written in: the one chosen for the open
 * project or the app, or the vocabulary's when none was ("auto" guesses
 * it from each document, which is no language for a whole index).
 */
export function wikiLanguageNow(): string {
  const configured = useWikiStore.getState().outputLanguage
  return configured && configured !== "auto" ? configured : vocabulary().wikiLanguage
}

/** The pages of the wiki of the project at `projectPath`, read from disk. */
export async function readWikiPages(projectPath: string): Promise<IndexedPage[]> {
  const root = normalizePath(projectPath).replace(/\/+$/, "")
  const tree = await listDirectory(`${root}/wiki`).catch(() => [] as FileNode[])
  const pages: IndexedPage[] = []
  for (const file of markdownFiles(tree)) {
    const absolute = normalizePath(file)
    const path = absolute.startsWith(`${root}/`) ? absolute.slice(root.length + 1) : absolute
    if (AGGREGATES.has(path.toLowerCase())) continue
    const content = await readFile(absolute).catch(() => "")
    const frontmatter = parseFrontmatter(content).frontmatter ?? {}
    const title = typeof frontmatter.title === "string" && frontmatter.title.trim()
      ? frontmatter.title.trim()
      : (path.split("/").pop() ?? path).replace(/\.md$/i, "")
    const type = typeof frontmatter.type === "string" ? frontmatter.type.trim() : ""
    const entityKind = typeof frontmatter.entity_kind === "string" ? frontmatter.entity_kind.trim() : undefined
    pages.push({ path, title, type, entityKind })
  }
  return pages
}

/**
 * Writes the index of the project's wiki again, from every page on disk.
 * Returns whether it changed.
 */
export async function rebuildWikiIndex(projectPath: string): Promise<boolean> {
  const root = normalizePath(projectPath).replace(/\/+$/, "")
  const words = wikiVocabulary(wikiLanguageNow()).wikiIndex
  const text = buildWikiIndex(await readWikiPages(root), words)
  const indexPath = `${root}/wiki/index.md`
  const before = await readFile(indexPath).catch(() => "")
  if (before === text) return false
  await writeFile(indexPath, text)
  return true
}
