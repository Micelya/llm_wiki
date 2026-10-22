/**
 * What the floating card of a link shows of the article it names
 * (P-246): its type and a short summary, read from the article the app
 * already wrote in the base, never asked to a model. The kind of a person
 * or organization is the italic word under the title (`*Persona* · …`),
 * the summary the first paragraph of text after it: a document's topic,
 * or the first sentences of what the documents say of someone.
 */

export interface ArticlePreview {
  /** `entity_kind` of its header (`persona`, `organizacion`…), when it has one. */
  kind: string | null
  /** The word under its title (`Persona`), when it has one. */
  kindLabel: string | null
  /** Its first paragraph of text, plain and cut short; null when it has none. */
  summary: string | null
}

/** Characters of the summary at most; it is cut at a word, with an ellipsis. */
export const SUMMARY_LENGTH = 260

const KIND_LINE = /^\*([^*\n]+)\*(?:\s+·|\s*$)/

/** An article's type and short summary, from its markdown (with its header). */
export function articlePreview(body: string, length = SUMMARY_LENGTH): ArticlePreview {
  let text = body.replace(/\r\n?/g, "\n")
  let kind: string | null = null
  if (text.startsWith("---\n")) {
    const end = text.indexOf("\n---", 4)
    if (end > 0) {
      const header = text.slice(4, end)
      kind = /^entity_kind:\s*(.+)$/m.exec(header)?.[1].trim().replace(/^["']|["']$/g, "") || null
      text = text.slice(end + 4)
    }
  }
  text = text
    .replace(/<!--[\s\S]*?-->/g, "")
    .replace(/```[\s\S]*?```/g, "")
    .split("\n")
    .filter((line) => !/^\[\^[^\]\s]+\]:/.test(line))
    .join("\n")
  let kindLabel: string | null = null
  let summary: string | null = null
  for (const block of text.split(/\n\s*\n/)) {
    const paragraph = block.trim()
    // Headings, lists, tables, quotes, markup and the app's own placeholders are not a summary.
    if (!paragraph || /^(#|[-*+] |\d+[.)] |\||>|<|\u0001)/.test(paragraph)) continue
    const label = KIND_LINE.exec(paragraph)
    if (label) {
      kindLabel ??= label[1].trim()
      continue
    }
    summary = plain(paragraph)
    if (summary) break
  }
  return { kind, kindLabel, summary: summary ? cut(summary, length) : null }
}

/** Markdown of a paragraph as plain text: links by their words, no marks. */
export function plain(markdown: string): string {
  return markdown
    .replace(/\[\^[^\]\s]+\]/g, "")
    .replace(/\[\[([^\]|\n]+)\|([^\]\n]*)\]\]/g, (_all, target: string, alias: string) => alias.trim() || target.trim())
    .replace(/\[\[([^\]\n]+)\]\]/g, "$1")
    .replace(/!?\[([^\]\n]*)\]\([^)\n]*\)/g, "$1")
    .replace(/(\*\*|__)(.+?)\1/g, "$2")
    .replace(/(^|[^\w*])[*_]([^*_\n]+)[*_](?=[^\w*]|$)/g, "$1$2")
    .replace(/`([^`\n]*)`/g, "$1")
    .replace(/\s+/g, " ")
    .trim()
}

function cut(text: string, length: number): string {
  if (text.length <= length) return text
  const head = text.slice(0, length)
  const space = head.lastIndexOf(" ")
  return `${(space > length * 0.6 ? head.slice(0, space) : head).replace(/[\s,;:.·—-]+$/, "")}…`
}

/** The first document a note names (`[[target|name]]`), to open it from its card. */
export function firstWikilink(markdown: string): { target: string; name: string } | null {
  const found = /\[\[([^\]|\n]+)(?:\|([^\]\n]*))?\]\]/.exec(markdown)
  if (!found) return null
  const target = found[1].trim()
  return { target, name: found[2]?.trim() || target }
}
