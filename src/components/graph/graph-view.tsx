import { useEffect, useCallback, useMemo, useState, useRef, type ChangeEvent, type SetStateAction } from "react"
import Graph from "graphology"
import { SigmaContainer, useLoadGraph, useRegisterEvents, useSetSettings, useSigma } from "@react-sigma/core"
import "@react-sigma/core/lib/style.css"
import type { NodeHoverDrawingFunction } from "sigma/rendering"
import type { SigmaNodeEventPayload } from "sigma/types"
import forceAtlas2 from "graphology-layout-forceatlas2"
import { Network, RefreshCw, ZoomIn, ZoomOut, Maximize, Layers, Tag, Lightbulb, AlertTriangle, Link2, X, Search, Loader2, Filter, RotateCcw, EyeOff } from "lucide-react"
import { ErrorBoundary } from "@/components/error-boundary"
import { useResearchStore } from "@/stores/research-store"
import { Button } from "@/components/ui/button"
import { useWikiStore, type GraphColorMode } from "@/stores/wiki-store"
import { getKnowledgeOverview } from "@/commands/docbase"
import { buildBaseGraph, objectOfNode, type GraphNode, type GraphEdge, type CommunityInfo, type GraphRelation } from "@/lib/wiki-graph"
import { openPageObject } from "@/lib/open-wiki-path"
import { GRAPH_NODE_TYPES, matterStateInWords, nodeTypeName, relationStateInWords } from "@/lib/graph-words"
import { findSurprisingConnections, detectKnowledgeGaps, researchNodeForKnowledgeGap, researchSeedForKnowledgeGap, type SurprisingConnection, type KnowledgeGap } from "@/lib/graph-insights"
import type { ResearchTarget } from "@/lib/deep-research"
import { queueResearch } from "@/lib/deep-research"
import { optimizeResearchTopic } from "@/lib/optimize-research-topic"
import { groundedFallbackResearchQuery } from "@/lib/research-query-grounding"
import { applyGraphFilters, hasActiveGraphFilters, type GraphFilterState } from "@/lib/graph-filters"
import { applyGraphSearch } from "@/lib/graph-search"
import { HUB_LINK_SHARE, focusLabel, hubLabelThreshold, resolveFocus, shortenLabel, type NodeFocus } from "@/lib/graph-labels"
import { vocabulary } from "@/vocabulary"

/** A color for each kind of node of the base; a proposed matter is a lighter one (D-410). */
// The brand's hues (P-057), in tones that read over the light and the dark
// background alike: the canvas of the graph cannot read the CSS variables.
const NODE_TYPE_COLORS: Record<string, string> = {
  persona: "#B0478F",      // the magenta of the brand, lighter
  organizacion: "#5B83B8", // the blue of «información», lighter
  grupo: "#DD0D6E",        // the pink of the brand
  documento: "#9B7693",    // the medium purple of the manual
  asunto: "#2E8A57",       // the green of «bien»: a matter a person confirmed
  other: "#A69CAA",        // lilac grey
}
const PROPOSED_MATTER_COLOR = "#8CCBA8" // the same green, lighter

/** The interface's words of the graph. */
const TEXT = {
  buildingGraph: "Armando el grafo…",
  retry: "Reintentar",
  noPages: "Todavía no hay nada en el grafo",
  importSourcesHint: "Se arma con lo que la app lee de los documentos de las carpetas incorporadas.",
  knowledgeGraph: "Grafo",
  hidden: "ocultos",
  pages: "nodos",
  links: "relaciones",
  searchLabel: "Buscar en el grafo",
  searchPlaceholder: "Buscar nodos…",
  clearSearch: "Borrar la búsqueda",
  closeSearch: "Cerrar la búsqueda",
  noSearchResults: "Ningún nodo coincide",
  noVisibleNodes: "No hay nodos visibles",
  filter: "Filtrar",
  reset: "Restablecer",
  resetFilters: "Restablecer los filtros",
  any: "Cualquiera",
  showAllTypes: "Mostrar todos los tipos",
  doubleClickToggle: "Doble clic para mostrar u ocultar",
  expandLegend: "Mostrar la leyenda",
  collapseLegend: "Ocultar la leyenda",
  type: "Tipo",
  community: "Comunidad",
  insights: "Análisis",
  resizing: "Ajustando…",
  graphFilters: "Filtros del grafo",
  quickFilters: "Filtros rápidos",
  hideIsolated: "Ocultar los nodos sin relaciones",
  minLinks: "Mínimo de relaciones",
  minLinksHint: "Oculta los nodos con menos",
  maxLinks: "Máximo de relaciones",
  maxLinksHint: "Oculta los nodos con más",
  displayTuning: "Presentación",
  nodeSize: "Tamaño de los nodos",
  spacing: "Separación",
  displayTuningHint: "Sirven para leer un grafo denso en una pantalla chica.",
  nodeTypes: "Tipos",
  hiddenNodes: "Nodos ocultos",
  show: "Mostrar",
  hideThisNode: "Ocultar este nodo",
  nodeTypesLabel: "Tipos",
  communitiesLabel: "Comunidades",
  showAll: "Mostrar todos",
  surprisingConnections: "Relaciones que llaman la atención",
  knowledgeGaps: "Huecos y puntos clave",
  deepResearch: "Investigar en internet",
  generatingTopic: "Preparando el tema…",
  researchTopic: "Tema",
  searchQueries: "Búsquedas",
  cancel: "Cancelar",
  startResearch: "Investigar",
  dismiss: "Descartar",
  close: "Cerrar",
}

const CUSTOM_NODE_COLORS = [
  "#5B83B8",
  "#2E8A57",
  "#C98A2E",
  "#DD0D6E",
  "#9B7693",
  "#3C9DA0",
  "#B0478F",
  "#7A9A3A",
]

/** The communities, in the brand's hues and the states' ones, apart from each other. */
const COMMUNITY_COLORS = [
  "#B0478F",
  "#5B83B8",
  "#C98A2E",
  "#2E8A57",
  "#9B7693",
  "#DD0D6E",
  "#3C9DA0",
  "#E09AD0",
  "#93B8EA",
  "#8A4A00",
  "#74D3A0",
  "#6E5670",
]

type GraphThemePalette = {
  defaultEdge: string
  label: string
  hoverLabelText: string
  hoverLabelBackground: string
  hoverLabelBorder: string
  hoverLabelShadow: string
  mutedNodeMixTarget: string
  dimmedEdge: string
  activeEdge: string
  edgeLabel: string
}

const BASE_NODE_SIZE = 8
const MAX_NODE_SIZE = 28
const DEFAULT_GRAPH_SPACING = 1
const GRAPH_SPACING_DEBOUNCE_MS = 180
const WORKER_LAYOUT_NODE_THRESHOLD = 220

type HoverState = { node: string; neighbors: Set<string> } | null
/** The relations of an edge under the pointer, or the one tapped (pinned). */
type EdgeDetail = {
  source: string
  target: string
  relations: GraphRelation[]
  x: number
  y: number
  pinned: boolean
}

function graphThemePalette(isDark: boolean): GraphThemePalette {
  return isDark
    ? {
        // The dark greys of the brand (index.css, --d-*).
        defaultEdge: "rgba(155,118,147,0.22)",
        label: "#EEE9F1",
        hoverLabelText: "#EEE9F1",
        hoverLabelBackground: "rgba(26,17,30,0.96)",
        hoverLabelBorder: "rgba(194,188,198,0.30)",
        hoverLabelShadow: "rgba(0,0,0,0.55)",
        mutedNodeMixTarget: "#2E2236",
        dimmedEdge: "rgba(67,53,75,0.30)",
        activeEdge: "#E09AD0",
        edgeLabel: "#C2BCC6",
      }
    : {
        // The lilac greys of the brand (index.css, --g-*).
        defaultEdge: "#DAD5DE",
        label: "#2F0A38",
        hoverLabelText: "#1B0A20",
        hoverLabelBackground: "rgba(255,255,255,0.97)",
        hoverLabelBorder: "rgba(47,10,56,0.14)",
        hoverLabelShadow: "rgba(47,10,56,0.18)",
        mutedNodeMixTarget: "#F1EDF3",
        dimmedEdge: "rgba(194,188,198,0.30)",
        activeEdge: "#8B2778",
        edgeLabel: "#6E5670",
      }
}

function drawRoundedRect(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
) {
  const safeRadius = Math.min(radius, width / 2, height / 2)
  context.beginPath()
  context.moveTo(x + safeRadius, y)
  context.lineTo(x + width - safeRadius, y)
  context.quadraticCurveTo(x + width, y, x + width, y + safeRadius)
  context.lineTo(x + width, y + height - safeRadius)
  context.quadraticCurveTo(x + width, y + height, x + width - safeRadius, y + height)
  context.lineTo(x + safeRadius, y + height)
  context.quadraticCurveTo(x, y + height, x, y + height - safeRadius)
  context.lineTo(x, y + safeRadius)
  context.quadraticCurveTo(x, y, x + safeRadius, y)
  context.closePath()
}

