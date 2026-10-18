/**
 * Foldable sections in the pages the app writes.
 *
 * A page of a matter explains, document by document, why its documents go
 * together: useful to check, too long to read every time (D-268). The app
 * writes it as a callout with a fold mark, the way Obsidian does:
 *
 *     > [!info]- Why these documents go together (12 documents)
 *     > - document — reasons
 *
 * and this plugin draws such a quote as a closed `<details>` whose
 * `<summary>` is the first line (`[!info]+` opens it). Any other quote is
 * left as it is. No HTML is read from the page, so nothing a model or a
 * person writes can inject markup.
 */

interface MdNode {
  type: string
  value?: string
  children?: MdNode[]
  data?: { hName?: string; hProperties?: Record<string, unknown> }
}

const MARK = /^\[!([A-Za-z-]+)\]([+-]?)[ \t]*/

function fold(node: MdNode): void {
  for (const child of node.children ?? []) fold(child)
  if (node.type !== "blockquote") return
  const first = node.children?.[0]
  const text = first?.type === "paragraph" ? first.children?.[0] : undefined
  if (!first || text?.type !== "text" || typeof text.value !== "string") return
  const found = MARK.exec(text.value)
  if (!found || found[2] === "") return
  // The summary is the first line of the first paragraph; what follows it
  // in the same paragraph stays as the body.
  const rest = text.value.slice(found[0].length)
  const newline = rest.indexOf("\n")
  const title = newline >= 0 ? rest.slice(0, newline) : rest
  const after = newline >= 0 ? rest.slice(newline + 1) : ""
  const titleChildren: MdNode[] = [{ type: "text", value: title }]
  const remaining = first.children!.slice(1)
  const summary: MdNode = {
    type: "paragraph",
    children: newline >= 0 ? titleChildren : [...titleChildren, ...remaining],
    data: { hName: "summary" },
  }
  const body: MdNode[] = []
  if (newline >= 0 && (after.trim() !== "" || remaining.length > 0)) {
    body.push({ type: "paragraph", children: [{ type: "text", value: after }, ...remaining] })
  }
  node.children = [summary, ...body, ...node.children!.slice(1)]
  node.data = { ...(node.data ?? {}), hName: "details", hProperties: found[2] === "+" ? { open: true } : {} }
}

/** A remark plugin: quotes with a fold mark become `<details>`. */
export function remarkFoldable() {
  return (tree: MdNode) => {
    fold(tree)
  }
}
