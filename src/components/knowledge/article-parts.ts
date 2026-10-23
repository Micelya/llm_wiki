/**
 * The parts of an article, to lay it out as a page of an encyclopedia
 * (P-057, D-493): the app writes each page in the base as markdown with a
 * known form (`src-tauri/src/docbase/knowledge/pages.rs`), and the screen
 * reads that form back to show its documents and its people in tables, its
 * conflicts in a notice, its links in cards. Only the screen changes: the
 * text of the page stays as the app wrote it (the chat and the search read
 * it whole). Whatever does not have the expected form is shown as it is
 * written, as markdown: nothing goes missing.
 */

/** The words the pages are written with, by id (`usePageWords`). */
export type PageWord = (id: string) => string

/** What a section of a page is, by its heading. */
export type SectionKind = "says" | "documents" | "proposed" | "relations" | "related" | "parties" | "facts" | "other"

export interface ArticleSection {
  kind: SectionKind
  /** Its heading as the page writes it, without the count. */
  heading: string
  /** The count the heading carries: «Documentos (11)». */
  count: number | null
  /** Its markdown, without its heading. */
  body: string
  /** The anchor of its heading on the screen, unique in the page. */
  id: string
}

export interface ArticleParts {
  /** What goes before the first section: what it is, other names, a group. */
  lead: string
  sections: ArticleSection[]
}

/** The sections with a form of their own, by the id of their heading's word. */
const KINDS: [SectionKind, string][] = [
  ["says", "says"],
  ["documents", "documents"],
  ["relations", "relations"],
  ["related", "related"],
  ["parties", "matterParties"],
  ["facts", "matterFacts"],
]

function headingOf(line: string): { heading: string; count: number | null } | null {
  const found = /^## (.+?)\s*$/.exec(line)
  if (!found) return null
  const counted = /^(.*?) \((\d+)\)$/.exec(found[1])
  return counted ? { heading: counted[1], count: Number(counted[2]) } : { heading: found[1], count: null }
}

/** The paragraphs of a text: its blocks between blank lines. */
export function paragraphs(text: string): string[] {
  return text
    .split(/\n\s*\n/)
    .map((block) => block.trim())
    .filter(Boolean)
}

/**
 * An article in its parts: what goes before its first section, and each
 * section with what it is. The documents a model proposes for a matter
 * (`### Posiblemente de este asunto`, inside «Documentos») become a section
 * of their own, after it; the line of why its documents go together stays
 * with the documents.
 */
export function parseArticle(body: string, word: PageWord): ArticleParts {
  const lines = body.replace(/\r\n/g, "\n").split("\n")
  const lead: string[] = []
  const found: { heading: string; count: number | null; lines: string[] }[] = []
  for (const line of lines) {
    const heading = headingOf(line)
    if (heading) {
      found.push({ ...heading, lines: [] })
    } else if (found.length === 0) {
      lead.push(line)
    } else {
      found[found.length - 1].lines.push(line)
    }
  }
  const sections: ArticleSection[] = []
  const add = (kind: SectionKind, heading: string, count: number | null, text: string) => {
    sections.push({ kind, heading, count, body: text.trim(), id: `seccion-${sections.length + 1}` })
  }
  for (const section of found) {
    const kind = KINDS.find(([, id]) => word(id) === section.heading)?.[0] ?? "other"
    if (kind !== "documents") {
      add(kind, section.heading, section.count, section.lines.join("\n"))
      continue
    }
    const proposedHeading = `### ${word("matterProposed")}`
    const start = section.lines.findIndex((line) => line.trim() === proposedHeading)
    if (start < 0) {
      add(kind, section.heading, section.count, section.lines.join("\n"))
      continue
    }
    const after = section.lines.slice(start + 1)
    const next = after.findIndex((line) => line.startsWith("### "))
    const block = next < 0 ? after : after.slice(0, next)
    const rest = next < 0 ? [] : after.slice(next)
    const why = `*${word("matterWhy")}*`
    const proposed = paragraphs(block.join("\n")).filter((p) => !p.startsWith(why))
    const kept = paragraphs(block.join("\n")).filter((p) => p.startsWith(why))
    add(kind, section.heading, section.count, [...section.lines.slice(0, start), ...rest, "", ...kept].join("\n"))
    add("proposed", word("matterProposed"), null, proposed.join("\n\n"))
  }
  return { lead: lead.join("\n").trim(), sections }
}

/**
 * The first line of a page says what it is (`*Persona* · …`): what follows
 * the type, for the line under the name («Se lo nombra como … · Expediente
 * …»), and the paragraphs after it (the companies of a group).
 */
export function leadParts(lead: string): { facts: string; rest: string[] } {
  const [first = "", ...rest] = paragraphs(lead)
  if (!/^\*[^*\n]+\*/.test(first)) return { facts: "", rest: first ? [first, ...rest] : rest }
  return { facts: first.replace(/^\*[^*\n]+\*(?: · )?/, "").trim(), rest }
}

// --- What the documents say ---------------------------------------------

