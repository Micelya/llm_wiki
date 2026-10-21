import { useState, type ReactNode } from "react"
import { useDocbaseStore } from "@/stores/docbase-store"
import { useKnowledgeStore } from "@/stores/knowledge-store"
import { useWikiStore } from "@/stores/wiki-store"

/** Opens a document of the base in its detail. */
export function useOpenDocument(): (locationId: number | null) => void {
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const openDocument = useDocbaseStore((s) => s.openDocument)
  return (locationId) => {
    if (locationId === null) return
    setActiveView("documents")
    openDocument(locationId)
  }
}

/** Opens an entity in the screen of people and organizations. */
export function useOpenEntity(): (id: string) => void {
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const openEntity = useKnowledgeStore((s) => s.openEntity)
  return (id) => {
    openEntity(id)
    setActiveView("entities")
  }
}

/** Opens a matter in the screen of matters. */
export function useOpenMatter(): (id: string) => void {
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const openMatter = useKnowledgeStore((s) => s.openMatter)
  return (id) => {
    openMatter(id)
    setActiveView("matters")
  }
}


/** A section of a detail screen, laid out like the document's. */
export function Section({ title, children, action }: { title: ReactNode; children: ReactNode; action?: ReactNode }) {
  return (
    <section className="mt-8 first:mt-0">
      <div className="flex items-end justify-between gap-3 border-b pb-1">
        <h2 className="font-serif text-xl">{title}</h2>
        {action}
      </div>
      <div className="mt-3">{children}</div>
    </section>
  )
}

/** A quote of a document, as the record keeps it. */
export function Quote({ children }: { children: ReactNode }) {
  return <q className="text-sm text-muted-foreground italic before:content-['«'] after:content-['»']">{children}</q>
}

/**
 * Runs an action of a screen, shows its error and tells when it is busy,
 * so two clicks do not decide twice.
 */
export function useAction(): {
  busy: boolean
  problem: string | null
  run: (action: () => Promise<unknown>) => Promise<void>
  clear: () => void
} {
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  return {
    busy,
    problem,
    clear: () => setProblem(null),
    run: async (action) => {
      setBusy(true)
      setProblem(null)
      try {
        await action()
      } catch (cause) {
        setProblem(typeof cause === "string" ? cause : String((cause as Error)?.message ?? cause))
      } finally {
        setBusy(false)
      }
    },
  }
}
