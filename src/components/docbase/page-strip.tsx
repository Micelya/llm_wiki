import { useEffect, useRef, useState, type ReactNode } from "react"
import type { PDFDocumentLoadingTask, PDFDocumentProxy, RenderTask } from "pdfjs-dist"
import { Check, ExternalLink, Merge, Scissors, Sparkles } from "lucide-react"
import { Button } from "@/components/ui/button"
import { cutAsk, cutConfirm, cutJoin, cutSplit, includedParts, type IncludedPart, type PartOf } from "@/commands/docbase"
import { getFileSize, readFileAsBase64 } from "@/commands/fs"
import { decodeBase64 } from "@/components/editor/file-preview"
import { Section } from "@/components/knowledge/shared"
import { vocabulary, type Vocabulary } from "@/vocabulary"
import { ErrorNote } from "./page"
import { useDocbaseQuery } from "./use-docbase-query"

/**
 * The strip of pages of a file that joins several documents (D-560): each
 * row is one of the documents it includes, from the page it starts on,
 * with how that start stands (backed, uncertain or confirmed) and who
 * said it, and the pages nothing backs marked to look at. What is backed
 * goes on by itself, and what is uncertain goes on marked (D-603): a
 * person looks only at that. A PDF of one document with uncertain steps
 * has its strip too, with the whole file as its only row. A person splits
 * a document at a page, joins one to the one before it, or confirms them
 * all. While some step is uncertain, a person can ask the model about the
 * file (D-606): by itself the model is asked only in a file a rule already
 * cut.
 */

/** What a person does to the cuts of a file. */
export type CutAction = { kind: "split"; page: number } | { kind: "join"; page: number } | { kind: "confirm" } | { kind: "ask" }

/** Each action is its command of the base, which answers the parts as they are now. */
export function runCutAction(hash: string, action: CutAction): Promise<IncludedPart[]> {
  switch (action.kind) {
    case "split":
      return cutSplit(hash, action.page)
    case "join":
      return cutJoin(hash, action.page)
    case "confirm":
      return cutConfirm(hash)
    case "ask":
      return cutAsk(hash)
  }
}

/** A text of the vocabulary with its `{name}` filled. */
export function fill(text: string, values: Record<string, string | number>): string {
  return text.replace(/\{(\w+)\}/g, (whole, name: string) => (name in values ? String(values[name]) : whole))
}

/**
 * «Incierto · lo propuso la app por el texto»: how a document's start stands, in the vertical's words.
 * The mark decides over the state: the appearance of a whole file stays confirmed in the base (D-603).
 */
export function cutLine(part: Pick<PartOf, "cutBy" | "cutState" | "uncertain">, words: Vocabulary): string {
  const terms = words.included
  const state = part.uncertain ? "uncertain" : part.cutState
  return `${terms.states[state] ?? state} · ${terms.by[part.cutBy] ?? part.cutBy}`
}

/**
 * Under a file's name in the list: «incluye 5 documentos · 3 inciertos»; for a PDF of one document
 * with an uncertain step, «un documento · incierto»; nothing otherwise.
 */
export function IncludedNote({ included, unconfirmed, words = vocabulary() }: { included: number; unconfirmed: number; words?: Vocabulary }) {
  const terms = words.included
  if (included <= 0) {
    if (unconfirmed <= 0) return null
    return (
      <div className="mt-0.5 text-[0.78125rem] text-amber-800 dark:text-amber-300" data-included={0}>
        {terms.listOneUncertain}
      </div>
    )
  }
  return (
    <div className="mt-0.5 text-[0.78125rem] text-muted-foreground" data-included={included}>
      {fill(terms.listIncludes, { count: included })}
      {unconfirmed > 0 && (
        <>
          {" · "}
          <span className="text-amber-800 dark:text-amber-300">{fill(terms.listUnconfirmed, { count: unconfirmed })}</span>
        </>
      )}
    </div>
  )
}

const MAX_THUMBNAIL_PDF_BYTES = 128 * 1024 * 1024
const THUMB_WIDTH = 72

