import { useState, type ReactNode } from "react"
import { ArrowLeft } from "lucide-react"
import { EmptyNote } from "@/components/docbase/page"
import { NotesEditor } from "./notes-editor"

/** The two tabs of a detail screen: what is read, and the data one by one with what a person can do. */
export type DetailTab = "article" | "data"

/**
 * Which tab a detail screen shows: the one the person chose; until she
 * chooses, the article, or the data when it is known there is no article
 * to read.
 */
export function useDetailTab(withoutArticle: boolean, initial: DetailTab | null = null): [DetailTab, (tab: DetailTab) => void] {
  const [tab, setTab] = useState<DetailTab | null>(initial)
  return [tab ?? (withoutArticle ? "data" : "article"), setTab]
}

/**
 * The screen of a document, of a person or organization and of a matter,
 * the three with the same form (D-389, D-423): a header with the way back,
 * the name and its actions, and two tabs, «Artículo» (the article the base
 * keeps, with its quotes) and «Datos y correcciones» (the data one by one,
 * with its quote and what a person can do about it). The notes of a person
 * go at the foot of both tabs, the same editor on the two: what she was
 * writing stays when she changes tab.
 */
export function DetailTabs({
  back,
  onBack,
  title,
  actions,
  tab,
  onTab,
  pending = 0,
  notice,
  article,
  data,
  notesOf,
}: {
  /** Where the way back leads: "Documentos", "Casos"… */
  back: string
  onBack: () => void
  title: ReactNode
  /** Buttons on the right of the name. */
  actions?: ReactNode
  tab: DetailTab
  onTab: (tab: DetailTab) => void
  /** What waits for a person's decision in «Datos y correcciones». */
  pending?: number
  /** Problems and notices, above both tabs. */
  notice?: ReactNode
  article: ReactNode
  data: ReactNode
  /** The object the notes of a person are of; without it, no notes. */
  notesOf: string | null
}) {
  return (
    <div className="flex h-full flex-col">
      <header className="shrink-0 border-b px-8 pt-4">
        <button className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground" onClick={onBack}>
          <ArrowLeft className="size-4" /> {back}
        </button>
        <div className="mt-2 flex items-end justify-between gap-4 pb-2">
          <div className="min-w-0 flex-1">{title}</div>
          {actions ? <div className="flex shrink-0 gap-2">{actions}</div> : null}
        </div>
        <nav role="tablist" className="-mb-px flex gap-1 text-sm">
          <TabButton active={tab === "article"} onClick={() => onTab("article")}>
            Artículo
          </TabButton>
          <TabButton active={tab === "data"} onClick={() => onTab("data")}>
            Datos y correcciones
            {pending > 0 && (
              <span className="ml-1.5 rounded-full bg-amber-500/20 px-1.5 text-xs text-amber-700 dark:text-amber-400">{pending}</span>
            )}
          </TabButton>
        </nav>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-8 py-6">
        <div role="tabpanel" data-tab={tab} className={`mx-auto ${tab === "article" ? "max-w-4xl" : "max-w-6xl"}`}>
          {notice}
          {tab === "article" ? <div key="article">{article}</div> : <div key="data">{data}</div>}
          {notesOf ? (
            <div className="mt-8">
              <NotesEditor key={notesOf} objectId={notesOf} />
            </div>
          ) : null}
        </div>
      </div>
    </div>
  )
}

function TabButton({ active, onClick, children }: { active: boolean; onClick: () => void; children: ReactNode }) {
  return (
    <button
      role="tab"
      aria-selected={active}
      className={`flex items-center border-b-2 px-3 py-1.5 ${
        active ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:text-foreground"
      }`}
      onClick={onClick}
    >
      {children}
    </button>
  )
}

/**
 * What the tab «Artículo» says when there is no article to read, and the
 * way to «Datos y correcciones», where what is known of it is.
 */
export function NoArticle({ children, onShowData }: { children: ReactNode; onShowData: () => void }) {
  return (
    <EmptyNote>
      {children}{" "}
      <button className="text-primary hover:underline" onClick={onShowData}>
        Ver «Datos y correcciones»
      </button>
    </EmptyNote>
  )
}
