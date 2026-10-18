/**
 * The last touch on every page the wiki model writes for a law firm,
 * before it goes to disk (D-237). Deterministic: it never asks the model.
 *
 * - The path of a page always ends in `.md` (a page without it is a file
 *   the reader and the index do not see).
 * - The title in the header is the page's first heading, never
 *   "Source: <path>".
 * - No fields of an academic reference (`authors`, `year`, `url`,
 *   `venue`): a document of the archive is not a paper.
 * - The page never speaks of the wiki itself nor of "the material"
 *   ("ya figura en el wiki", "no está incluida en el material"): a
 *   sentence that does is dropped.
 * - A [[link]] that points to no page of the wiki is left as its text.
 */

export const ACADEMIC_FIELDS = ["authors", "year", "url", "venue"] as const

/**
 * Sentences that speak of the wiki or of the material it was given rather
 * than of the documents. Kept conservative: a contract may well speak of
 * "defectos en el material".
 */
export const SELF_REFERENCE_PATTERNS: readonly RegExp[] = [
  /\bwikis?\b/i,
  /\b(?:en|del|al|el|este|dicho)\s+(?:el\s+)?material(?:es)?\s+(?:proporcionad|disponible|analizad|aportad|cargad|revisad|facilitad|incluid)/i,
  /\bno\s+(?:est[aá]n?|figuran?|constan?|aparecen?|se\s+encuentran?|se\s+incluyen?|fue(?:ron)?\s+incluid[oa]s?)\s+(?:incluid[oa]s?\s+)?en\s+(?:el|este|dicho)\s+material\b/i,
  /\b(?:the\s+)?(?:provided|available|supplied)\s+materials?\b/i,
  /\bmaterials?\s+(?:provided|available|supplied)\b/i,
]

const FRONTMATTER = /^(---\s*\r?\n)([\s\S]*?)(\r?\n---\s*(?:\r?\n|$))/

/** The path of a page, always with its `.md`. */
export function withMdExtension(path: string): string {
  return /\.md$/i.test(path) ? path : `${path.replace(/\.+$/, "")}.md`
}

function yamlQuoted(text: string): string {
  return JSON.stringify(text)
}

const SOURCE_PREFIX = /^\s*(?:source|fuente)\s*:\s*/i