/** The documents a file includes, with its strip of pages; nothing for a file of one document. */
export function CompoundStrip({
  hash,
  path,
  isPdf,
  onOpen,
}: {
  hash: string
  path: string
  isPdf: boolean
  onOpen: (part: IncludedPart) => void
}) {
  const query = useDocbaseQuery(`included:${hash}`, () => includedParts(hash))
  const [parts, setParts] = useState<IncludedPart[] | null>(null)
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  useEffect(() => {
    if (query.data) setParts(query.data)
  }, [query.data])
  const shown = parts ?? query.data
  const pdf = usePdf(isPdf && shown && shown.length > 0 ? path : null)
  if (!shown || shown.length === 0) return null

  async function act(action: CutAction) {
    setBusy(true)
    setProblem(null)
    try {
      setParts(await runCutAction(hash, action))
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }

  return (
    <PageStrip
      parts={shown}
      words={vocabulary()}
      busy={busy}
      problem={problem}
      onAction={(action) => void act(action)}
      onOpen={onOpen}
      thumbnail={(page) => <PageThumb pdf={pdf} page={page} />}
    />
  )
}

/** The strip itself, with what it shows given: the screen of a joined file and its tests. */
export function PageStrip({
  parts,
  words,
  busy = false,
  problem = null,
  onAction,
  onOpen,
  thumbnail,
}: {
  parts: IncludedPart[]
  words: Vocabulary
  busy?: boolean
  problem?: string | null
  onAction: (action: CutAction) => void
  onOpen: (part: IncludedPart) => void
  thumbnail?: (page: number) => ReactNode
}) {
  const terms = words.included
  const unconfirmed = parts.filter((part) => part.cutState !== "confirmed").length
  const uncertain = parts.filter((part) => part.uncertain).length
  const whole = parts.length === 1 && parts[0].whole
  return (
    <Section
      title={whole ? terms.wholeTitle : terms.title}
      count={whole ? undefined : parts.length}
      intro={whole ? terms.wholeDescription : terms.description}
      action={
        <div className="flex flex-wrap justify-end gap-1.5">
          {uncertain > 0 && (
            <Button variant="outline" size="sm" disabled={busy} title={terms.askHelp} data-cut-ask="" onClick={() => onAction({ kind: "ask" })}>
              <Sparkles /> {terms.ask}
            </Button>
          )}
          <Button variant={uncertain > 0 ? "default" : "outline"} size="sm" disabled={busy || unconfirmed === 0} onClick={() => onAction({ kind: "confirm" })}>
            <Check /> {unconfirmed > 0 ? terms.confirm : terms.confirmed}
          </Button>
        </div>
      }
    >
      {problem && (
        <div className="mb-3">
          <ErrorNote>{problem}</ErrorNote>
        </div>
      )}
      <ol className="flex flex-col gap-3" data-strip="">
        {parts.map((part, index) => (
          <li
            key={part.versionId}
            className="rounded-[10px] border bg-card px-4 py-3"
            data-part={index + 1}
            data-cut-state={part.cutState}
            data-cut-by={part.cutBy}
            data-uncertain={part.uncertain ? "" : undefined}
          >
            <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
              <span className="font-medium">{fill(terms.part, { n: index + 1 })}</span>
              <span className="text-sm text-muted-foreground">{fill(terms.pages, { first: part.firstPage, last: part.lastPage })}</span>
              <span
                className={
                  part.uncertain
                    ? "rounded-md bg-amber-500/10 px-1.5 text-sm text-amber-800 dark:text-amber-300"
                    : "text-sm text-muted-foreground"
                }
              >
                {cutLine(part, words)}
              </span>
              {!part.whole && (
                <Button variant="link" size="sm" className="ml-auto h-auto p-0" onClick={() => onOpen(part)}>
                  <ExternalLink /> {terms.open}
                </Button>
              )}
            </div>
            <div className="mt-2 flex flex-wrap gap-2">
              {pagesOf(part).map((page) => {
                const doubtful = part.doubtfulPages.includes(page)
                const starts = page === part.firstPage
                return (
                  <div
                    key={page}
                    data-page={page}
                    data-doubtful={doubtful ? "" : undefined}
                    className={`flex w-[88px] flex-col items-center gap-1 rounded-md border p-1 text-xs ${
                      doubtful ? "border-amber-500 bg-amber-500/5" : starts ? "border-link" : "border-border"
                    }`}
                    title={doubtful ? terms.doubtfulHelp : undefined}
                  >
                    {thumbnail ? thumbnail(page) : <PagePlaceholder page={page} />}
                    <span className="text-muted-foreground">{page}</span>
                    {doubtful && <span className="font-medium text-amber-800 dark:text-amber-300">{terms.doubtful}</span>}
                    {page > 1 &&
                      (starts ? (
                        <button
                          type="button"
                          className="text-link hover:underline disabled:opacity-50"
                          disabled={busy}
                          data-action="join"
                          onClick={() => onAction({ kind: "join", page })}
                        >
                          <Merge className="inline size-3" /> {terms.join}
                        </button>
                      ) : (
                        <button
                          type="button"
                          className="text-link hover:underline disabled:opacity-50"
                          disabled={busy}
                          data-action="split"
                          onClick={() => onAction({ kind: "split", page })}
                        >
                          <Scissors className="inline size-3" /> {terms.split}
                        </button>
                      ))}
                  </div>
                )
              })}
            </div>
          </li>
        ))}
      </ol>
    </Section>
  )
}

function pagesOf(part: IncludedPart): number[] {
  const pages: number[] = []
  for (let page = part.firstPage; page <= part.lastPage && page > 0; page += 1) pages.push(page)
  return pages
}

function PagePlaceholder({ page }: { page: number }) {
  return (
    <div className="flex h-[96px] w-[72px] items-center justify-center rounded-sm bg-muted text-muted-foreground" aria-hidden>
      {page}
    </div>
  )
}

/** One page drawn small, once it comes into sight; its number while it is not drawn. */
function PageThumb({ pdf, page }: { pdf: PDFDocumentProxy | null; page: number }) {
  const ref = useRef<HTMLCanvasElement | null>(null)
  const [drawn, setDrawn] = useState(false)
  useEffect(() => {
    const canvas = ref.current
    if (!pdf || !canvas || page > pdf.numPages) return
    let cancelled = false
    let task: RenderTask | null = null
    const draw = async () => {
      const pdfPage = await pdf.getPage(page)
      if (cancelled) return
      const ratio = window.devicePixelRatio || 1
      const viewport = pdfPage.getViewport({ scale: (THUMB_WIDTH * ratio) / pdfPage.getViewport({ scale: 1 }).width })
      const context = canvas.getContext("2d")
      if (!context) return
      canvas.width = Math.floor(viewport.width)
      canvas.height = Math.floor(viewport.height)
      task = pdfPage.render({ canvas, canvasContext: context, viewport })
      await task.promise
      if (!cancelled) setDrawn(true)
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return
        observer.disconnect()
        draw().catch(() => {})
      },
      { rootMargin: "200px" },
    )
    observer.observe(canvas)
    return () => {
      cancelled = true
      observer.disconnect()
      task?.cancel()
    }
  }, [pdf, page])
  return (
    <div className="relative h-[96px] w-[72px]">
      <canvas ref={ref} className={`h-full w-full rounded-sm bg-white object-contain ${drawn ? "" : "opacity-0"}`} />
      {!drawn && (
        <div className="absolute inset-0">
          <PagePlaceholder page={page} />
        </div>
      )}
    </div>
  )
}

