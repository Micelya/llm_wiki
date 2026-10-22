import {
  getDocumentArticle,
  getKnowledgeArticle,
  getKnowledgeOverview,
  objectOfPage,
  onDocbaseChanged,
  type KnowledgeArticle,
  type PageObject,
} from "@/commands/docbase"
import { articlePreview } from "@/lib/article-preview"
import { entityTypeName, vocabulary } from "@/vocabulary"

/** What the floating card of a link shows (P-246). */
export interface LinkPreview {
  /** What the link names, to open it from the card. */
  object: PageObject
  title: string
  /** Its type, in the words of the profession: "Persona", "Caso", "Documento". */
  typeLabel: string
  /** Its short summary, from its article; null when it has none. */
  summary: string | null
}

const asked = new Map<string, Promise<LinkPreview | null>>()
const known = new Map<string, LinkPreview | null>()
let listening = false

/** Forgets what the cards showed: the base changed, the articles may too. */
export function forgetLinkPreviews(): void {
  asked.clear()
  known.clear()
}

/** What a card already has for a link, without asking the base; undefined when not yet. */
export function knownLinkPreview(reference: string): LinkPreview | null | undefined {
  return known.get(reference)
}

/**
 * What the card of a link shows, read from the article the app wrote in
 * the base (nothing is asked to a model), once per link until the base
 * changes; null when the link names nothing the base knows.
 */
export function loadLinkPreview(reference: string): Promise<LinkPreview | null> {
  listen()
  let found = asked.get(reference)
  if (!found) {
    found = previewOf(reference).then(
      (preview) => {
        if (asked.get(reference) === found) known.set(reference, preview)
        return preview
      },
      (cause: unknown) => {
        asked.delete(reference)
        throw cause
      },
    )
    asked.set(reference, found)
  }
  return found
}

function listen(): void {
  if (listening) return
  listening = true
  try {
    onDocbaseChanged(forgetLinkPreviews).catch(() => {
      listening = false
    })
  } catch {
    listening = false
  }
}

async function previewOf(reference: string): Promise<LinkPreview | null> {
  const object = await objectOfPage(reference)
  if (!object) return null
  if (object.kind === "overview") {
    const overview = await getKnowledgeOverview()
    const shown = overview ? articlePreview(overview.body) : null
    return { object, title: "Inicio", typeLabel: vocabulary().archive.name, summary: shown?.summary ?? null }
  }
  let article: KnowledgeArticle | null = await getKnowledgeArticle(object.objectId)
  if (!article && object.kind === "document" && object.contentId !== null) article = await getDocumentArticle(object.contentId)
  return previewOfArticle(object, article)
}

/** The card of what a link names, from its article (or only its kind, without one). */
export function previewOfArticle(object: PageObject, article: KnowledgeArticle | null): LinkPreview {
  const shown = article ? articlePreview(article.body) : { kind: null, kindLabel: null, summary: null }
  return {
    object,
    title: article?.title ?? titleOfPath(object.path),
    typeLabel: typeLabelOf(object, shown.kindLabel, shown.kind),
    summary: shown.summary,
  }
}

function typeLabelOf(object: PageObject, kindLabel: string | null, kind: string | null): string {
  switch (object.kind) {
    case "matter":
      return vocabulary().knowledge.matter
    case "document":
      return "Documento"
    default:
      return kindLabel ?? (kind ? entityTypeName(kind) : "Ficha")
  }
}

function titleOfPath(path: string): string {
  const name = path.split("/").pop() ?? path
  return name.replace(/\.md$/i, "").replace(/-/g, " ")
}
