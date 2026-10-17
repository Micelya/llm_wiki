import { describe, expect, it } from "vitest"
import { unified } from "unified"
import remarkParse from "remark-parse"
import { remarkFoldable } from "./markdown-fold"

interface MdNode {
  type: string
  value?: string
  children?: MdNode[]
  data?: { hName?: string; hProperties?: Record<string, unknown> }
}

function tree(markdown: string): MdNode {
  const processor = unified().use(remarkParse).use(remarkFoldable)
  return processor.runSync(processor.parse(markdown)) as unknown as MdNode
}

function text(node: MdNode): string {
  return node.value ?? (node.children ?? []).map(text).join("")
}

describe("foldable sections", () => {
  it("draws a quote with a fold mark as closed details", () => {
    const root = tree("> [!info]- Por qué van juntos (3 documentos)\n> - a — comparten el número 123\n> - b — nombran el mismo asunto\n")
    const quote = root.children![0]
    expect(quote.data?.hName).toBe("details")
    expect(quote.data?.hProperties).toEqual({})
    const summary = quote.children![0]
    expect(summary.data?.hName).toBe("summary")
    expect(text(summary)).toBe("Por qué van juntos (3 documentos)")
    expect(text(quote)).toContain("comparten el número 123")
  })

  it("opens it with a plus", () => {
    const quote = tree("> [!note]+ Abierto\n> cuerpo\n").children![0]
    expect(quote.data?.hName).toBe("details")
    expect(quote.data?.hProperties).toEqual({ open: true })
  })

  it("leaves any other quote as it is", () => {
    expect(tree("> una cita\n").children![0].data?.hName).toBeUndefined()
    expect(tree("> [!info] sin pliegue\n").children![0].data?.hName).toBeUndefined()
  })
})