/** The PDF at `path`, read once, for its pages to be drawn; null while it loads or when it cannot be read. */
function usePdf(path: string | null): PDFDocumentProxy | null {
  const [pdf, setPdf] = useState<PDFDocumentProxy | null>(null)
  useEffect(() => {
    if (!path) return
    let disposed = false
    let loadingTask: PDFDocumentLoadingTask | null = null
    void (async () => {
      try {
        // As the preview reads a PDF: its bytes over Tauri, never a file URL.
        if ((await getFileSize(path)) > MAX_THUMBNAIL_PDF_BYTES) return
        const [{ getDocument, GlobalWorkerOptions }, workerModule, file] = await Promise.all([
          import("pdfjs-dist/legacy/build/pdf.mjs"),
          import("pdfjs-dist/legacy/build/pdf.worker.min.mjs?url"),
          readFileAsBase64(path),
        ])
        if (disposed) return
        GlobalWorkerOptions.workerSrc = workerModule.default
        loadingTask = getDocument({ data: decodeBase64(file.base64) })
        const loaded = await loadingTask.promise
        if (disposed) await loaded.destroy()
        else setPdf(loaded)
      } catch {
        // Without its pages drawn, the strip shows their numbers.
      }
    })()
    return () => {
      disposed = true
      setPdf(null)
      if (loadingTask) void loadingTask.destroy()
    }
  }, [path])
  return pdf
}
