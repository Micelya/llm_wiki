import { Fragment, type ReactNode } from "react"
import { cn } from "@/lib/utils"

/** Where a long name may break besides its spaces: after an underscore or a slash. */
const BREAK_AFTER = "_/\\"

/** The pieces of a name, each ending in an underscore or a slash but the last. */
export function namePieces(text: string): string[] {
  const pieces: string[] = []
  let start = 0
  for (let index = 0; index < text.length; index++) {
    if (BREAK_AFTER.includes(text[index])) {
      pieces.push(text.slice(start, index + 1))
      start = index + 1
    }
  }
  if (start < text.length) pieces.push(text.slice(start))
  return pieces
}

/**
 * The pieces of a name with a place to break after each underscore and
 * slash («Escrito_de_inicio_2024-08-19.pdf», «Clientes/Ejemplar/Despido»).
 * A hyphen is already a place to break.
 */
export function breakable(text: string): ReactNode {
  const pieces = namePieces(text)
  if (pieces.length <= 1) return text
  return pieces.map((piece, index) => (
    <Fragment key={index}>
      {index > 0 && <wbr />}
      {piece}
    </Fragment>
  ))
}

/**
 * A name that can be one long word, as many file names are: it breaks
 * after its underscores, slashes and hyphens, and anywhere when a piece is
 * still wider than its place, so it never pushes its column or its card
 * out of the window. In a table its column can be as narrow as the
 * others need.
 */
export function LongName({ children, className, title }: { children: string; className?: string; title?: string }) {
  return (
    <span className={cn("wrap-anywhere", className)} title={title}>
      {breakable(children)}
    </span>
  )
}