export interface SaysParts {
  /** The sentences of the synthesis (or what says it is not written yet). */
  lede: string[]
  /** How to read the quotes, in words. */
  legend: string | null
  /** The data in which the documents differ: one line each («Domicilio: «A»[1] · «B»[2]»). */
  conflicts: string[]
  /** How many data more there are, and where. */
  more: string | null
  /** Anything else, as written. */
  rest: string[]
}

export function saysParts(body: string, word: PageWord): SaysParts {
  const parts: SaysParts = { lede: [], legend: null, conflicts: [], more: null, rest: [] }
  const blocks = paragraphs(body)
  const more = new RegExp(`^${escape(word("saysMore")).replace("\\{n\\}", "\\d+")}$`)
  for (let index = 0; index < blocks.length; index++) {
    const block = blocks[index]
    if (block === `*${word("certainLegend")}*`) {
      parts.legend = word("certainLegend")
    } else if (block === `**${word("conflicts")}**`) {
      const list = blocks[index + 1]
      if (list && list.split("\n").every((line) => line.startsWith("- "))) {
        parts.conflicts = list.split("\n").map((line) => line.slice(2).trim())
        index += 1
      } else {
        parts.rest.push(block)
      }
    } else if (block === word("saysMoreOne") || more.test(block)) {
      parts.more = block
    } else if (parts.legend === null && parts.conflicts.length === 0 && parts.more === null) {
      parts.lede.push(block)
    } else {
      parts.rest.push(block)
    }
  }
  return parts
}

function escape(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}

// --- Documents -------------------------------------------------------------

export interface DocumentRow {
  /** The document, as markdown: its link, or its name. */
  document: string
  /** Its roles in it, as the page says them; null on the page of a matter. */
  roles: string | null
  /** Its type, in words. */
  type: string | null
  /** In the matter by a proposal of the model, not yet confirmed. */
  proposed: boolean
}

export interface DocumentGroup {
  /** The matter, as markdown; null for the documents without a heading. */
  heading: string | null
  rows: DocumentRow[]
}

/**
 * The documents of a page by matter: `### [[asunto|Name]]` and below it
 * `- [[doc|Name]] — parte, firmante · carta documento` on the page of a
 * person (`withRoles`), `- [[doc|Name]] · carta documento` on the page of a
 * matter. What is not a list of documents goes in `extra`, as written.
 * Null when a line of the list has another form: the screen shows the
 * section as it is written.
 */
export function documentGroups(body: string, word: PageWord, withRoles: boolean): { groups: DocumentGroup[]; extra: string[] } | null {
  const groups: DocumentGroup[] = []
  const extra: string[] = []
  const proposed = ` (${word("proposed")})`
  for (const block of paragraphs(body)) {
    const lines = block.split("\n")
    if (lines.length === 1 && lines[0].startsWith("### ")) {
      groups.push({ heading: lines[0].slice(4).trim(), rows: [] })
      continue
    }
    if (!lines.every((line) => line.startsWith("- "))) {
      extra.push(block)
      continue
    }
    if (groups.length === 0) groups.push({ heading: null, rows: [] })
    for (const line of lines) {
      let text = line.slice(2).trim()
      const isProposed = text.endsWith(proposed)
      if (isProposed) text = text.slice(0, -proposed.length).trim()
      let row: DocumentRow | null
      if (withRoles) {
        const at = text.lastIndexOf(" — ")
        if (at < 0) return null
        const [roles, ...type] = text.slice(at + 3).split(" · ")
        row = { document: text.slice(0, at), roles: roles.trim(), type: type.join(" · ").trim() || null, proposed: isProposed }
      } else {
        const at = text.lastIndexOf(" · ")
        const type = at < 0 ? null : text.slice(at + 3)
        row =
          type !== null && !type.includes("]]")
            ? { document: text.slice(0, at), roles: null, type: type.trim(), proposed: isProposed }
            : { document: text, roles: null, type: null, proposed: isProposed }
      }
      groups[groups.length - 1].rows.push(row)
    }
  }
  return { groups: groups.filter((group) => group.rows.length > 0 || group.heading !== null), extra }
}

/** A document a model proposes for a matter: `- [[doc|Name]] — why`. */
export function proposedRows(body: string): { document: string; reason: string | null }[] | null {
  const rows: { document: string; reason: string | null }[] = []
  for (const block of paragraphs(body)) {
    // The note under the heading («sin confirmar: …») is said by the screen.
    if (/^\*[^\n]+\*$/.test(block)) continue
    for (const line of block.split("\n")) {
      if (!line.startsWith("- ")) return null
      const text = line.slice(2).trim()
      const at = text.indexOf(" — ")
      rows.push(at < 0 ? { document: text, reason: null } : { document: text.slice(0, at), reason: text.slice(at + 3) })
    }
  }
  return rows
}

// --- Links -----------------------------------------------------------------

export interface RelationRow {
  /** What the link is: «Empleado de». */
  label: string
  /** Whom to, as markdown, with its notes and how sure it is. */
  value: string
}

/**
 * The links of a person with others: `- Empleado de [[x|Name]][^1]`, or
 * with how sure it is, `- Cónyuge de *(sin confirmar)* [[x|Name]][^2]`;
 * the mark goes after the name, where the screen shows it. A line without
 * a link to a page has `label` empty: the screen shows it whole.
 */
