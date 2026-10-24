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


/**
 * A section of a detail screen, as in the mockup (D-493): its title in
 * Mulish with a line under it, how many things it has in grey after it,
 * and its action at the right. `id` is its anchor for «En esta página».
 */
export function Section({
  title,
  count,
  children,
  action,
  id,
  intro,
}: {
  title: ReactNode
  count?: number | null
  children: ReactNode
  action?: ReactNode
  id?: string
  /** One line of what the section is, in grey under its title. */
  intro?: ReactNode
}) {
  return (
    <section id={id} className="mt-10 scroll-mt-6 first:mt-0">
      <div className="mb-3.5 flex items-center gap-2.5 border-b pb-2">
        <h2 className="font-ui text-[1.3125rem] leading-tight font-bold text-strong">
          {title}
          {count !== undefined && count !== null && (
            <span className="ml-2.5 font-ui text-[0.9375rem] font-semibold text-muted-foreground tabular-nums">{count}</span>
          )}
        </h2>
        {action && <div className="ml-auto flex items-center gap-2">{action}</div>}
      </div>
      {intro && <p className="-mt-1 mb-3 text-sm text-muted-foreground">{intro}</p>}
      {children}
    </section>
  )
}

/** A quote of a document, as the record keeps it: «…», in the text's colour. */
export function Quote({ children }: { children: ReactNode }) {
  return <q className="text-foreground before:content-['«'] after:content-['»']">{children}</q>
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