function createGraphNodeHoverRenderer(palette: GraphThemePalette): NodeHoverDrawingFunction {
  return (context, data, settings) => {
    const label = typeof data.label === "string" ? data.label : ""
    const labelSize = settings.labelSize
    const font = settings.labelFont
    const weight = settings.labelWeight
    const nodeRadius = Math.max(data.size, labelSize / 2) + 3

    context.save()
    context.shadowOffsetX = 0
    context.shadowOffsetY = 2
    context.shadowBlur = 10
    context.shadowColor = palette.hoverLabelShadow
    context.fillStyle = palette.hoverLabelBackground
    context.strokeStyle = palette.hoverLabelBorder
    context.lineWidth = 1

    context.beginPath()
    context.arc(data.x, data.y, nodeRadius, 0, Math.PI * 2)
    context.closePath()
    context.fill()
    context.stroke()

    if (label) {
      context.font = `${weight} ${labelSize}px ${font}`
      const paddingX = 8
      const paddingY = 4
      const gap = 6
      const textWidth = context.measureText(label).width
      const boxWidth = Math.ceil(textWidth + paddingX * 2)
      const boxHeight = Math.ceil(labelSize + paddingY * 2)
      const boxX = data.x + nodeRadius + gap
      const boxY = data.y - boxHeight / 2

      drawRoundedRect(context, boxX, boxY, boxWidth, boxHeight, 5)
      context.fill()
      context.stroke()

      context.shadowBlur = 0
      context.shadowOffsetY = 0
      context.fillStyle = palette.hoverLabelText
      context.fillText(label, boxX + paddingX, data.y + labelSize / 3)
    }

    context.restore()
  }
}

function useResolvedDarkMode(): boolean {
  const [isDark, setIsDark] = useState(() => document.documentElement.classList.contains("dark"))

  useEffect(() => {
    const root = document.documentElement
    const sync = () => setIsDark(root.classList.contains("dark"))
    sync()
    const observer = new MutationObserver(sync)
    observer.observe(root, { attributes: true, attributeFilter: ["class"] })
    return () => observer.disconnect()
  }, [])

  return isDark
}

function nodeColor(type: string, confirmation?: string | null): string {
  if (type === "asunto" && confirmation === "proposed") return PROPOSED_MATTER_COLOR
  if (NODE_TYPE_COLORS[type]) return NODE_TYPE_COLORS[type]
  let hash = 0
  for (const char of type) hash = (hash * 31 + char.charCodeAt(0)) >>> 0
  return CUSTOM_NODE_COLORS[hash % CUSTOM_NODE_COLORS.length] ?? NODE_TYPE_COLORS.other
}

function hexToRgba(hex: string, alpha: number): string {
  const r = parseInt(hex.slice(1, 3), 16)
  const g = parseInt(hex.slice(3, 5), 16)
  const b = parseInt(hex.slice(5, 7), 16)
  return `rgba(${r},${g},${b},${alpha})`
}

function mixColor(color1: string, color2: string, ratio: number): string {
  const hex = (c: string) => parseInt(c, 16)
  const r1 = hex(color1.slice(1, 3)), g1 = hex(color1.slice(3, 5)), b1 = hex(color1.slice(5, 7))
  const r2 = hex(color2.slice(1, 3)), g2 = hex(color2.slice(3, 5)), b2 = hex(color2.slice(5, 7))
  const r = Math.round(r1 + (r2 - r1) * ratio)
  const g = Math.round(g1 + (g2 - g1) * ratio)
  const b = Math.round(b1 + (b2 - b1) * ratio)
  return `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`
}

function graphDensityScale(nodeCount: number): number {
  if (nodeCount <= 150) return 1
  return Math.max(0.35, Math.sqrt(150 / nodeCount))
}

function nodeSize(linkCount: number, maxLinks: number, nodeCount: number, userScale: number): number {
  if (maxLinks === 0) return BASE_NODE_SIZE
  const ratio = linkCount / maxLinks
  const size = BASE_NODE_SIZE + Math.sqrt(ratio) * (MAX_NODE_SIZE - BASE_NODE_SIZE)
  return size * graphDensityScale(nodeCount) * userScale
}

function layoutIterations(nodeCount: number): number {
  if (nodeCount > 2500) return 28
  if (nodeCount > 1200) return 40
  if (nodeCount > 600) return 65
  if (nodeCount > 250) return 90
  return 140
}

function edgeVisibilityThreshold(nodeCount: number): number {
  if (nodeCount > 2500) return 0.16
  if (nodeCount > 1200) return 0.1
  if (nodeCount > 700) return 0.05
  return 0
}

function labelSizeThreshold(nodeCount: number): number {
  if (nodeCount > 2500) return 18
  if (nodeCount > 1200) return 14
  if (nodeCount > 600) return 10
  return 6
}

function labelDensity(nodeCount: number): number {
  if (nodeCount > 2500) return 0.08
  if (nodeCount > 1200) return 0.14
  if (nodeCount > 600) return 0.24
  return 0.4
}

function graphDataKey(nodes: readonly GraphNode[], edges: readonly GraphEdge[], graphSpacing: number): string {
  const nodeIds = nodes.map((n) => n.id).sort()
  const edgeIds = edges
    .map((e) => `${e.source}->${e.target}:${Math.round(e.weight * 1000)}`)
    .sort()
  return `${hashParts(nodeIds)}:${hashParts(edgeIds)}:${nodes.length}:${edges.length}:${graphSpacing.toFixed(2)}`
}

function hashParts(parts: readonly string[]): string {
  let hash = 2166136261
  for (const part of parts) {
    for (let i = 0; i < part.length; i++) {
      hash ^= part.charCodeAt(i)
      hash = Math.imul(hash, 16777619)
    }
    hash ^= 0xff
    hash = Math.imul(hash, 16777619)
  }
  return (hash >>> 0).toString(36)
}

function makeLayoutWorker(): Worker | null {
  try {
    return new Worker(new URL("./graph-layout-worker.ts", import.meta.url), { type: "module" })
  } catch (err) {
    console.warn("[Graph] failed to start layout worker; falling back to main-thread layout:", err)
    return null
  }
}

// --- Inner components ---

// Cache computed node positions so re-renders don't re-layout
const positionCache = new Map<string, { x: number; y: number }>()
let lastLayoutDataKey = ""
let pendingLayoutDataKey = ""

