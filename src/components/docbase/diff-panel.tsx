import { useEffect, useState } from "react"
import { getDocument } from "@/commands/docbase"
import { ErrorNote } from "./page"
import { formatCount } from "./format"
import { diffSummary, diffWords, fold, sideBySide, tokens, type DiffBlock } from "./text-diff"

/**
 * Two similar documents side by side, with what changes marked: the
 * data (amounts, dates, numbers, identifiers) apart from the wording.
 * The texts are the ones the base extracted; long stretches both have
 * are folded.
 */
export function DiffPanel({
  left,
  right,
}: {
  left: { locationId: number; name: string }
  right: { locationId: number; name: string }
}) {
  const [blocks, setBlocks] = useState<DiffBlock[] | null>(null)
  const [problem, setProblem] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    Promise.all([getDocument(left.locationId), getDocument(right.locationId)])
      .then(([a, b]) => {
        if (!active) return
        if (!a?.text || !b?.text) {
          setProblem("Uno de los dos todavía no tiene su texto en la base.")
          return
        }
        const ops = diffWords(tokens(a.text), tokens(b.text))
        if (!ops) {
          setProblem("Tienen demasiadas diferencias para mostrarlas lado a lado. Abrí cada uno para compararlos.")
          return
        }
        setBlocks(sideBySide(ops))
      })
      .catch((cause) => active && setProblem(String(cause)))
    return () => {
      active = false
    }
  }, [left.locationId, right.locationId])

  if (problem) return <div className="mt-3"><ErrorNote>{problem}</ErrorNote></div>
  if (!blocks) return <p className="mt-3 text-xs text-muted-foreground">Comparando los textos…</p>

  return (
    <div className="mt-3 rounded-md border">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b bg-muted/30 px-3 py-1.5 text-xs">
        <span className="font-medium">{diffSummary(blocks)}</span>
        <span className="flex gap-3 text-muted-foreground">
          <span>
            <Mark tone="data">dato</Mark> cambia un dato
          </span>
          <span>
            <Mark tone="removed">texto</Mark> / <Mark tone="added">texto</Mark> cambia la redacción
          </span>
        </span>
      </div>
      <div className="grid max-h-[28rem] grid-cols-2 overflow-y-auto text-xs leading-relaxed">
        <div className="sticky top-0 border-r border-b bg-card px-3 py-1 font-medium break-words">{left.name}</div>
        <div className="sticky top-0 border-b bg-card px-3 py-1 font-medium break-words">{right.name}</div>
        {blocks.map((block, index) =>
          block.kind === "same" ? (
            <SameRow key={index} words={block.words} first={index === 0} last={index === blocks.length - 1} />
          ) : (
            <ChangeRow key={index} block={block} />
          ),
        )}
      </div>
    </div>
  )
}

function SameRow({ words, first, last }: { words: string[]; first: boolean; last: boolean }) {
  const folded = fold(words, first, last)
  const text = (
    <>
      {folded.head.join(" ")}
      {folded.hidden > 0 && (
        <span className="mx-1 rounded bg-muted px-1 text-muted-foreground">… {formatCount(folded.hidden)} palabras iguales …</span>
      )}
      {folded.tail.join(" ")}
    </>
  )
  return (
    <>
      <div className="border-r px-3 py-1 break-words text-muted-foreground">{text}</div>
      <div className="px-3 py-1 break-words text-muted-foreground">{text}</div>
    </>
  )
}

function ChangeRow({ block }: { block: Extract<DiffBlock, { kind: "change" }> }) {
  const tone = block.data ? "data" : undefined
  return (
    <>
      <div className="border-r px-3 py-1 break-words">
        {block.left.length > 0 ? <Mark tone={tone ?? "removed"}>{block.left.join(" ")}</Mark> : <Missing />}
      </div>
      <div className="px-3 py-1 break-words">
        {block.right.length > 0 ? <Mark tone={tone ?? "added"}>{block.right.join(" ")}</Mark> : <Missing />}
      </div>
    </>
  )
}

function Missing() {
  return <span className="text-muted-foreground italic">(no está)</span>
}

function Mark({ tone, children }: { tone: "data" | "removed" | "added"; children: React.ReactNode }) {
  const color =
    tone === "data"
      ? "bg-amber-500/20 text-amber-900 dark:text-amber-200"
      : tone === "removed"
        ? "bg-red-500/15 text-red-900 dark:text-red-200"
        : "bg-emerald-500/15 text-emerald-900 dark:text-emerald-200"
  return <span className={`rounded px-0.5 ${color}`}>{children}</span>
}
