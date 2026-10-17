import { describe, expect, it } from "vitest"
import { diffSummary, diffWords, fold, sideBySide, tokens, type DiffOp } from "./text-diff"

/** Applies the differences to `a`: it must give `b`. */
function apply(ops: DiffOp[]): { a: string[]; b: string[] } {
  const a: string[] = []
  const b: string[] = []
  for (const op of ops) {
    if (op.kind !== "added") a.push(...op.words)
    if (op.kind !== "removed") b.push(...op.words)
  }
  return { a, b }
}

const edits = (ops: DiffOp[]) => ops.filter((op) => op.kind !== "same").reduce((sum, op) => sum + op.words.length, 0)

describe("the differences between two texts, word by word", () => {
  it("finds the words that change", () => {
    const ops = diffWords(tokens("Pagará la suma de 100 pesos el día 5"), tokens("Pagará la suma de 250 pesos el día 10"))!
    expect(ops).toEqual([
      { kind: "same", words: ["Pagará", "la", "suma", "de"] },
      { kind: "removed", words: ["100"] },
      { kind: "added", words: ["250"] },
      { kind: "same", words: ["pesos", "el", "día"] },
      { kind: "removed", words: ["5"] },
      { kind: "added", words: ["10"] },
    ])
  })

  it("always rebuilds both texts, with the fewest changes", () => {
    const cases: [string, string, number][] = [
      ["a b c d e", "a x c d e y", 3],
      ["", "a b", 2],
      ["a b", "", 2],
      ["uno dos tres", "uno dos tres", 0],
      ["a b c a b b a", "c b a b a c", 5],
    ]
    for (const [left, right, expected] of cases) {
      const ops = diffWords(tokens(left), tokens(right))!
      expect(apply(ops)).toEqual({ a: tokens(left), b: tokens(right) })
      expect(edits(ops)).toBe(expected)
    }
  })

  it("gives up past the limit of differences instead of freezing", () => {
    const a = Array.from({ length: 50 }, (_, i) => `a${i}`)
    const b = Array.from({ length: 50 }, (_, i) => `b${i}`)
    expect(diffWords(a, b, 20)).toBeNull()
    expect(diffWords(a, b, 100)).not.toBeNull()
  })

  it("handles long texts that differ in little", () => {
    const a = Array.from({ length: 20000 }, (_, i) => `p${i % 97}`)
    const b = [...a]
    b[100] = "cambio"
    b.splice(15000, 0, "nuevo")
    const ops = diffWords(a, b)!
    expect(edits(ops)).toBe(3)
    expect(apply(ops)).toEqual({ a, b })
  })
})

describe("the side-by-side view", () => {
  it("pairs what one says with what the other says, and tells data from wording", () => {
    const blocks = sideBySide(diffWords(tokens("Total 100 a pagar en efectivo"), tokens("Total 250 a pagar con tarjeta"))!)
    expect(blocks).toEqual([
      { kind: "same", words: ["Total"] },
      { kind: "change", left: ["100"], right: ["250"], data: true },
      { kind: "same", words: ["a", "pagar"] },
      { kind: "change", left: ["en", "efectivo"], right: ["con", "tarjeta"], data: false },
    ])
    expect(diffSummary(blocks)).toBe("1 cambio de datos y 1 cambio de redacción.")
    expect(diffSummary([{ kind: "same", words: ["x"] }])).toBe("Los dos textos son iguales, palabra por palabra.")
  })

  it("folds long stretches both have, keeping some words around each change", () => {
    const words = Array.from({ length: 100 }, (_, i) => `w${i}`)
    expect(fold(words, false, false, 5)).toMatchObject({ head: words.slice(0, 5), hidden: 90, tail: words.slice(95) })
    // At the beginning nothing before the first change is needed.
    expect(fold(words, true, false, 5)).toMatchObject({ head: [], hidden: 95 })
    expect(fold(words.slice(0, 12), false, false, 5).hidden).toBe(0)
  })
})