function GraphLoader({
  nodes,
  edges,
  colorMode,
  nodeScale,
  graphSpacing,
}: {
  nodes: GraphNode[]
  edges: GraphEdge[]
  colorMode: GraphColorMode
  nodeScale: number
  graphSpacing: number
}) {
  const loadGraph = useLoadGraph()
  const sigma = useSigma()

  useEffect(() => {
    const dataKey = graphDataKey(nodes, edges, graphSpacing)
    const needsLayout = dataKey !== lastLayoutDataKey && dataKey !== pendingLayoutDataKey
    let cancelled = false
    let worker: Worker | null = null

    const graph = new Graph()
    const maxLinks = Math.max(...nodes.map((n) => n.linkCount), 1)
    const weakEdgeThreshold = edgeVisibilityThreshold(nodes.length)

    for (const node of nodes) {
      const cached = positionCache.get(node.id)
      const color = colorMode === "community"
        ? COMMUNITY_COLORS[node.community % COMMUNITY_COLORS.length]
        : nodeColor(node.type, node.confirmation)
      // A matter says what a person decided of it, and its status.
      const state = node.type === "asunto" ? matterStateInWords(node) : ""
      graph.addNode(node.id, {
        type: "circle",
        x: cached?.x ?? Math.random() * 100,
        y: cached?.y ?? Math.random() * 100,
        size: nodeSize(node.linkCount, maxLinks, nodes.length, nodeScale),
        color,
        label: shortenLabel(node.label),
        fullLabel: state ? `${node.label} · ${state}` : node.label,
        nodeType: node.type,
        community: node.community,
      })
    }

    // Calculate max weight for normalization
    const maxWeight = Math.max(...edges.map((e) => e.weight), 1)

    for (const edge of edges) {
      if (graph.hasNode(edge.source) && graph.hasNode(edge.target)) {
        const edgeKey = `${edge.source}->${edge.target}`
        if (!graph.hasEdge(edgeKey) && !graph.hasEdge(`${edge.target}->${edge.source}`)) {
          const normalizedWeight = edge.weight / maxWeight // 0..1
          const size = 0.5 + normalizedWeight * 3.5 // 0.5..4
          // Stronger relationships → darker color
          const alpha = Math.round(40 + normalizedWeight * 180) // 40..220
          const color = `rgba(133,121,136,${alpha / 255})` // lilac grey with variable opacity
          graph.addEdgeWithKey(edgeKey, edge.source, edge.target, {
            color,
            size,
            weight: edge.weight,
            // The relations of the base it stands for (D-351), named on the canvas.
            label: edge.relations ? shortenLabel(Array.from(new Set(edge.relations.map((r) => r.label))).join(" · ")) : undefined,
            relations: edge.relations ?? [],
            normalizedWeight,
            sourceNode: edge.source,
            targetNode: edge.target,
            lowPriority: weakEdgeThreshold > 0 && normalizedWeight < weakEdgeThreshold,
          })
        }
      }
    }

    const runMainThreadLayout = () => {
      const settings = forceAtlas2.inferSettings(graph)
      forceAtlas2.assign(graph, {
        iterations: layoutIterations(nodes.length),
        settings: {
          ...settings,
          gravity: 1,
          scalingRatio: graphSpacing * (nodes.length > 400 ? 3 : 2),
          strongGravityMode: true,
          barnesHutOptimize: nodes.length > 50,
        },
      })
      lastLayoutDataKey = dataKey

      // Cache computed positions
      graph.forEachNode((nodeId, attrs) => {
        positionCache.set(nodeId, { x: attrs.x, y: attrs.y })
      })
    }

    // Only run expensive ForceAtlas2 layout when data actually changed.
    // Large graphs are laid out in a Web Worker so the UI remains
    // responsive while coordinates settle.
    if (needsLayout && nodes.length > 1 && nodes.length < WORKER_LAYOUT_NODE_THRESHOLD) {
      runMainThreadLayout()
    }

    loadGraph(graph)

    if (needsLayout && nodes.length >= WORKER_LAYOUT_NODE_THRESHOLD) {
      worker = makeLayoutWorker()
      if (!worker) {
        runMainThreadLayout()
        loadGraph(graph)
        return undefined
      }
      pendingLayoutDataKey = dataKey

      worker.onmessage = (event: MessageEvent<{ key: string; positions: Array<{ id: string; x: number; y: number }> }>) => {
        if (cancelled || event.data.key !== dataKey) return
        for (const { id, x, y } of event.data.positions) {
          if (!graph.hasNode(id)) continue
          graph.setNodeAttribute(id, "x", x)
          graph.setNodeAttribute(id, "y", y)
          positionCache.set(id, { x, y })
        }
        lastLayoutDataKey = dataKey
        if (pendingLayoutDataKey === dataKey) pendingLayoutDataKey = ""
        sigma.refresh()
      }
      worker.onerror = (event) => {
        if (cancelled) return
        console.warn("[Graph] layout worker failed; falling back to main-thread layout:", event.message)
        if (pendingLayoutDataKey === dataKey) pendingLayoutDataKey = ""
        runMainThreadLayout()
        loadGraph(graph)
      }
      worker.postMessage({
        key: dataKey,
        nodes: nodes.map((node) => {
          const cached = positionCache.get(node.id)
          return {
            id: node.id,
            x: cached?.x ?? graph.getNodeAttribute(node.id, "x"),
            y: cached?.y ?? graph.getNodeAttribute(node.id, "y"),
          }
        }),
        edges: edges.map((edge) => ({ source: edge.source, target: edge.target, weight: edge.weight })),
        iterations: layoutIterations(nodes.length),
        scalingRatio: graphSpacing * (nodes.length > 400 ? 3 : 2),
      })
    }

    return () => {
      cancelled = true
      if (pendingLayoutDataKey === dataKey) pendingLayoutDataKey = ""
      worker?.terminate()
    }
  }, [loadGraph, sigma, nodes, edges, colorMode, nodeScale, graphSpacing])

  return null
}

function GraphRenderSettings({
  hoverState,
  pinnedFocus,
  highlightedNodes,
  nodeCount,
  nodeScale,
  palette,
}: {
  hoverState: HoverState
  pinnedFocus: NodeFocus
  highlightedNodes: Set<string>
  nodeCount: number
  nodeScale: number
  palette: GraphThemePalette
}) {
  const sigma = useSigma()
  const setSettings = useSetSettings()

  useEffect(() => {
    const hasNode = (node: string) => sigma.getGraph().hasNode(node)
    setSettings({
      hideEdgesOnMove: true,
      hideLabelsOnMove: true,
      labelColor: { color: palette.label },
      edgeLabelColor: { color: palette.edgeLabel },
      edgeLabelSize: 11,
      labelDensity: labelDensity(nodeCount),
      labelRenderedSizeThreshold: hubLabelThreshold(
        labelSizeThreshold(nodeCount),
        nodeSize(HUB_LINK_SHARE, 1, nodeCount, nodeScale),
      ),
      renderEdgeLabels: true,
      defaultDrawNodeHover: createGraphNodeHoverRenderer(palette),
      nodeReducer: (node, attrs) => {
        const result = { ...attrs }
        const focus = resolveFocus(hoverState, pinnedFocus, hasNode)
        const hasHover = !!focus
        const hasHighlight = highlightedNodes.size > 0
        const isHoverNode = focus?.node === node
        const isHoverNeighbor = focus?.neighbors.has(node) ?? false
        const isHighlighted = highlightedNodes.has(node)

        if (isHighlighted) {
          result.size = (attrs.size ?? BASE_NODE_SIZE) * 1.5
          result.zIndex = 10
          result.forceLabel = true
        }
        if (isHoverNode) {
          result.size = (attrs.size ?? BASE_NODE_SIZE) * 1.4
          result.zIndex = 10
        }
        if (focus) {
          const focused = focusLabel(focus, node, String(attrs.label ?? ""), String(attrs.fullLabel ?? ""))
          result.label = focused.label
          if (focused.forceLabel) result.forceLabel = true
        }
        if ((hasHover && !isHoverNode && !isHoverNeighbor) || (hasHighlight && !isHighlighted)) {
          result.color = mixColor(attrs.color ?? "#A69CAA", palette.mutedNodeMixTarget, 0.75)
          result.label = ""
          result.size = (attrs.size ?? BASE_NODE_SIZE) * 0.6
        }
        return result
      },
      edgeReducer: (_edge, attrs) => {
        const result = { ...attrs }
        const source = String(attrs.sourceNode ?? "")
        const target = String(attrs.targetNode ?? "")
        const focus = resolveFocus(hoverState, pinnedFocus, hasNode)
        const hasHover = !!focus
        const hasHighlight = highlightedNodes.size > 0
        const hoverEdge = hasHover && (source === focus?.node || target === focus?.node)
        const highlightedEdge = hasHighlight && highlightedNodes.has(source) && highlightedNodes.has(target)

        if (attrs.lowPriority && !hoverEdge && !highlightedEdge) {
          result.hidden = true
          return result
        }
        if ((hasHover && !hoverEdge) || (hasHighlight && !highlightedEdge)) {
          result.color = palette.dimmedEdge
          result.size = 0.3
          result.label = ""
        }
        if (hoverEdge || highlightedEdge) {
          result.color = palette.activeEdge
          result.size = Math.max(2, (attrs.size ?? 1) * 1.5)
          // The relations of the node in focus are named.
          if (hoverEdge) result.forceLabel = true
        }
        return result
      },
    })
    sigma.refresh()
  }, [setSettings, sigma, hoverState, pinnedFocus, highlightedNodes, nodeCount, nodeScale, palette])

  return null
}

function EventHandler({
  onNodeClick,
  onNodeContextMenu,
  onHoverChange,
  onFocusChange,
  onEdgeDetail,
}: {
  onNodeClick: (nodeId: string) => void
  onNodeContextMenu: (nodeId: string, x: number, y: number) => void
  onHoverChange: (state: HoverState) => void
  onFocusChange: (focus: NodeFocus) => void
  onEdgeDetail: (detail: EdgeDetail | null, leaving?: boolean) => void
}) {
  const registerEvents = useRegisterEvents()
  const sigma = useSigma()

  useEffect(() => {
    registerEvents({
      clickNode: ({ node }) => {
        onFocusChange({ node, neighbors: new Set(sigma.getGraph().neighbors(node)) })
        onNodeClick(node)
      },
      clickStage: () => {
        onFocusChange(null)
        onEdgeDetail(null)
      },
      rightClickNode: (payload: SigmaNodeEventPayload) => {
        payload.preventSigmaDefault()
        payload.event.original.preventDefault()
        const point = clientPointFromEvent(payload.event.original)
        onNodeContextMenu(nodeIdFromPayload(payload), point.x, point.y)
      },
      rightClickStage: () => onNodeContextMenu("", 0, 0),
      enterNode: ({ node }) => {
        const container = sigma.getContainer()
        container.style.cursor = "pointer"
        const graph = sigma.getGraph()
        onHoverChange({ node, neighbors: new Set(graph.neighbors(node)) })
      },
      leaveNode: () => {
        const container = sigma.getContainer()
        container.style.cursor = "default"
        onHoverChange(null)
      },
      // Over an edge, or tapping it: its relations, with quote and state.
      enterEdge: ({ edge, event }) => {
        sigma.getContainer().style.cursor = "pointer"
        onEdgeDetail(edgeDetailOf(sigma.getGraph(), edge, event.x, event.y, false))
      },
      leaveEdge: () => {
        sigma.getContainer().style.cursor = "default"
        onEdgeDetail(null, true)
      },
      clickEdge: ({ edge, event }) => {
        onEdgeDetail(edgeDetailOf(sigma.getGraph(), edge, event.x, event.y, true))
      },
    })
  }, [registerEvents, sigma, onNodeClick, onNodeContextMenu, onHoverChange, onFocusChange, onEdgeDetail])

  return null
}

