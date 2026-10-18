import { describe, expect, it } from "vitest"
import { unified } from "unified"
import remarkParse from "remark-parse"
import remarkMath from "remark-math"
import { REMARK_MATH_OPTIONS } from "./markdown-math"

function nodeTypes(markdown: string, withOptions: boolean): string[] {
  const processor = unified()
    .use(remarkParse)
    .use(remarkMath, withOptions ? REMARK_MATH_OPTIONS : undefined)
  const tree = processor.runSync(processor.parse(markdown))
  const types: string[] = []
  const walk = (node: { type: string; children?: unknown[] }) => {
    types.push(node.type)
    for (const child of (node.children ?? []) as { type: string }[]) walk(child)
  }
  walk(tree as unknown as { type: string; children?: unknown[] })
  return types
}

describe("markdown math in the readers", () => {
  const amounts = "Reclama $ 1.500.000 por la cuota y $ 300 de intereses."

  it("would draw text between two amounts as a formula with the default", () => {
    expect(nodeTypes(amounts, false)).toContain("inlineMath")
  })

  it("never draws an amount as a formula", () => {
    const types = nodeTypes(amounts, true)
    expect(types).not.toContain("inlineMath")
    expect(types).not.toContain("math")
  })

  it("keeps double-dollar math", () => {
    expect(nodeTypes("La fórmula $$a+b$$ queda.", true)).toContain("inlineMath")
    expect(nodeTypes("$$\na+b\n$$", true)).toContain("math")
  })
})
