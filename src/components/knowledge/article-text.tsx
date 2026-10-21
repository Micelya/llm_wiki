import { getPageWords } from "@/commands/docbase"
import { formatAgo } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { WikiReader } from "@/components/editor/wiki-reader"

let pageWords: Promise<Record<string, string>> | null = null

/** The words of the pages do not change while the app runs: asked once. */
function pageWordsOnce(): Promise<Record<string, string>> {
  pageWords ??= getPageWords().catch((cause: unknown) => {
    pageWords = null
    throw cause
  })
  return pageWords
}

/**
 * The words the pages are written with, by id (`matterFacts`…), to find a
 * section of an article by its heading; `null` while they load.
 */
export function usePageWords(): ((id: string) => string) | null {
  const words = useDocbaseQuery("page-words", pageWordsOnce)
  const data = words.data
  return data ? (id) => data[id] ?? id : null
}

/**
 * An article as the projection last wrote it in the base (D-407): its
 * links open the screen of what they name. A correction shows in it when
 * the app writes it again, in the background.
 */
export function ArticleText({ body, writtenAt }: { body: string; writtenAt?: number }) {
  return (
    <div>
      <WikiReader body={body} />
      {writtenAt !== undefined && (
        <p className="mt-4 text-xs text-muted-foreground">
          Lo escribió la app {formatAgo(writtenAt)}, desde los documentos. Una corrección se ve acá cuando lo vuelve a escribir.
        </p>
      )}
    </div>
  )
}