function edgeDetailOf(graph: Graph, edge: string, x: number, y: number, pinned: boolean): EdgeDetail {
  return {
    source: String(graph.getEdgeAttribute(edge, "sourceNode") ?? graph.source(edge)),
    target: String(graph.getEdgeAttribute(edge, "targetNode") ?? graph.target(edge)),
    relations: (graph.getEdgeAttribute(edge, "relations") as GraphRelation[] | undefined) ?? [],
    x,
    y,
    pinned,
  }
}

function nodeIdFromPayload(payload: SigmaNodeEventPayload): string {
  return payload.node
}

function clientPointFromEvent(event: MouseEvent | TouchEvent): { x: number; y: number } {
  if ("clientX" in event) return { x: event.clientX, y: event.clientY }
  const touch = event.touches[0] ?? event.changedTouches[0]
  return { x: touch?.clientX ?? 0, y: touch?.clientY ?? 0 }
}

function ZoomControls() {
  const sigma = useSigma()

  return (
    <div className="absolute top-3 right-3 flex flex-col gap-1">
      <Button
        variant="outline"
        size="icon"
        className="h-7 w-7 bg-background/80 backdrop-blur-sm"
        onClick={() => {
          const camera = sigma.getCamera()
          camera.animatedZoom({ duration: 200 })
        }}
      >
        <ZoomIn className="h-3.5 w-3.5" />
      </Button>
      <Button
        variant="outline"
        size="icon"
        className="h-7 w-7 bg-background/80 backdrop-blur-sm"
        onClick={() => {
          const camera = sigma.getCamera()
          camera.animatedUnzoom({ duration: 200 })
        }}
      >
        <ZoomOut className="h-3.5 w-3.5" />
      </Button>
      <Button
        variant="outline"
        size="icon"
        className="h-7 w-7 bg-background/80 backdrop-blur-sm"
        onClick={() => {
          const camera = sigma.getCamera()
          camera.animatedReset({ duration: 300 })
        }}
      >
        <Maximize className="h-3.5 w-3.5" />
      </Button>
    </div>
  )
}

// --- Main component ---

