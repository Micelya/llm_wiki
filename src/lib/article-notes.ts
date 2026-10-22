/**
 * The numbered notes of an article (D-441). The app writes each quote a
 * text rests on as a note, the way markdown writes notes: `[^1]` where the
 * text says it, and `[^1]: «the quote» ([[document|its name]])` once, at
 * the end of the page. The article shows the number, and the note when it
 * is touched, instead of a list of references at its foot.
 */

/** Where a note's mark points, in the markdown the reader renders. */
export const NOTE_HREF = "#micelya-nota-"

export interface ArticleNotes {
  /** The text without the notes, each mark as a link to its note. */
  body: string
  /** Each note by its label ("1"), as markdown. */
  notes: Record<string, string>
}

const DEFINITION = /^\[\^([^\]\s]+)\]:[ \t]?(.*)$/

/**
 * The notes of a page out of its text: the lines `[^n]: …` go (with the
 * blank lines they leave), and each mark `[^n]` of a note there is becomes
 * a link the reader shows as the note's number. A mark without its note
 * stays as it is written; code is left untouched.
 */
export function splitNotes(text: string): ArticleNotes {
  const notes: Record<string, string> = {}
  const kept: string[] = []
  let fenced = false
  for (const line of text.split("\n")) {
    if (line.trimStart().startsWith("```")) fenced = !fenced
    const found = fenced ? null : DEFINITION.exec(line.replace(/\r$/, ""))
    if (found) {
      notes[found[1]] = found[2].trim()
      continue
    }
    kept.push(line)
  }
  if (Object.keys(notes).length === 0) return { body: text, notes }
  const body = kept
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .replace(/\n+$/, "\n")
    .split(/(```[\s\S]*?```|`[^`\n]+`)/g)
    .map((part, index) =>
      index % 2 === 1
        ? part
        : part.replace(/\[\^([^\]\s]+)\]/g, (mark, label: string) =>
            label in notes ? `[${label}](${NOTE_HREF}${encodeURIComponent(label)})` : mark,
          ),
    )
    .join("")
  return { body, notes }
}

/** The label of a note a link points to, or null for any other link. */
export function noteLabel(href: string): string | null {
  if (!href.startsWith(NOTE_HREF)) return null
  try {
    return decodeURIComponent(href.slice(NOTE_HREF.length))
  } catch {
    return href.slice(NOTE_HREF.length)
  }
}
