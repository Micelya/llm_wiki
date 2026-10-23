import { useState, useCallback, useEffect } from "react"
import { Search, FileText } from "lucide-react"
import { useTranslation } from "react-i18next"
import { searchBase, type BaseSearchResult } from "@/commands/docbase"
import { tokenizeQuery } from "@/lib/search"
import { openWikiPath } from "@/lib/open-wiki-path"
import { isImeComposing } from "@/lib/keyboard-utils"
import { useSearchStore } from "@/stores/search-store"

/**
 * The search of the base (D-408): the texts, numbers and summaries of
 * its documents, and the person, organization or matter the question
 * names. Each result opens the screen of what it is, never a file. It is
 * asked from the top bar (Ctrl K), and refined here.
 */
export function SearchView() {
  const { t } = useTranslation()
  const asked = useSearchStore((s) => s.asked)
  const askedQuery = useSearchStore((s) => s.query)
  const [query, setQuery] = useState(askedQuery)
  const [results, setResults] = useState<BaseSearchResult[]>([])
  const [searching, setSearching] = useState(false)
  const [hasSearched, setHasSearched] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  const doSearch = useCallback(async (q: string) => {
    setProblem(null)
    if (!q.trim()) {
      setResults([])
      return
    }
    setSearching(true)
    setHasSearched(true)
    try {
      const found = await searchBase(q.trim())
      setResults(found.results)
    } catch (err) {
      console.error("Search failed:", err)
      setResults([])
      setProblem(typeof err === "string" ? err : String((err as Error)?.message ?? err))
    } finally {
      setSearching(false)
    }
  }, [])

  // What the top bar asked, every time it asks.
  useEffect(() => {
    if (asked === 0) return
    setQuery(askedQuery)
    void doSearch(askedQuery)
  }, [asked])

  async function handleOpen(result: BaseSearchResult) {
    setProblem(null)
    const opened = await openWikiPath(result.path)
    if (!opened) setProblem(`«${result.title}» ya no está en la base: buscá de nuevo.`)
  }

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="shrink-0 border-b bg-background px-10 pt-6 pb-4">
        <h1 className="mb-3 font-ui text-[1.75rem] leading-tight font-bold tracking-[-0.01em] text-strong">Buscar</h1>
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (isImeComposing(e)) return
              if (e.key === "Enter") void doSearch(query)
            }}
            placeholder={t("search.placeholderWithShortcut")}
            className="h-10 w-full max-w-3xl rounded-lg border border-border-strong bg-card py-2 pr-3 pl-9 text-sm placeholder:text-muted-foreground focus:border-link focus:ring-3 focus:ring-ring/30 focus:outline-none"
          />
        </div>
        {problem && <p className="mt-2 text-xs text-err">{problem}</p>}
      </div>

      {searching ? (
        <div className="flex-1 p-4 text-center text-sm text-muted-foreground">{t("search.searching")}</div>
      ) : !hasSearched ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-2 p-8 text-center text-sm text-muted-foreground">
          <Search className="h-8 w-8 text-muted-foreground/30" />
          <p>{t("search.pressEnter")}</p>
        </div>
      ) : results.length === 0 ? (
        <div className="flex-1 p-4 text-center text-sm text-muted-foreground">
          {t("search.noResults")} <span className="font-medium">"{query}"</span>
        </div>
      ) : (
        <div className="flex min-h-0 flex-1 flex-col">
          <div className="shrink-0 px-10 pt-4 pb-1 text-[0.8125rem] text-muted-foreground">
            {t("search.pageCount", { count: results.length })}
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto px-10 pt-2 pb-6">
            <div className="flex max-w-3xl flex-col gap-2">
              {results.map((result) => (
                <SearchResultCard key={result.path} result={result} query={query} onClick={() => void handleOpen(result)} />
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

function SearchResultCard({ result, query, onClick }: { result: BaseSearchResult; query: string; onClick: () => void }) {
  const named = result.graphRelatedTo ?? []
  return (
    <button
      type="button"
      onClick={onClick}
      className="w-full rounded-[10px] border bg-card p-3.5 text-left text-sm shadow-card transition-colors hover:border-border-strong hover:bg-hover"
    >
      <div className="mb-1.5 flex items-start gap-2">
        <FileText className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
        <div className="min-w-0 flex-1">
          <div className="truncate font-ui font-bold text-link">
            <HighlightedText text={result.title} query={query} />
          </div>
          {named.length > 0 && <div className="truncate text-[11px] text-muted-foreground">Nombra a {named.join(", ")}</div>}
        </div>
      </div>
      {result.snippet && (
        <p className="line-clamp-2 text-xs text-muted-foreground">
          <HighlightedText text={result.snippet} query={query} />
        </p>
      )}
    </button>
  )
}

function HighlightedText({ text, query }: { text: string; query: string }) {
  if (!query.trim()) return <>{text}</>

  // Each word of the question that counts, not the question as typed:
  // a period at its end must not stop a word from being marked.
  const tokens = tokenizeQuery(query)
  const patterns = tokens.length > 0 ? tokens : [query.trim()]
  const regex = new RegExp(`(${patterns.map(escapeRegex).join("|")})`, "gi")
  const parts = text.split(regex)

  return (
    <>
      {parts.map((part, i) =>
        regex.test(part) ? (
          <mark key={i} className="rounded bg-warn-bg px-0.5 text-strong ring-1 ring-warn-line">
            {part}
          </mark>
        ) : (
          <span key={i}>{part}</span>
        ),
      )}
    </>
  )
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}
