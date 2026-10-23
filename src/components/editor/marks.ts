import { createContext, type ReactNode } from "react"
import type { CertaintyWords } from "@/components/knowledge/article-parts"

/**
 * The words of how sure a page is of a datum, for the notes of an article:
 * the reader that shows the article gives them to the cards of its notes,
 * which show «sin confirmar» as the same amber mark.
 */
export const MarksContext = createContext<CertaintyWords | undefined>(undefined)

/** The text of what an emphasis holds, when it is only text. */
export function plainText(children: ReactNode): string | null {
  if (typeof children === "string") return children
  if (Array.isArray(children) && children.every((child) => typeof child === "string")) return children.join("")
  return null
}
