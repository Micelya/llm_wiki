import { create } from "zustand"

/**
 * Navigation state of the section «Plantillas»: which template is open,
 * by the location of its principal document. In a store so a document's
 * template can send the user straight to it.
 */
interface TemplatesState {
  /** The location of the principal of the template whose detail is open. */
  principal: number | null
  openTemplate: (principal: number | null) => void
}

export const useTemplatesStore = create<TemplatesState>((set) => ({
  principal: null,
  openTemplate: (principal) => set({ principal }),
}))