export function GraphView() {
  const dataVersion = useWikiStore((s) => s.dataVersion)
  const isDarkMode = useResolvedDarkMode()
  const graphPalette = useMemo(() => graphThemePalette(isDarkMode), [isDarkMode])
  const drawNodeHover = useMemo(() => createGraphNodeHoverRenderer(graphPalette), [graphPalette])

  const [nodes, setNodes] = useState<GraphNode[]>([])
  const [edges, setEdges] = useState<GraphEdge[]>([])
  const [communities, setCommunities] = useState<CommunityInfo[]>([])
  const [surprisingConns, setSurprisingConns] = useState<SurprisingConnection[]>([])
  const [knowledgeGaps, setKnowledgeGaps] = useState<KnowledgeGap[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [hoveredType, setHoveredType] = useState<string | null>(null)
  const graphUiState = useWikiStore((s) => s.graphUiState)
  const setGraphUiState = useWikiStore((s) => s.setGraphUiState)
  const resetGraphUiState = useWikiStore((s) => s.resetGraphUiState)
  const colorMode = graphUiState.colorMode
  const filters = graphUiState.filters
  const nodeScale = graphUiState.nodeScale
  const graphSpacingDraft = graphUiState.graphSpacingDraft
  const [showInsights, setShowInsights] = useState(false)
  const [highlightedNodes, setHighlightedNodes] = useState<Set<string>>(new Set())
  const [hoverState, setHoverState] = useState<HoverState>(null)
  const [pinnedFocus, setPinnedFocus] = useState<NodeFocus>(null)
  const [dismissedInsights, setDismissedInsights] = useState<Set<string>>(new Set())
  const [sigmaKey, setSigmaKey] = useState(0)
  const [isResizing, setIsResizing] = useState(false)
  const [legendCollapsed, setLegendCollapsed] = useState(false)
  const [showFilters, setShowFilters] = useState(false)
  const [graphSearchOpen, setGraphSearchOpen] = useState(false)
  const [graphSearch, setGraphSearch] = useState("")
  const [graphSpacing, setGraphSpacing] = useState(graphSpacingDraft)
  const [edgeDetail, setEdgeDetail] = useState<EdgeDetail | null>(null)
  const [nodeMenu, setNodeMenu] = useState<{ nodeId: string; x: number; y: number } | null>(null)
  const graphContainerRef = useRef<HTMLDivElement>(null)
  const researchDialogTokenRef = useRef(0)
  const graphSearchInputRef = useRef<HTMLInputElement>(null)

  const setColorMode = useCallback((colorMode: GraphColorMode) => {
    setGraphUiState((prev) => ({ ...prev, colorMode }))
  }, [setGraphUiState])

  const setFilters = useCallback((next: SetStateAction<GraphFilterState>) => {
    setGraphUiState((prev) => ({
      ...prev,
      filters: typeof next === "function" ? next(prev.filters) : next,
    }))
  }, [setGraphUiState])

  const setNodeScale = useCallback((nodeScale: number) => {
    setGraphUiState((prev) => ({ ...prev, nodeScale }))
  }, [setGraphUiState])

  const setGraphSpacingDraft = useCallback((graphSpacingDraft: number) => {
    setGraphUiState((prev) => ({ ...prev, graphSpacingDraft }))
  }, [setGraphUiState])

  // Research confirmation dialog
  const [researchDialog, setResearchDialog] = useState<{
    loading: boolean
    topic: string
    queries: string[]
    dismissKey?: string
    /** The person, organization or matter of the gap: the research is kept of it (P-236). */
    target?: ResearchTarget
  } | null>(null)
  const lastLoadedVersion = useRef(-1)
  const graphLoadRequest = useRef(0)

  const loadGraph = useCallback(async () => {
    const requestId = ++graphLoadRequest.current
    setLoading(true)
    setError(null)
    try {
      const result = await buildBaseGraph(useWikiStore.getState().dataVersion)
      if (requestId !== graphLoadRequest.current) return
      setNodes(result.nodes)
      setEdges(result.edges)
      setCommunities(result.communities)
      setSurprisingConns(findSurprisingConnections(result.nodes, result.edges, result.communities))
      setKnowledgeGaps(detectKnowledgeGaps(result.nodes, result.edges, result.communities))
      lastLoadedVersion.current = useWikiStore.getState().dataVersion
    } catch (err) {
      if (requestId !== graphLoadRequest.current) return
      const message = err instanceof Error ? err.message : String(err)
      setError(message)
    } finally {
      if (requestId === graphLoadRequest.current) setLoading(false)
    }
  }, [])

  // Spacing changes trigger ForceAtlas2 layout through GraphLoader's dataKey.
  // Keep the slider responsive while debouncing the expensive relayout.
  useEffect(() => {
    const timer = window.setTimeout(() => setGraphSpacing(graphSpacingDraft), GRAPH_SPACING_DEBOUNCE_MS)
    return () => window.clearTimeout(timer)
  }, [graphSpacingDraft])

  useEffect(() => {
    if (dataVersion !== lastLoadedVersion.current) {
      loadGraph()
    }
  }, [loadGraph, dataVersion])

  useEffect(() => {
    if (!graphSearchOpen) return
    const id = window.requestAnimationFrame(() => graphSearchInputRef.current?.focus())
    return () => window.cancelAnimationFrame(id)
  }, [graphSearchOpen])

  // A node opens the screen of its object: its person or organization,
  // its matter or its document (D-410).
  const handleNodeClick = useCallback(
    (nodeId: string) => {
      const node = nodes.find((n) => n.id === nodeId)
      if (node) openPageObject(objectOfNode(node))
    },
    [nodes],
  )

  const handleEdgeDetail = useCallback((detail: EdgeDetail | null, leaving?: boolean) => {
    // Leaving an edge keeps the one a person tapped.
    setEdgeDetail((current) => (leaving && current?.pinned ? current : detail))
  }, [])

  const handleNodeContextMenu = useCallback((nodeId: string, x: number, y: number) => {
    if (!nodeId) {
      setNodeMenu(null)
      return
    }
    const rect = graphContainerRef.current?.getBoundingClientRect()
    setNodeMenu({
      nodeId,
      x: rect ? x - rect.left : x,
      y: rect ? y - rect.top : y,
    })
  }, [])

  const resetFilters = useCallback(() => {
    resetGraphUiState()
    setGraphSpacing(DEFAULT_GRAPH_SPACING)
    setNodeMenu(null)
  }, [resetGraphUiState])

  const knowledgeGapKey = useCallback((gap: KnowledgeGap) => (
    `gap:${gap.type}:${gap.title}:${gap.nodeIds.join(",")}`
  ), [])

  const visibleKnowledgeGaps = useMemo(
    () => knowledgeGaps.filter((gap) => !dismissedInsights.has(knowledgeGapKey(gap))),
    [dismissedInsights, knowledgeGaps, knowledgeGapKey],
  )

  const dismissInsight = useCallback((key: string, ids?: Set<string>) => {
    setDismissedInsights((prev) => new Set([...prev, key]))
    if (ids && highlightedNodes.size === ids.size && [...ids].every((id) => highlightedNodes.has(id))) {
      setHighlightedNodes(new Set())
    }
  }, [highlightedNodes])

  const handleResearchClick = useCallback(async (gap: KnowledgeGap, dismissKey?: string) => {
    const store = useWikiStore.getState()
    const token = researchDialogTokenRef.current + 1
    researchDialogTokenRef.current = token
    const { term: gapTerm, context: gapContext } = researchSeedForKnowledgeGap(gap, nodes)
    // A research from a node of a person, organization or matter is kept of
    // that object; of a document or a group of nodes, it is a loose one.
    const node = researchNodeForKnowledgeGap(gap, nodes)
    const object = node ? objectOfNode(node) : null
    const target: ResearchTarget | undefined = node && object && (object.kind === "entity" || object.kind === "matter")
      ? { objectId: object.objectId, name: node.label }
      : undefined

    // Show loading state
    setResearchDialog({ loading: true, topic: "", queries: [], dismissKey, target })

    try {
      // The overview of the archive, from the base, for context.
      let overview = ""
      try { overview = (await getKnowledgeOverview())?.body ?? "" } catch {}

      const result = await optimizeResearchTopic(
        store.llmConfig,
        gapTerm,
        gapContext,
        gap.type,
        overview,
        "",
      )
      if (researchDialogTokenRef.current !== token) return
      setResearchDialog({ loading: false, topic: result.topic, queries: result.searchQueries, dismissKey, target })
    } catch {
      if (researchDialogTokenRef.current !== token) return
      setResearchDialog({
        loading: false,
        topic: gapTerm,
        queries: [groundedFallbackResearchQuery(gapTerm, gapContext)],
        dismissKey,
        target,
      })
    }
  }, [nodes])

  const handleResearchConfirm = useCallback(() => {
    if (!researchDialog) return
    const store = useWikiStore.getState()
    queueResearch(
      researchDialog.topic,
      store.llmConfig,
      store.searchApiConfig,
      { searchQueries: researchDialog.queries, target: researchDialog.target },
    )
    if (researchDialog.dismissKey) {
      setDismissedInsights((prev) => new Set([...prev, researchDialog.dismissKey!]))
      setHighlightedNodes(new Set())
    }
    setResearchDialog(null)
  }, [researchDialog])

  // Unmount sigma when panels resize or toggle to prevent WebGL crash.
  // Sigma crashes with "could not find suitable program for node type circle"
  // when its canvas is resized by external layout changes.

  // 1. Detect panel open/close (local graph preview, researchPanel, insights)
  const researchPanelForLayout = useResearchStore((s) => s.panelOpen)
  const layoutKey = `${researchPanelForLayout}-${showInsights}`
  const prevLayoutKey = useRef(layoutKey)

  useEffect(() => {
    if (prevLayoutKey.current !== layoutKey) {
      prevLayoutKey.current = layoutKey
      setIsResizing(true)
      const timer = setTimeout(() => {
        setSigmaKey((k) => k + 1)
        setIsResizing(false)
      }, 100)
      return () => clearTimeout(timer)
    }
  }, [layoutKey])

  // 2. Detect panel drag resize via data-panel-resizing attribute on body
  useEffect(() => {
    const observer = new MutationObserver(() => {
      const dragging = document.body.dataset.panelResizing === "true"
      if (dragging && !isResizing) {
        setIsResizing(true)
      }
      if (!dragging && isResizing) {
        // Drag ended — remount sigma after a tick
        setTimeout(() => {
          setSigmaKey((k) => k + 1)
          setIsResizing(false)
        }, 50)
      }
    })
    observer.observe(document.body, { attributes: true, attributeFilter: ["data-panel-resizing"] })
    return () => observer.disconnect()
  }, [isResizing])

  // Count nodes by type for legend
  const typeCounts = nodes.reduce<Record<string, number>>((acc, n) => {
    acc[n.type] = (acc[n.type] ?? 0) + 1
    return acc
  }, {})
  // What each kind of node is called, in the vocabulary's words; the
  // kinds of the base first, any other kind of entity after.
  const nodeTypeLabelMap = useMemo(() => {
    const words = vocabulary()
    const labels: Record<string, string> = {}
    for (const type of [...GRAPH_NODE_TYPES, ...Object.keys(typeCounts).sort()]) {
      labels[type] ??= nodeTypeName(type, true, words)
    }
    return labels
  }, [typeCounts])
  const matterCounts = useMemo(() => {
    const counts: Record<string, number> = {}
    for (const node of nodes) {
      if (node.type === "asunto" && node.confirmation) counts[node.confirmation] = (counts[node.confirmation] ?? 0) + 1
    }
    return counts
  }, [nodes])
  const matterStates = vocabulary().knowledge.matterStates
  const confirmationLabels: [string, string][] = [
    ["confirmed", matterStates.confirmed],
    ["proposed", matterStates.proposed],
  ]
  const toggleConfirmation = useCallback((confirmation: string) => {
    setFilters((prev) => {
      const next = new Set(prev.hiddenConfirmations)
      if (next.has(confirmation)) next.delete(confirmation)
      else next.add(confirmation)
      return { ...prev, hiddenConfirmations: next }
    })
  }, [setFilters])
  const nodeLabel = useCallback((id: string) => nodes.find((n) => n.id === id)?.label ?? id, [nodes])

  const filteredGraph = useMemo(
    () => applyGraphFilters(nodes, edges, filters),
    [nodes, edges, filters],
  )
  const searchedGraph = useMemo(
    () => applyGraphSearch(filteredGraph.nodes, filteredGraph.edges, graphSearch),
    [filteredGraph.nodes, filteredGraph.edges, graphSearch],
  )
  const searchActive = graphSearch.trim().length > 0
  const hiddenCount = nodes.length - filteredGraph.nodes.length
  const filtersActive = hasActiveGraphFilters(filters)
  const contextNode = nodeMenu ? nodes.find((node) => node.id === nodeMenu.nodeId) : null

  if (loading) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 text-muted-foreground">
        <RefreshCw className="h-8 w-8 animate-spin opacity-50" />
        <p className="text-sm">{TEXT.buildingGraph}</p>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 text-muted-foreground">
        <Network className="h-10 w-10 opacity-30" />
        <p className="text-sm text-destructive">{error}</p>
        <Button variant="outline" size="sm" onClick={loadGraph}>{TEXT.retry}</Button>
      </div>
    )
  }

  if (!loading && nodes.length === 0 && !error) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 text-muted-foreground">
        <Network className="h-10 w-10 opacity-30" />
        <p className="text-sm">{TEXT.noPages}</p>
        <p className="text-xs">{TEXT.importSourcesHint}</p>
      </div>
    )
  }

  return (
    <div className="relative flex h-full flex-col">
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between gap-2 border-b px-4 py-2 shrink-0">
        <div className="flex min-w-0 flex-wrap items-center gap-3">
          <div className="flex items-center gap-2">
            <Network className="h-4 w-4 text-brand-ink" />
            <span className="font-ui text-base font-bold text-strong">{TEXT.knowledgeGraph}</span>
          </div>
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <span className="rounded bg-muted px-1.5 py-0.5">{searchedGraph.nodes.length}/{nodes.length} {TEXT.pages}</span>
            <span className="rounded bg-muted px-1.5 py-0.5">{searchedGraph.edges.length}/{edges.length} {TEXT.links}</span>
            {hiddenCount > 0 && (
              <span className="rounded bg-warn-bg px-1.5 py-0.5 text-warn">
                {hiddenCount} {TEXT.hidden}
              </span>
            )}
          </div>
        </div>
        <div className="flex min-w-0 flex-wrap items-center justify-end gap-1">
          {graphSearchOpen || searchActive ? (
            <div className="relative mr-1 w-52 max-w-full">
              <Search className="pointer-events-none absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
              <input
                ref={graphSearchInputRef}
                value={graphSearch}
                onChange={(e) => setGraphSearch(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Escape") {
                    setGraphSearch("")
                    setGraphSearchOpen(false)
                  }
                }}
                className="h-7 w-full rounded-md border bg-background pl-7 pr-7 text-xs outline-none placeholder:text-muted-foreground focus:border-ring"
                placeholder={TEXT.searchPlaceholder}
                aria-label={TEXT.searchLabel}
              />
              <button
                type="button"
                className="absolute right-1.5 top-1/2 -translate-y-1/2 rounded p-0.5 text-muted-foreground hover:bg-accent hover:text-foreground"
                onClick={() => {
                  if (searchActive) {
                    setGraphSearch("")
                    return
                  }
                  setGraphSearchOpen(false)
                }}
                aria-label={searchActive ? TEXT.clearSearch : TEXT.closeSearch}
                title={searchActive ? TEXT.clearSearch : TEXT.closeSearch}
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
          ) : (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setGraphSearchOpen(true)}
              className="text-xs gap-1 h-7"
              aria-label={TEXT.searchLabel}
              title={TEXT.searchLabel}
            >
              <Search className="h-3.5 w-3.5" />
            </Button>
          )}
          <Button
            variant={showFilters ? "secondary" : "ghost"}
            size="sm"
            onClick={() => setShowFilters((v) => !v)}
            className="text-xs gap-1 h-7"
          >
            <Filter className="h-3 w-3" />
            {TEXT.filter}
          </Button>
          {filtersActive && (
            <Button
              variant="ghost"
              size="sm"
              onClick={resetFilters}
              className="text-xs gap-1 h-7"
              title={TEXT.resetFilters}
            >
              <RotateCcw className="h-3 w-3" />
              {TEXT.reset}
            </Button>
          )}
          <Button
            variant={colorMode === "type" ? "secondary" : "ghost"}
            size="sm"
            onClick={() => setColorMode("type")}
            className="text-xs gap-1 h-7"
          >
            <Tag className="h-3 w-3" />
            {TEXT.type}
          </Button>
          <Button
            variant={colorMode === "community" ? "secondary" : "ghost"}
            size="sm"
            onClick={() => setColorMode("community")}
            className="text-xs gap-1 h-7"
          >
            <Layers className="h-3 w-3" />
            {TEXT.community}
          </Button>
          {(surprisingConns.filter((c) => !dismissedInsights.has(c.key)).length > 0 || visibleKnowledgeGaps.length > 0) && (
            <Button
              variant={showInsights ? "secondary" : "ghost"}
              size="sm"
              onClick={() => {
                setShowInsights((v) => {
                  if (v) setHighlightedNodes(new Set())
                  return !v
                })
              }}
              className="text-xs gap-1 h-7"
            >
              <Lightbulb className="h-3 w-3" />
              {TEXT.insights}
              <span className="rounded bg-muted px-1 text-[10px]">
                {surprisingConns.filter((c) => !dismissedInsights.has(c.key)).length + visibleKnowledgeGaps.length}
              </span>
            </Button>
          )}
          <Button variant="ghost" size="sm" onClick={loadGraph} className="text-xs gap-1 h-7">
            <RefreshCw className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {/* Graph canvas + Insights side panel */}
      <div className="flex flex-1 min-h-0">
        {/* Graph canvas */}
        <div
          ref={graphContainerRef}
          className="relative flex-1 min-w-0 overflow-hidden bg-background"
          onContextMenu={(e) => e.preventDefault()}
          onClick={() => setNodeMenu(null)}
        >
          {isResizing ? (
            <div className="flex h-full items-center justify-center text-xs text-muted-foreground">
              {TEXT.resizing}
            </div>
          ) : (
            <>
              <ErrorBoundary>
                <SigmaContainer
                  key={sigmaKey}
                  style={{ width: "100%", height: "100%", background: "transparent" }}
                  settings={{
                    defaultNodeType: "circle",
                    renderEdgeLabels: true,
                    enableEdgeEvents: true,
                    hideEdgesOnMove: true,
                    hideLabelsOnMove: true,
                    defaultEdgeColor: graphPalette.defaultEdge,
                    defaultNodeColor: "#A69CAA",
                    labelSize: 13,
                    labelWeight: "bold",
                    labelColor: { color: graphPalette.label },
                    defaultDrawNodeHover: drawNodeHover,
                    stagePadding: 30,
                  }}
                >
                  <GraphLoader
                    nodes={searchedGraph.nodes}
                    edges={searchedGraph.edges}
                    colorMode={colorMode}
                    nodeScale={nodeScale}
                    graphSpacing={graphSpacing}
                  />
                  <EventHandler
                    onNodeClick={handleNodeClick}
                    onNodeContextMenu={handleNodeContextMenu}
                    onHoverChange={setHoverState}
                    onFocusChange={setPinnedFocus}
                    onEdgeDetail={handleEdgeDetail}
                  />
                  <GraphRenderSettings
                    hoverState={hoverState}
                    pinnedFocus={pinnedFocus}
                    highlightedNodes={searchActive ? searchedGraph.matchedNodeIds : highlightedNodes}
                    nodeCount={searchedGraph.nodes.length}
                    nodeScale={nodeScale}
                    palette={graphPalette}
                  />
                  <ZoomControls />
                </SigmaContainer>
              </ErrorBoundary>

              {searchedGraph.nodes.length === 0 && (
                <div className="pointer-events-none absolute inset-0 z-10 flex flex-col items-center justify-center gap-2 bg-background/85 text-muted-foreground backdrop-blur-[1px]">
                  <Search className="h-8 w-8 opacity-40" />
                  <p className="text-sm">{searchActive ? TEXT.noSearchResults : TEXT.noVisibleNodes}</p>
                  {searchActive && (
                    <Button
                      variant="outline"
                      size="sm"
                      className="pointer-events-auto"
                      onClick={() => setGraphSearch("")}
                    >
                      {TEXT.clearSearch}
                    </Button>
                  )}
                </div>
              )}
            </>
          )}

          {edgeDetail && edgeDetail.relations.length > 0 && (
            <div
              className="absolute z-20 max-w-xs rounded-md border bg-background/95 px-3 py-2 text-xs shadow-lg backdrop-blur-sm"
              style={{ left: edgeDetail.x + 12, top: edgeDetail.y + 12 }}
              onClick={(e) => e.stopPropagation()}
            >
              <div className="mb-1 flex items-start justify-between gap-2">
                <span className="font-medium text-foreground">
                  {nodeLabel(edgeDetail.source)} — {nodeLabel(edgeDetail.target)}
                </span>
                {edgeDetail.pinned && (
                  <button
                    type="button"
                    className="shrink-0 rounded p-0.5 text-muted-foreground hover:bg-accent"
                    onClick={() => setEdgeDetail(null)}
                    aria-label={TEXT.close}
                    title={TEXT.close}
                  >
                    <X className="h-3 w-3" />
                  </button>
                )}
              </div>
              <ul className="space-y-1">
                {edgeDetail.relations.map((relation, index) => (
                  <li key={`${relation.family}-${relation.kind}-${index}`}>
                    <span className="text-foreground">{relation.label}</span>
                    <span className="text-muted-foreground"> · {relationStateInWords(relation)}</span>
                    {relation.quote && <div className="italic text-muted-foreground">«{relation.quote}»</div>}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {showFilters && (
            <div className="absolute top-3 left-3 w-72 rounded-lg border bg-background/95 p-3 text-xs shadow-lg backdrop-blur-sm">
              <div className="mb-3 flex items-center justify-between">
                <div className="flex items-center gap-1.5 font-semibold text-foreground">
                  <Filter className="h-3.5 w-3.5" />
                  {TEXT.graphFilters}
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 px-1.5 text-[10px]"
                  onClick={resetFilters}
                >
                  {TEXT.reset}
                </Button>
              </div>

              <div className="space-y-3">
                <div className="space-y-1.5">
                  <div className="font-medium text-muted-foreground">{TEXT.quickFilters}</div>
                  <label className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      checked={filters.hideIsolated}
                      onChange={(e) => setFilters((prev) => ({ ...prev, hideIsolated: e.target.checked }))}
                    />
                    <span>{TEXT.hideIsolated}</span>
                  </label>
                </div>

                <div className="space-y-1.5">
                  <div className="font-medium text-muted-foreground">{TEXT.minLinks}</div>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      min={0}
                      className="h-7 w-20 rounded border bg-background px-2 text-xs"
                      value={filters.minLinks ?? ""}
                      onChange={(e) => {
                        const raw = e.target.value.trim()
                        const value = Number(raw)
                        setFilters((prev) => ({
                          ...prev,
                          minLinks: raw === "" || !Number.isFinite(value) ? undefined : Math.max(0, value),
                        }))
                      }}
                      placeholder={TEXT.any}
                    />
                    <span className="text-muted-foreground">{TEXT.minLinksHint}</span>
                  </div>
                </div>

                <div className="space-y-1.5">
                  <div className="font-medium text-muted-foreground">{TEXT.maxLinks}</div>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      min={0}
                      className="h-7 w-20 rounded border bg-background px-2 text-xs"
                      value={filters.maxLinks ?? ""}
                      onChange={(e) => {
                        const raw = e.target.value.trim()
                        const value = Number(raw)
                        setFilters((prev) => ({
                          ...prev,
                          maxLinks: raw === "" || !Number.isFinite(value) ? undefined : Math.max(0, value),
                        }))
                      }}
                      placeholder={TEXT.any}
                    />
                    <span className="text-muted-foreground">{TEXT.maxLinksHint}</span>
                  </div>
                </div>

                <div className="space-y-2">
                  <div className="font-medium text-muted-foreground">{TEXT.displayTuning}</div>
                  <label className="block space-y-1">
                    <div className="flex items-center justify-between">
                      <span>{TEXT.nodeSize}</span>
                      <span className="text-muted-foreground">{Math.round(nodeScale * 100)}%</span>
                    </div>
                    <input
                      type="range"
                      min={0.5}
                      max={1.5}
                      step={0.05}
                      value={nodeScale}
                      onChange={(e) => setNodeScale(Number(e.target.value))}
                      className="w-full"
                    />
                  </label>
                  <label className="block space-y-1">
                    <div className="flex items-center justify-between">
                      <span>{TEXT.spacing}</span>
                      <span className="text-muted-foreground">{Math.round(graphSpacingDraft * 100)}%</span>
                    </div>
                    <input
                      type="range"
                      min={0.6}
                      max={2.2}
                      step={0.05}
                      value={graphSpacingDraft}
                      onChange={(e) => setGraphSpacingDraft(Number(e.target.value))}
                      className="w-full"
                    />
                  </label>
                  <p className="text-[11px] leading-relaxed text-muted-foreground">
                    {TEXT.displayTuningHint}
                  </p>
                </div>

                <div className="space-y-1.5">
                  <div className="font-medium text-muted-foreground">{TEXT.nodeTypes}</div>
                  <div className="grid grid-cols-2 gap-1">
                    {Object.entries(nodeTypeLabelMap)
                      .filter(([type]) => (typeCounts[type] ?? 0) > 0)
                      .map(([type, label]) => (
                        <label key={type} className="flex min-w-0 items-center gap-1.5">
                          <input
                            type="checkbox"
                            checked={!filters.hiddenTypes.has(type)}
                            onChange={(e) => {
                              setFilters((prev) => {
                                const next = new Set(prev.hiddenTypes)
                                if (e.target.checked) next.delete(type)
                                else next.add(type)
                                return { ...prev, hiddenTypes: next }
                              })
                            }}
                          />
                          <span className="truncate">{label}</span>
                          <span className="text-muted-foreground/60">{typeCounts[type]}</span>
                        </label>
                      ))}
                  </div>
                </div>

                {(matterCounts.proposed ?? 0) + (matterCounts.confirmed ?? 0) > 0 && (
                  <div className="space-y-1.5">
                    <div className="font-medium text-muted-foreground">{vocabulary().knowledge.matters}</div>
                    <div className="grid grid-cols-2 gap-1">
                      {confirmationLabels.map(([confirmation, label]) => (
                        <label key={confirmation} className="flex min-w-0 items-center gap-1.5">
                          <input
                            type="checkbox"
                            checked={!filters.hiddenConfirmations.has(confirmation)}
                            onChange={() => toggleConfirmation(confirmation)}
                          />
                          <span className="truncate">{label}</span>
                          <span className="text-muted-foreground/60">{matterCounts[confirmation] ?? 0}</span>
                        </label>
                      ))}
                    </div>
                  </div>
                )}

                {filters.hiddenNodeIds.size > 0 && (
                  <div className="space-y-1.5">
                    <div className="font-medium text-muted-foreground">{TEXT.hiddenNodes}</div>
                    <div className="max-h-24 space-y-1 overflow-y-auto">
                      {[...filters.hiddenNodeIds].map((nodeId) => {
                        const node = nodes.find((n) => n.id === nodeId)
                        return (
                          <div key={nodeId} className="flex items-center justify-between gap-2 rounded bg-muted/50 px-2 py-1">
                            <span className="truncate">{node?.label ?? nodeId}</span>
                            <button
                              type="button"
                              className="text-muted-foreground hover:text-foreground"
                              onClick={() => setFilters((prev) => {
                                const next = new Set(prev.hiddenNodeIds)
                                next.delete(nodeId)
                                return { ...prev, hiddenNodeIds: next }
                              })}
                            >
                              {TEXT.show}
                            </button>
                          </div>
                        )
                      })}
                    </div>
                  </div>
                )}

                <div className="rounded bg-muted/50 px-2 py-1.5 text-muted-foreground">
                  {`Se ven ${filteredGraph.nodes.length} de ${nodes.length} nodos y ${filteredGraph.edges.length} de ${edges.length} relaciones.`}
                </div>
              </div>
            </div>
          )}

          {nodeMenu && contextNode && (
            <div
              className="absolute z-20 w-48 rounded-md border bg-background py-1 text-xs shadow-lg"
              style={{ left: nodeMenu.x, top: nodeMenu.y }}
              onClick={(e) => e.stopPropagation()}
            >
              <div className="border-b px-3 py-2">
                <div className="truncate font-medium text-foreground">{contextNode.label}</div>
                <div className="text-muted-foreground">{nodeTypeName(contextNode.type)} · {contextNode.linkCount} {TEXT.links}</div>
              </div>
              <button
                type="button"
                className="flex w-full items-center gap-2 px-3 py-2 text-left hover:bg-accent"
                onClick={() => {
                  setFilters((prev) => ({
                    ...prev,
                    hiddenNodeIds: new Set([...prev.hiddenNodeIds, contextNode.id]),
                  }))
                  setNodeMenu(null)
                }}
              >
                <EyeOff className="h-3.5 w-3.5" />
                {TEXT.hideThisNode}
              </button>
            </div>
          )}

          {/* Legend */}
          <div className="absolute bottom-3 left-3 rounded-lg border bg-background/90 backdrop-blur-sm px-3 py-2 text-xs shadow-sm max-w-[260px]">
            <div className="flex items-center justify-between mb-1.5">
              <span className="font-semibold text-foreground">
                {colorMode === "type" ? TEXT.nodeTypesLabel : TEXT.communitiesLabel}
              </span>
              <div className="flex items-center gap-1">
                {colorMode === "type" && (filters.hiddenTypes.size > 0 || filters.hiddenConfirmations.size > 0) && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-6 text-[10px] px-1"
                    onClick={() => setFilters((prev) => ({ ...prev, hiddenTypes: new Set(), hiddenConfirmations: new Set() }))}
                    title={TEXT.showAllTypes}
                  >
                    {TEXT.showAll}
                  </Button>
                )}
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 w-6 p-0"
                  onClick={() => setLegendCollapsed(!legendCollapsed)}
                  title={legendCollapsed ? TEXT.expandLegend : TEXT.collapseLegend}
                >
                  {legendCollapsed ? "▶" : "▼"}
                </Button>
              </div>
            </div>
            {!legendCollapsed && (
              colorMode === "type" ? (
                <div className="flex flex-col gap-0.5 max-h-48 overflow-y-auto legend-scroll" style={{ direction: "rtl" }}>
                  <div className="flex flex-col gap-0.5" style={{ direction: "ltr" }}>
                    {Object.entries(nodeTypeLabelMap)
                      .filter(([type]) => (typeCounts[type] ?? 0) > 0)
                      .flatMap(([type, label]) => {
                        // A matter's entry is two: confirmed and proposed, by color.
                        if (type === "asunto") {
                          return confirmationLabels
                            .filter(([confirmation]) => (matterCounts[confirmation] ?? 0) > 0)
                            .map(([confirmation, state]) => ({ key: `asunto-${confirmation}`, type, confirmation, label: `${label} · ${state.toLowerCase()}`, count: matterCounts[confirmation] ?? 0 }))
                        }
                        return [{ key: type, type, confirmation: null as string | null, label, count: typeCounts[type] ?? 0 }]
                      })
                      .map(({ key, type, confirmation, label, count }) => {
                        const isHidden = filters.hiddenTypes.has(type) || (confirmation !== null && filters.hiddenConfirmations.has(confirmation))
                        const swatch = isHidden ? "#A69CAA" : nodeColor(type, confirmation)
                        return (
                          <div
                            key={key}
                            className={`flex items-center gap-2 rounded px-1 py-0.5 transition-colors hover:bg-accent/50 ${isHidden ? "opacity-40" : ""}`}
                            onMouseEnter={() => setHoveredType(key)}
                            onMouseLeave={() => setHoveredType(null)}
                            onDoubleClick={() => {
                              if (confirmation !== null) {
                                toggleConfirmation(confirmation)
                                return
                              }
                              setFilters((prev) => {
                                const next = new Set(prev.hiddenTypes)
                                if (next.has(type)) {
                                  next.delete(type)
                                } else {
                                  next.add(type)
                                }
                                return { ...prev, hiddenTypes: next }
                              })
                            }}
                            title={TEXT.doubleClickToggle}
                          >
                            <span
                              className="inline-block h-3 w-3 rounded-full shrink-0 shadow-sm"
                              style={{
                                backgroundColor: swatch,
                                boxShadow: `0 0 4px ${hexToRgba(swatch, 0.4)}`,
                              }}
                            />
                            <span className={hoveredType === key ? "text-foreground font-medium" : "text-muted-foreground"}>
                              {label}
                            </span>
                            <span className="text-muted-foreground/60 ml-auto">{count}</span>
                            {isHidden && <span className="text-muted-foreground/60 text-[10px]">{TEXT.hidden}</span>}
                          </div>
                        )
                      })}
                  </div>
                </div>
              ) : (
                <div className="flex flex-col gap-0.5 max-h-48 overflow-y-auto legend-scroll" style={{ direction: "rtl" }}>
                  <div className="flex flex-col gap-0.5" style={{ direction: "ltr" }}>
                    {communities.map((c) => (
                    <div
                      key={c.id}
                      className="flex items-center gap-2 rounded px-1 py-0.5 transition-colors hover:bg-accent/50"
                    >
                      <span
                        className="inline-block h-3 w-3 rounded-full shrink-0 shadow-sm"
                        style={{
                          backgroundColor: COMMUNITY_COLORS[c.id % COMMUNITY_COLORS.length],
                          boxShadow: `0 0 4px ${hexToRgba(COMMUNITY_COLORS[c.id % COMMUNITY_COLORS.length], 0.4)}`,
                        }}
                      />
                      <span className="text-muted-foreground truncate" title={c.topNodes.join(", ")}>
                        {c.topNodes[0] ?? `Comunidad ${c.id + 1}`}
                      </span>
                      <span className="text-muted-foreground/60 ml-auto shrink-0">{c.nodeCount}</span>
                      {c.meanIntraDegree < 2 && c.nodeCount >= 3 && (
                        <span
                          className="text-warn shrink-0"
                          title={`Poco relacionada por dentro: ${c.meanIntraDegree.toFixed(1)} relaciones por nodo`}
                        >
                          !
                        </span>
                      )}
                    </div>
                  ))}
                  </div>
                </div>
              )
            )}
          </div>
        </div>

        {/* Insights Side Panel */}
        {showInsights && (
          <div className="w-80 shrink-0 border-l bg-background overflow-y-auto">
            <div className="px-4 py-3 border-b">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Lightbulb className="h-4 w-4 text-brand" />
                  <span className="text-sm font-medium">{TEXT.insights}</span>
                </div>
                <button
                  className="p-1 rounded hover:bg-muted text-muted-foreground"
                  onClick={() => {
                    setShowInsights(false)
                    setHighlightedNodes(new Set())
                  }}
                >
                  <X className="h-4 w-4" />
                </button>
              </div>
            </div>

            <div className="p-3 flex flex-col gap-4">
              {/* Surprising Connections */}
              {surprisingConns.filter((c) => !dismissedInsights.has(c.key)).length > 0 && (
                <div>
                  <div className="flex items-center gap-1.5 mb-2 text-xs font-semibold text-foreground">
                    <Link2 className="h-3.5 w-3.5 text-info" />
                    {TEXT.surprisingConnections}
                  </div>
                  <div className="flex flex-col gap-2">
                    {surprisingConns
                      .filter((conn) => !dismissedInsights.has(conn.key))
                      .map((conn, i) => {
                        const ids = new Set([conn.source.id, conn.target.id])
                        const isActive = highlightedNodes.size === ids.size &&
                          [...ids].every((id) => highlightedNodes.has(id))
                        return (
                          <div
                            key={i}
                            className={`rounded-lg border p-3 text-sm cursor-pointer transition-colors ${isActive ? "bg-info-bg border-info-line" : "hover:bg-hover"}`}
                            onClick={() => setHighlightedNodes(isActive ? new Set() : ids)}
                          >
                            <div className="flex items-start justify-between gap-2 mb-1">
                              <span className="font-medium text-foreground text-xs">
                                {conn.source.label} ↔ {conn.target.label}
                              </span>
                              <button
                                className="shrink-0 rounded p-0.5 text-muted-foreground hover:bg-destructive/20 hover:text-destructive"
                                onClick={(e) => {
                                  e.stopPropagation()
                                  dismissInsight(conn.key, ids)
                                }}
                              >
                                <X className="h-3.5 w-3.5" />
                              </button>
                            </div>
                            <p className="text-xs text-muted-foreground">
                              {conn.reasons.join(", ")}
                            </p>
                          </div>
                        )
                      })}
                  </div>
                </div>
              )}

              {/* Knowledge Gaps */}
              {visibleKnowledgeGaps.length > 0 && (
                <div>
                  <div className="flex items-center gap-1.5 mb-2 text-xs font-semibold text-foreground">
                    <AlertTriangle className="h-3.5 w-3.5 text-warn" />
                    {TEXT.knowledgeGaps}
                  </div>
                  <div className="flex flex-col gap-2">
                    {visibleKnowledgeGaps.map((gap, i) => {
                      const gapKey = knowledgeGapKey(gap)
                      const ids = new Set(gap.nodeIds)
                      const isActive = highlightedNodes.size > 0 &&
                        [...ids].every((id) => highlightedNodes.has(id)) &&
                        [...highlightedNodes].every((id) => ids.has(id))
                      return (
                        <div
                          key={i}
                          className={`rounded-lg border p-3 text-sm cursor-pointer transition-colors ${isActive ? "bg-warn-bg border-warn-line" : "hover:bg-hover"}`}
                          onClick={() => setHighlightedNodes(isActive ? new Set() : ids)}
                        >
                          <div className="flex items-start justify-between gap-2 mb-1">
                            <div className="font-medium text-xs text-foreground">{gap.title}</div>
                            <button
                              className="shrink-0 rounded p-0.5 text-muted-foreground hover:bg-destructive/20 hover:text-destructive"
                              title={TEXT.dismiss}
                              onClick={(e) => {
                                e.stopPropagation()
                                dismissInsight(gapKey, ids)
                              }}
                            >
                              <X className="h-3.5 w-3.5" />
                            </button>
                          </div>
                          <p className="text-xs text-muted-foreground mb-2">{gap.description}</p>
                          <p className="text-xs text-muted-foreground/80 italic mb-2">{gap.suggestion}</p>
                          <Button
                            variant="default"
                            size="sm"
                            className="h-7 text-xs gap-1"
                            onClick={(e) => {
                              e.stopPropagation()
                              handleResearchClick(gap, gapKey)
                            }}
                          >
                            <Search className="h-3.5 w-3.5" />
                            {TEXT.deepResearch}
                          </Button>
                        </div>
                      )
                    })}
                  </div>
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Research Topic Confirmation Dialog */}
      {researchDialog && (
        <div className="absolute inset-0 z-50 flex items-center justify-center bg-scrim">
          <div className="w-[480px] rounded-lg border bg-background shadow-xl">
            <div className="flex items-center justify-between border-b px-4 py-3">
              <div className="flex items-center gap-2">
                <Search className="h-4 w-4 text-brand-ink" />
                <span className="font-medium text-sm">{TEXT.deepResearch}</span>
              </div>
              <button
                className="p-1 rounded hover:bg-muted text-muted-foreground"
                onClick={() => {
                  researchDialogTokenRef.current += 1
                  setResearchDialog(null)
                }}
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            {researchDialog.loading ? (
              <div className="flex items-center justify-center gap-2 py-12 text-sm text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                {TEXT.generatingTopic}
              </div>
            ) : (
              <div className="p-4">
                <div className="mb-3">
                  <label className="text-xs font-medium text-muted-foreground mb-1 block">{TEXT.researchTopic}</label>
                  <input
                    type="text"
                    className="w-full rounded-md border bg-background px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-ring"
                    value={researchDialog.topic}
                    onChange={(e: ChangeEvent<HTMLInputElement>) =>
                      setResearchDialog((prev) =>
                        prev ? { ...prev, topic: e.target.value } : prev
                      )
                    }
                  />
                </div>
                <p className="mb-3 text-xs text-muted-foreground">
                  {researchDialog.target
                    ? `Se guarda en la página de ${researchDialog.target.name}, aparte de lo que dicen los documentos.`
                    : "Se guarda como investigación suelta, en el panel de investigación."}
                </p>
                <div className="mb-4">
                  <label className="text-xs font-medium text-muted-foreground mb-1 block">{TEXT.searchQueries}</label>
                  <div className="flex flex-col gap-1.5">
                    {researchDialog.queries.map((q, idx) => (
                      <input
                        key={idx}
                        type="text"
                        className="w-full rounded-md border bg-background px-3 py-1.5 text-xs focus:outline-none focus:ring-2 focus:ring-ring"
                        value={q}
                        onChange={(e: ChangeEvent<HTMLInputElement>) =>
                          setResearchDialog((prev) => {
                            if (!prev) return prev
                            const newQueries = [...prev.queries]
                            newQueries[idx] = e.target.value
                            return { ...prev, queries: newQueries }
                          })
                        }
                      />
                    ))}
                  </div>
                </div>
                <div className="flex justify-end gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      researchDialogTokenRef.current += 1
                      setResearchDialog(null)
                    }}
                  >
                    {TEXT.cancel}
                  </Button>
                  <Button
                    variant="default"
                    size="sm"
                    className="gap-1"
                    onClick={handleResearchConfirm}
                  >
                    <Search className="h-3.5 w-3.5" />
                    {TEXT.startResearch}
                  </Button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

    </div>
  )
}
