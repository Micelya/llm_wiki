/**
 * The differences between two texts, word by word, to show two similar
 * documents side by side. Computed on the screen, on request, from the
 * texts the base already has: nothing is stored.
 *
 * Myers' algorithm: it finds the fewest words to remove and add, and its
 * cost grows with how much the texts differ, not with how long they are,
 * which suits two documents that are similar by definition. Past a limit
 * of differences it gives up instead of freezing the screen.
 */

export type DiffOp = { kind: "same" | "removed" | "added"; words: string[] }

/** A part of the side-by-side view: text both have, or what changes from one to the other. */
export type DiffBlock =
  | { kind: "same"; words: string[] }
  | { kind: "change"; left: string[]; right: string[]; data: boolean }

/** Differences (words removed plus words added) past which the texts are not shown side by side. */
export const MAX_EDITS = 2500

export function tokens(text: string): string[] {
  return text.split(/\s+/).filter(Boolean)
}

/** The words to keep, remove and add to go from `a` to `b`; `null` past `maxEdits`. */
export function diffWords(a: readonly string[], b: readonly string[], maxEdits: number = MAX_EDITS): DiffOp[] | null {
  // What both start and end with costs nothing to set aside.
  let start = 0
  while (start < a.length && start < b.length && a[start] === b[start]) start++
  let endA = a.length
  let endB = b.length
  while (endA > start && endB > start && a[endA - 1] === b[endB - 1]) {
    endA--
    endB--
  }
  const middle = middleDiff(a.slice(start, endA), b.slice(start, endB), maxEdits)
  if (middle === null) return null
  const ops: DiffOp[] = []
  push(ops, "same", a.slice(0, start))
  for (const op of middle) push(ops, op.kind, op.words)
  push(ops, "same", a.slice(endA))
  return ops
}

function push(ops: DiffOp[], kind: DiffOp["kind"], words: readonly string[]): void {
  if (words.length === 0) return
  const last = ops[ops.length - 1]
  if (last && last.kind === kind) last.words.push(...words)
  else ops.push({ kind, words: [...words] })
}

function middleDiff(a: readonly string[], b: readonly string[], maxEdits: number): DiffOp[] | null {
  const n = a.length
  const m = b.length
  if (n === 0 && m === 0) return []
  if (n === 0) return [{ kind: "added", words: [...b] }]
  if (m === 0) return [{ kind: "removed", words: [...a] }]
  const max = Math.min(n + m, maxEdits)
  const offset = max + 1
  const v = new Int32Array(2 * max + 3)
  // For each step d, the furthest x reached on diagonals -d..d.
  const trace: Int32Array[] = []
  for (let d = 0; d <= max; d++) {
    const snapshot = new Int32Array(2 * d + 1)
    for (let k = -d; k <= d; k++) snapshot[k + d] = v[k + offset]
    trace.push(snapshot)
    for (let k = -d; k <= d; k += 2) {
      let x: number
      if (k === -d || (k !== d && v[k - 1 + offset] < v[k + 1 + offset])) x = v[k + 1 + offset]
      else x = v[k - 1 + offset] + 1
      let y = x - k
      while (x < n && y < m && a[x] === b[y]) {
        x++
        y++
      }
      v[k + offset] = x
      if (x >= n && y >= m) return backtrack(a, b, trace, d, k, x)
    }
  }
  return null
}

function backtrack(a: readonly string[], b: readonly string[], trace: Int32Array[], dEnd: number, kEnd: number, xEnd: number): DiffOp[] {
  const reversed: { kind: DiffOp["kind"]; word: string }[] = []
  let x = xEnd
  let y = xEnd - kEnd
  for (let d = dEnd; d > 0; d--) {
    // The values at the start of step d are those left by step d - 1.
    const before = trace[d]
    const at = (k: number) => before[k + d]
    const k = x - y
    const prevK = k === -d || (k !== d && at(k - 1) < at(k + 1)) ? k + 1 : k - 1
    const prevX = at(prevK)
    const prevY = prevX - prevK
    while (x > prevX && y > prevY) {
      reversed.push({ kind: "same", word: a[x - 1] })
      x--
      y--
    }
    if (x === prevX) reversed.push({ kind: "added", word: b[prevY] })
    else reversed.push({ kind: "removed", word: a[prevX] })
    x = prevX
    y = prevY
  }
  while (x > 0 && y > 0) {
    reversed.push({ kind: "same", word: a[x - 1] })
    x--
    y--
  }
  const ops: DiffOp[] = []
  for (let index = reversed.length - 1; index >= 0; index--) push(ops, reversed[index].kind, [reversed[index].word])
  return ops
}

/** A changed word that is a datum (an amount, a date, a number, an identifier): it has a digit. */
export function looksLikeData(word: string): boolean {
  return /\d/.test(word)
}

/**
 * The differences as the side-by-side view shows them: what both have,
 * and each change with what one says and what the other says. A change
 * is a datum when every word that changes in it has a digit; otherwise
 * it is wording.
 */
export function sideBySide(ops: readonly DiffOp[]): DiffBlock[] {
  const blocks: DiffBlock[] = []
  for (const op of ops) {
    if (op.kind === "same") {
      blocks.push({ kind: "same", words: op.words })
      continue
    }
    let last = blocks[blocks.length - 1]
    if (!last || last.kind !== "change") {
      last = { kind: "change", left: [], right: [], data: true }
      blocks.push(last)
    }
    if (op.kind === "removed") last.left.push(...op.words)
    else last.right.push(...op.words)
  }
  for (const block of blocks) {
    if (block.kind === "change") block.data = [...block.left, ...block.right].every(looksLikeData)
  }
  return blocks
}

/** How many words of text both have are shown around a change; the rest of a long stretch is folded. */
export const CONTEXT_WORDS = 12

/** A long stretch both have, folded: its beginning and end, and how many words are in between. */
export function fold(words: readonly string[], first: boolean, last: boolean, context: number = CONTEXT_WORDS): {
  head: string[]
  hidden: number
  tail: string[]
} {
  const keepHead = first ? 0 : context
  const keepTail = last ? 0 : context
  if (words.length <= keepHead + keepTail + context) return { head: [...words], hidden: 0, tail: [] }
  return {
    head: words.slice(0, keepHead),
    hidden: words.length - keepHead - keepTail,
    tail: words.slice(words.length - keepTail),
  }
}

/** How much changes, in a sentence. */
export function diffSummary(blocks: readonly DiffBlock[]): string {
  const changes = blocks.filter((block): block is Extract<DiffBlock, { kind: "change" }> => block.kind === "change")
  if (changes.length === 0) return "Los dos textos son iguales, palabra por palabra."
  const data = changes.filter((block) => block.data).length
  const wording = changes.length - data
  const parts: string[] = []
  if (data > 0) parts.push(`${data} ${data === 1 ? "cambio de datos" : "cambios de datos"}`)
  if (wording > 0) parts.push(`${wording} ${wording === 1 ? "cambio de redacción" : "cambios de redacción"}`)
  return `${parts.join(" y ")}.`
}