/** What is left of a "Source: <path>" title: the file name, without its extension. */
function withoutSourcePrefix(title: string): string {
  if (!SOURCE_PREFIX.test(title)) return title.trim()
  const rest = title.replace(SOURCE_PREFIX, "").trim().replace(/^["']|["']$/g, "")
  const name = rest.replace(/\\/g, "/").split("/").pop() ?? rest
  return name.replace(/\.[A-Za-z0-9]{1,5}$/, "").replace(/\.[A-Za-z0-9]{1,5}$/, "").trim() || rest
}

/**
 * The header's title taken from the first heading of the page; a
 * "Source: …" title or heading becomes the document's name.
 */
export function titleFromHeading(content: string): string {
  const match = content.match(FRONTMATTER)
  const body = match ? content.slice(match[0].length) : content
  const headingMatch = body.match(/^#[ \t]+(.+?)[ \t]*$/m)
  let out = content
  let heading = headingMatch?.[1]
  if (headingMatch && heading !== undefined && SOURCE_PREFIX.test(heading)) {
    heading = withoutSourcePrefix(heading)
    const fixedBody = body.replace(headingMatch[0], `# ${heading}`)
    out = match ? `${match[0]}${fixedBody}` : fixedBody
  }
  const fm = out.match(FRONTMATTER)
  if (!fm) return out
  const lines = fm[2].split(/\r?\n/)
  const titleAt = lines.findIndex((line) => /^title\s*:/i.test(line))
  const current = titleAt >= 0 ? lines[titleAt].replace(/^title\s*:\s*/i, "").trim().replace(/^"(.*)"$|^'(.*)'$/, "$1$2") : ""
  const title = heading?.trim() || withoutSourcePrefix(current)
  if (!title) return out
  const line = `title: ${yamlQuoted(title)}`
  if (titleAt >= 0) {
    if (current === title && !SOURCE_PREFIX.test(current)) return out
    lines[titleAt] = line
  } else {
    lines.push(line)
  }
  return `${fm[1]}${lines.join("\n")}${fm[3]}${out.slice(fm[0].length)}`
}

/** The header without the fields of an academic reference. */
export function withoutAcademicFields(content: string): string {
  const fm = content.match(FRONTMATTER)
  if (!fm) return content
  const academic = new RegExp(`^(?:${ACADEMIC_FIELDS.join("|")})\\s*:`, "i")
  const lines = fm[2].split(/\r?\n/)
  const kept: string[] = []
  let skipping = false
  for (const line of lines) {
    if (academic.test(line)) {
      skipping = true
      continue
    }
    // The items of a block list under a dropped field go with it.
    if (skipping && /^\s+(?:-|\S)/.test(line) && !/^\S/.test(line)) continue
    skipping = false
    kept.push(line)
  }
  if (kept.length === lines.length) return content
  return `${fm[1]}${kept.join("\n")}${fm[3]}${content.slice(fm[0].length)}`
}

/**
 * Words after which a period does not end a sentence ("N.º", "art.", "Dr.").
 * A legal form ("S.A.", "Ltd.") often ends one, so it is not here.
 */
const ABBREVIATIONS = new Set([
  "nº", "n.º", "no", "nro", "núm", "num", "art", "arts", "inc", "cfr", "conf", "etc", "pág", "pag", "p", "pp",
  "sr", "sra", "srta", "dr", "dra", "lic", "ing", "cr", "cra", "esc", "av", "avda", "ee.uu", "uu", "vs", "aprox",
])

/** A text in sentences: a period ends one only before a capital or an opening sign, and not after an abbreviation or an initial. */
function splitSentences(text: string): string[] {
  const out: string[] = []
  let start = 0
  const re = /[.!?…]+["»”)]?\s+(?=[A-ZÁÉÍÓÚÑÜ¿¡«"“(\[*])/g
  let match: RegExpExecArray | null
  while ((match = re.exec(text)) !== null) {
    const before = text.slice(start, match.index + 1)
    const word = (before.match(/(\S+)\.$/)?.[1] ?? "").replace(/^[(«"“]+/, "")
    if (match[0].startsWith(".") && (ABBREVIATIONS.has(word.toLowerCase()) || /^\p{Lu}$/u.test(word))) continue
    const end = match.index + match[0].length
    out.push(text.slice(start, end).trimEnd())
    start = end
  }
  if (start < text.length) out.push(text.slice(start))
  return out
}

/** A tail that only says where the wiki keeps something. */
const SELF_CLAUSE =
  /,?\s+(?:que\s+)?(?:ya\s+)?(?:(?:est[aá]n?|figuran?|constan?)\s+)?(?:registrad|incluid|cargad|presentes?|existentes?|guardad|recogid)?[oa]?s?\s*en\s+(?:el|este)\s+wiki\b/i

function withoutSelfClause(sentence: string): string {
  const match = sentence.match(SELF_CLAUSE)
  if (!match || match.index === undefined) return sentence
  const before = sentence.slice(0, match.index).trimEnd()
  const after = sentence.slice(match.index + match[0].length)
  // Only a tail: what follows is the end of the sentence or a new clause.
  if (!/^\s*(?:[.;:]|,\s|$)/.test(after) || before.split(/\s+/).length < 4) return sentence
  const rest = after.replace(/^\s*,?\s*/, after.trim().startsWith(",") ? ", " : "")
  return `${before}${rest.startsWith(",") || /^[.;:]/.test(rest) ? rest : rest ? ` ${rest}` : "."}`.replace(/\s+([.;:])/, "$1")
}

function speaksOfItself(sentence: string): boolean {
  // A link target or an image path is not prose.
  const prose = sentence.replace(/\[\[[^\]]*\]\]/g, " ").replace(/\]\([^)]*\)/g, "]")
  return SELF_REFERENCE_PATTERNS.some((pattern) => pattern.test(prose))
}

/**
 * The body without the sentences that speak of the wiki or of its
 * material. Headings, tables and code are left as they are. A paragraph
 * written over several lines is one unit (a sentence may go on in the next
 * line); a list item is one unit. A stray `---FILE: …---` line (what is
 * left of an answer the model nested) goes too.
 */
export function withoutSelfReference(content: string): { content: string; removed: string[] } {
  const fm = content.match(FRONTMATTER)
  const head = fm ? fm[0] : ""
  const body = fm ? content.slice(head.length) : content
  const removed: string[] = []
  const out: string[] = []
  let paragraph: string[] = []
  let marker = ""

  const flush = () => {
    if (paragraph.length === 0) return
    const text = paragraph.map((l, i) => (i === 0 ? l.slice(marker.length) : l.trim())).join(" ")
    const sentences = splitSentences(text)
    let changed = false
    const kept: string[] = []
    for (const sentence of sentences) {
      if (!speaksOfItself(sentence)) {
        kept.push(sentence)
        continue
      }
      changed = true
      removed.push(sentence.trim())
      // "…las facturas ya registradas en el wiki." keeps what it says of
      // the documents, without its tail.
      const trimmed = withoutSelfClause(sentence)
      if (trimmed !== sentence && !speaksOfItself(trimmed)) kept.push(trimmed)
    }
    if (!changed) out.push(...paragraph)
    else if (kept.length > 0) out.push(`${marker}${kept.join(" ")}`)
    paragraph = []
    marker = ""
  }

  let inCode = false
  for (const line of body.split("\n")) {
    if (/^\s*(?:```|~~~)/.test(line)) {
      flush()
      inCode = !inCode
      out.push(line)
      continue
    }
    if (inCode) {
      out.push(line)
      continue
    }
    if (/^\s*---\s*(?:END\s+)?FILE\b/i.test(line)) {
      flush()
      removed.push(line.trim())
      continue
    }
    if (/^\s*(?:#|\||<!--)/.test(line) || !line.trim()) {
      flush()
      out.push(line)
      continue
    }
    const itemMarker = line.match(/^(\s*(?:[-*+]|\d+[.)])\s+|\s*>\s*)/)?.[0]
    if (itemMarker !== undefined) {
      flush()
      marker = itemMarker
    }
    paragraph.push(line)
  }
  flush()
  return { content: head + out.join("\n"), removed }
}

function normalizeTarget(target: string): string {
  return target
    .replace(/\\/g, "/")
    .split("#")[0]
    .replace(/^\/+/, "")
    .replace(/^wiki\//i, "")
    .replace(/\.md$/i, "")
    .trim()
    .toLowerCase()
}

/**
 * The keys a link can name a page by: its path inside `wiki/` without
 * `.md`, and its file name (what the structural review accepts).
 */
export function wikiPageKeys(paths: Iterable<string>): Set<string> {
  const keys = new Set<string>()
  for (const path of paths) {
    const key = normalizeTarget(path)
    if (!key) continue
    keys.add(key)
    keys.add(key.split("/").pop() ?? key)
  }
  return keys
}

export function linkResolves(keys: Set<string>, target: string): boolean {
  const key = normalizeTarget(target)
  if (!key) return false
  return keys.has(key) || keys.has(key.split("/").pop() ?? key)
}

/** The body with every [[link]] to no page left as its text. */
export function withValidLinks(content: string, keys: Set<string>): { content: string; unlinked: string[] } {
  const unlinked: string[] = []
  const out = content.replace(/(!?)\[\[([^\]|]+?)(?:\|([^\]]*))?\]\]/g, (whole, bang: string, target: string, label?: string) => {
    if (bang) return whole
    if (linkResolves(keys, target)) return whole
    unlinked.push(target.trim())
    const text = label?.trim() || target.trim().split("#")[0].split("/").pop() || target.trim()
    return text
  })
  return { content: out, unlinked }
}

export interface LegalFinish {
  content: string
  warnings: string[]
}

/** Everything above, for one page the wiki model wrote. */
export function finishLegalPage(path: string, content: string, keys: Set<string>): LegalFinish {
  const warnings: string[] = []
  let out = titleFromHeading(content)
  out = withoutAcademicFields(out)
  const self = withoutSelfReference(out)
  out = self.content
  if (self.removed.length > 0) {
    warnings.push(`${path}: se quitaron ${self.removed.length} oraciones que hablaban del propio wiki o de su material.`)
  }
  const links = withValidLinks(out, keys)
  out = links.content
  if (links.unlinked.length > 0) {
    warnings.push(`${path}: ${links.unlinked.length} enlaces sin página quedaron como texto (${links.unlinked.slice(0, 5).join(", ")}).`)
  }
  return { content: out, warnings }
}
