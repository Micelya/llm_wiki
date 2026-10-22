import { create } from "zustand"

/**
 * Navigation state of the section «Plantillas»: which template is open,
 * by the location of its principal document, and whether a document is
 * being generated from it. In a store so a document's template can send
 * the user straight to it.
 */
interface TemplatesState {
  /** The location of the principal of the template whose detail is open. */
  principal: number | null
  /** A document is being generated from that template (D-488). */
  generating: boolean
  openTemplate: (principal: number | null) => void
  /** Opens the screen to generate a document from the template of this principal. */
  generateFrom: (principal: number) => void
}

export const useTemplatesStore = create<TemplatesState>((set) => ({
  principal: null,
  generating: false,
  openTemplate: (principal) => set({ principal, generating: false }),
  generateFrom: (principal) => set({ principal, generating: true }),
}))
