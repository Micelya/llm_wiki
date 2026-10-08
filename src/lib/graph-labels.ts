/**
 * Label policy for the wiki graph.
 *
 * With every node labelled the labels pile on top of each other and none
 * can be read. The rules here keep the graph legible, and the graph view
 * only asks them what to draw:
 *
 * - At the default zoom only the well-connected nodes carry a label; the
 *   rest get theirs as the user zooms in.
 * - Long titles are shortened on the canvas; the node under the pointer
 *   shows its full title.
 * - A clicked node stays in focus, with its neighbours, until the
 *   background is clicked. Hovering another node takes precedence.
 */

export type NodeFocus = { node: string; neighbors: Set<string> } | null

export const MAX_LABEL_CHARS = 32
/** Share of the busiest node's links a node needs to be labelled without zooming. */
export const HUB_LINK_SHARE = 0.15
/** Above this many neighbours, forcing all their labels would overlap again. */
export const MAX_FORCED_NEIGHBOR_LABELS = 12

export function shortenLabel(label: string, maxChars: number = MAX_LABEL_CHARS): string {
  const chars = Array.from(label.trim())
  if (chars.length <= maxChars) return chars.join("")
  return `${chars.slice(0, maxChars - 1).join("").trimEnd()}…`
}

/**
 * Rendered size below which a node gets no label. `hubNodeSize` is the
 * size of a node holding `HUB_LINK_SHARE` of the links; `baseThreshold`
 * is the stricter limit large graphs already apply.
 */
export function hubLabelThreshold(baseThreshold: number, hubNodeSize: number): number {
  return Math.max(baseThreshold, hubNodeSize)
}

/** The focus in effect: hover wins over the pinned node; stale nodes are ignored. */
export function resolveFocus(
  hover: NodeFocus,
  pinned: NodeFocus,
  hasNode: (node: string) => boolean,
): NodeFocus {
  if (hover && hasNode(hover.node)) return hover
  if (pinned && hasNode(pinned.node)) return pinned
  return null
}

export interface FocusLabel {
  label: string
  forceLabel: boolean
}

/** Label of a node while some node is in focus. */
export function focusLabel(
  focus: NonNullable<NodeFocus>,
  node: string,
  shortLabel: string,
  fullLabel: string,
): FocusLabel {
  if (focus.node === node) return { label: fullLabel || shortLabel, forceLabel: true }
  if (focus.neighbors.has(node)) {
    return {
      label: shortLabel,
      forceLabel: focus.neighbors.size <= MAX_FORCED_NEIGHBOR_LABELS,
    }
  }
  return { label: "", forceLabel: false }
}