export function relationRows(body: string): RelationRow[] | null {
  const rows: RelationRow[] = []
  for (const block of paragraphs(body)) {
    for (const line of block.split("\n")) {
      if (!line.startsWith("- ")) return null
      const text = line.slice(2).trim()
      const at = text.indexOf("[[")
      if (at <= 0) {
        rows.push({ label: "", value: text })
        continue
      }
      let label = text.slice(0, at).trim()
      let value = text.slice(at)
      const mark = /\s*(\*\([^)]*\)\*)$/.exec(label)
      if (mark) {
        label = label.slice(0, mark.index).trim()
        const end = value.indexOf("]]")
        value = end < 0 ? `${value} ${mark[1]}` : `${value.slice(0, end + 2)} ${mark[1]}${value.slice(end + 2)}`
      }
      rows.push({ label, value })
    }
  }
  return rows
}

export interface RelatedCard {
  /** Its link, as markdown. */
  link: string
  name: string
  /** Its type, in words: «Persona». */
  type: string
  /** How many documents it shares, in words: «9 documentos en común». */
  shared: string | null
}

/** The entities related to one: `- [[x|Name]] · Organización — 9 documentos en común`. */
export function relatedCards(body: string): RelatedCard[] | null {
  const cards: RelatedCard[] = []
  for (const block of paragraphs(body)) {
    for (const line of block.split("\n")) {
      const found = /^- (\[\[[^|\]]+\|([^\]]+)\]\]) · (.+?)(?: — (.+))?$/.exec(line.trim())
      if (!found) return null
      cards.push({ link: found[1], name: found[2].trim(), type: found[3].trim(), shared: found[4]?.trim() ?? null })
    }
  }
  return cards
}

/**
 * The initials of a name, for its card: the first and the last word that
 * start with a capital, leaving out abbreviations like «S.A.»: «Talleres
 * Inventados S.A.» → «TI», «Luis Alberto Ficticio» → «LF».
 */
export function initials(name: string): string {
  const words = name
    .replace(/[«»"()]/g, " ")
    .split(/\s+/)
    .filter((word) => /^\p{Lu}/u.test(word) && !/^(\p{L}\.)+\p{L}?\.?$/u.test(word))
  const chosen = words.length > 1 ? [words[0], words[words.length - 1]] : words
  return chosen.map((word) => word[0]).join("") || name.trim().slice(0, 1).toLocaleUpperCase("es")
}

// --- The page of a matter ----------------------------------------------------

export interface PartyRow {
  /** Who, as markdown. */
  who: string
  type: string
  roles: string
  documents: number
}

/** The parties of a matter: `- [[x|Name]] · Persona — parte, remitente (9 documentos)`. */
export function partyRows(body: string): PartyRow[] | null {
  const rows: PartyRow[] = []
  for (const block of paragraphs(body)) {
    for (const line of block.split("\n")) {
      const found = /^- (.+) · ([^·—]+?) — (.*) \((\d+) [^()]+\)$/.exec(line.trim())
      if (!found) return null
      rows.push({ who: found[1].trim(), type: found[2].trim(), roles: found[3].trim(), documents: Number(found[4]) })
    }
  }
  return rows
}

export interface FactGroup {
  /** Of whom, as markdown. */
  subject: string
  rows: { label: string; value: string }[]
}

/** What the documents of a matter say, by whom: `- [[x|Name]] — Cargo: técnica[^3]`. */
export function factGroups(body: string): FactGroup[] | null {
  const groups: FactGroup[] = []
  for (const block of paragraphs(body)) {
    for (const line of block.split("\n")) {
      const found = /^- (.+?) — ([^:\n]+?): (.+)$/.exec(line.trim())
      if (!found) return null
      const [, subject, label, value] = found
      const last = groups[groups.length - 1]
      if (last && last.subject === subject) last.rows.push({ label, value })
      else groups.push({ subject, rows: [{ label, value }] })
    }
  }
  return groups
}

// --- How sure ----------------------------------------------------------------

export interface CertaintyWords {
  doubtful: string
  confirmed: string
  corrected: string
}

export function certaintyWords(word: PageWord): CertaintyWords {
  return { doubtful: word("certainDoubtful"), confirmed: word("certainConfirmed"), corrected: word("certainCorrected") }
}

/**
 * What a mark of how sure a page is of a datum says (`*(sin confirmar: no
 * es seguro de quién se habla)*`, as the text inside the emphasis): doubtful
 * («sin confirmar», amber) or a person's word; null for any other emphasis.
 */
export function certaintyMark(said: string, words: CertaintyWords): { tone: "doubt" | "person"; label: string; why: string | null } | null {
  const found = /^\((.+?)(?::\s*(.+))?\)$/.exec(said.trim())
  if (!found) return null
  const [, label, why = null] = found
  if (label === words.doubtful) return { tone: "doubt", label, why }
  if (label === words.confirmed || label === words.corrected) return { tone: "person", label, why }
  return null
}
