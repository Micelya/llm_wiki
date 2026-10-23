import { useState, type ReactNode } from "react"
import { ChevronRight } from "lucide-react"
import { EmptyNote } from "@/components/docbase/page"
import { Tab, TabList } from "@/components/ui/tabs"
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

/** The name of the page, large, in Mulish (D-493). */
export function DetailTitle({ children }: { children: ReactNode }) {
  return <h1 className="min-w-0 font-ui text-[1.875rem] leading-[1.15] font-bold tracking-[-0.01em] break-words text-strong">{children}</h1>
}

/**
 * The screen of a document, of a person or organization and of a matter,
 * the three with the same form (D-389, D-423), laid out as the approved
 * mockup (D-493): on top where you are («Casos › nombre») and what it is
 * («Caso»), the name large with its badges, a line of facts under it, its
 * actions at the right; two tabs, «Artículo» (the article the base keeps,
 * with its quotes) and «Datos y correcciones» (the data one by one, with
 * its quote and what a person can do about it), each with its column at
 * the right (the fact box, the index of the page). The notes of a person
 * go at the foot of both tabs, the same editor on the two: what she was
 * writing stays when she changes tab.
 */
export function DetailTabs({
  back,
  onBack,
  current,
  kicker,
  title,
  facts,
  actions,
  tab,
  onTab,
  pending = 0,
  notice,
  article,
  data,
  articleAside,
  dataAside,
  notesOf,
}: {
  /** Where the way back leads: "Documentos", "Casos"… */
  back: string
  onBack: () => void
  /** The name of what is open, after the way back. */
  current: string
  /** What it is, small and in capitals over its name: «Persona», «Caso». */
  kicker?: string
  /** Its name (a `DetailTitle`), and its badges. */
  title: ReactNode
  /** A line under the name: other ways it is written, its file number. */
  facts?: ReactNode
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
  /** The column at the right of each tab. */
  articleAside?: ReactNode
  dataAside?: ReactNode
  /** The object the notes of a person are of; without it, no notes. */
  notesOf: string | null
}) {
  const aside = tab === "article" ? articleAside : dataAside
  return (
    <div className="flex h-full flex-col">
      <header className="shrink-0 border-b bg-background px-10 pt-[18px]">
        <nav aria-label="Dónde estás" className="flex min-w-0 items-center gap-1.5 font-ui text-[0.8125rem] font-semibold text-muted-foreground">
          <button className="shrink-0 hover:text-link" onClick={onBack}>
            {back}
          </button>
          <ChevronRight className="size-3.5 shrink-0" aria-hidden />
          <span className="truncate" aria-current="page">
            {current}
          </span>
        </nav>
        <div className="mt-2.5 flex items-end justify-between gap-6">
          <div className="min-w-0 flex-1">
            {kicker && (
              <p className="mb-1 font-ui text-xs font-bold tracking-[0.08em] text-[var(--m-purpura-medio)] uppercase dark:text-muted-foreground">
                {kicker}
              </p>
            )}
            <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1.5">{title}</div>
            {facts && <div className="mt-1.5 text-sm text-muted-foreground [&_b]:font-semibold [&_b]:text-foreground">{facts}</div>}
          </div>
          {actions ? <div className="flex shrink-0 flex-wrap justify-end gap-2 pb-1">{actions}</div> : null}
        </div>
        <TabList className="mt-4 border-b-0">
          <Tab active={tab === "article"} onClick={() => onTab("article")}>
            Artículo
          </Tab>
          <Tab active={tab === "data"} onClick={() => onTab("data")} count={pending > 0 ? pending : undefined} countTone="warn">
            Datos y correcciones
          </Tab>
        </TabList>
      </header>
      <div data-detail-scroll className="min-h-0 flex-1 overflow-y-auto px-10 pt-7 pb-12">
        <div role="tabpanel" data-tab={tab} className="max-w-[1180px]">
          {notice}
          <div className={aside ? "grid gap-x-11 gap-y-8 xl:grid-cols-[minmax(0,1fr)_18.75rem]" : undefined}>
            <div className="min-w-0">
              {tab === "article" ? <div key="article">{article}</div> : <div key="data">{data}</div>}
              {notesOf ? (
                <div className="mt-10">
                  <NotesEditor key={notesOf} objectId={notesOf} />
                </div>
              ) : null}
            </div>
            {/* In a narrow window the column goes first, as a fact box on top. */}
            {aside ? <aside className="order-first min-w-0 xl:order-none">{aside}</aside> : null}
          </div>
        </div>
      </div>
    </div>
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
      <button className="text-link hover:underline" onClick={onShowData}>
        Ver «Datos y correcciones»
      </button>
    </EmptyNote>
  )
}
