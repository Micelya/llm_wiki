import { useCallback, useRef, useState } from "react"
import { useWikiStore } from "@/stores/wiki-store"
import { IconSidebar } from "./icon-sidebar"
import { UpdateBanner } from "./update-banner"
import { ContentArea } from "./content-area"
import { ResearchPanel } from "./research-panel"
import { useResearchStore } from "@/stores/research-store"
import { ErrorBoundary } from "@/components/error-boundary"
import { AttentionSources } from "@/components/attention/sources-host"
import { ProgressPoller } from "@/components/progress/use-progress"
import { ProgressLine } from "@/components/progress/progress-line"
import { PortalLine } from "@/components/portals/portal-line"
import { isResearchPanelVisible } from "./research-panel-nav"

export function AppLayout() {
  const activeView = useWikiStore((s) => s.activeView)
  const researchPanelOpen = useResearchStore((s) => s.panelOpen)
  const [rightWidth, setRightWidth] = useState(400)
  const isDraggingRight = useRef(false)
  const containerRef = useRef<HTMLDivElement>(null)

  const startDrag = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    isDraggingRight.current = true
    document.body.style.cursor = "col-resize"
    document.body.style.userSelect = "none"
    document.body.dataset.panelResizing = "true"

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current || !isDraggingRight.current) return
      const rect = containerRef.current.getBoundingClientRect()
      const newWidth = rect.right - e.clientX
      // Hard cap: 250 to 50% of container
      setRightWidth(Math.max(250, Math.min(rect.width * 0.5, newWidth)))
    }

    const handleMouseUp = () => {
      isDraggingRight.current = false
      document.body.style.cursor = ""
      document.body.style.userSelect = ""
      delete document.body.dataset.panelResizing
      document.removeEventListener("mousemove", handleMouseMove)
      document.removeEventListener("mouseup", handleMouseUp)
    }

    document.addEventListener("mousemove", handleMouseMove)
    document.addEventListener("mouseup", handleMouseUp)
  }, [])

  // Chat, skills and settings use the whole work area; beside the other
  // screens the research panel can stay open.
  const hasRightPanel = isResearchPanelVisible(activeView, researchPanelOpen)

  return (
    // Outer column layout: full-width update banner on top (when an
    // update is available AND not dismissed for this version), the
    // IconSidebar + content row below.
    <div className="flex h-full flex-col bg-background text-foreground">
      <AttentionSources />
      <ProgressPoller />
      <UpdateBanner />
      <div className="flex min-h-0 flex-1">
        <IconSidebar />
        <div ref={containerRef} className="relative flex min-w-0 flex-1 overflow-hidden">
          <div className="min-w-0 flex-1 overflow-hidden">
            <ErrorBoundary>
              <ContentArea />
            </ErrorBoundary>
          </div>

          {hasRightPanel && (
            <>
              <div
                className="w-1.5 shrink-0 cursor-col-resize bg-border/40 transition-colors hover:bg-primary/30 active:bg-primary/40"
                onMouseDown={startDrag}
              />
              <div
                className="flex shrink-0 flex-col overflow-hidden border-l"
                style={{ width: rightWidth }}
              >
                <ErrorBoundary>
                  <ResearchPanel />
                </ErrorBoundary>
              </div>
            </>
          )}
        </div>
      </div>
      {/* A walk over a judicial portal, from any screen: it goes on while the person works elsewhere. */}
      <PortalLine />
      {/* What the app is processing, from any screen (D-426). */}
      <ProgressLine />
    </div>
  )
}
