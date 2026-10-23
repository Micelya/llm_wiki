import type { ReactNode } from "react"
import {
  Bell, Briefcase, Files, Globe, Hand, HardDrive, House, Landmark, LayoutTemplate, LoaderCircle,
  MessageSquare, Network, Settings, Sparkles, Users, type LucideIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { useWikiStore, type WikiState } from "@/stores/wiki-store"
import { useResearchStore } from "@/stores/research-store"
import { useUpdateStore, hasAvailableUpdate } from "@/stores/update-store"
import { isResearchPanelVisible, nextResearchPanelNavState } from "./research-panel-nav"
import { navigationCount } from "@/lib/attention/order"
import { useAttention } from "@/components/attention/use-attention"
import { usePortalStatus } from "@/components/portals/use-portal-status"
import { vocabulary } from "@/vocabulary"

type View = WikiState["activeView"]

/**
 * The item of the menu a screen belongs to: the screens made to resolve
 * what Atención asks (copies and versions, the types, the doubtful
 * identities) are reached from it, and the search from the top bar.
 */
export function menuItemOf(view: View): View | null {
  switch (view) {
    case "similar":
    case "document-types":
    case "identity-doubts":
      return "attention"
    case "search":
      return null
    default:
      return view
  }
}

/** The items of the archive's group, named by the vocabulary of the vertical where it has the word. */
function archiveItems(): { view: View; icon: LucideIcon; label: string }[] {
  const words = vocabulary()
  return [
    { view: "home", icon: House, label: "Inicio" },
    { view: "attention", icon: Bell, label: words.attention.name },
    { view: "documents", icon: Files, label: "Documentos" },
    { view: "entities", icon: Users, label: words.knowledge.entities },
    { view: "matters", icon: Briefcase, label: words.knowledge.matters },
    { view: "templates", icon: LayoutTemplate, label: words.templates.name },
    { view: "origins", icon: HardDrive, label: "Orígenes" },
    { view: "portals", icon: Landmark, label: words.portals.name },
  ]
}

/**
 * The menu at the left (P-057, D-493): every item with its name written,
 * in Spanish, in two groups as in the mockup, the archive and the tools,
 * and the settings at the foot; how many things wait in Atención, and a
 * word where something needs the person. Under 1000 px it keeps only the
 * icons, each with its name when the mouse is over it.
 */
export function AppSidebar() {
  const words = vocabulary()
  const activeView = useWikiStore((s) => s.activeView)
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const attentionCount = navigationCount(useAttention().counts)
  // The portal waits for a person (its challenge, signing in).
  const portalNeedsPerson = usePortalStatus()?.progress.state === "needs-person"
  const researchPanelOpen = useResearchStore((s) => s.panelOpen)
  const researchActive = useResearchStore((s) => s.tasks.filter((t) => t.status !== "done" && t.status !== "error").length)
  const setResearchPanel = useResearchStore((s) => s.setPanelOpen)
  // While the update exists, even after its banner was closed: the only
  // sign left of where to find it.
  const updateAvailable = useUpdateStore((s) => hasAvailableUpdate(s))
  const current = menuItemOf(activeView)

  function toggleResearch() {
    const next = nextResearchPanelNavState(activeView, researchPanelOpen)
    if (next.activeView !== activeView) setActiveView(next.activeView)
    setResearchPanel(next.researchPanelOpen)
  }

  return (
    <nav
      aria-label="Menú principal"
      className="flex h-full w-16 shrink-0 flex-col overflow-y-auto border-r bg-sidebar px-3 pt-3.5 pb-3 min-[1000px]:w-[252px]"
    >
      <div role="group" aria-label={words.archive.name} className="flex flex-col gap-0.5">
        {archiveItems().map(({ view, icon, label }) => (
          <NavItem
            key={view}
            icon={icon}
            label={label}
            active={current === view}
            onClick={() => setActiveView(view)}
            hint={
              view === "attention" && attentionCount > 0
                ? `${attentionCount} ${attentionCount === 1 ? "cosa espera" : "cosas esperan"} a una persona`
                : view === "portals" && portalNeedsPerson
                  ? "El portal te necesita"
                  : undefined
            }
          >
            {view === "attention" && attentionCount > 0 && <NavCount>{attentionCount > 99 ? "99+" : attentionCount}</NavCount>}
            {view === "portals" && portalNeedsPerson && (
              <NavFlag icon={Hand} tone="warn">
                te necesita
              </NavFlag>
            )}
          </NavItem>
        ))}
      </div>

      <div className="hidden px-2.5 pt-3.5 pb-1.5 font-ui text-[11px] font-bold tracking-[0.08em] text-muted-foreground uppercase min-[1000px]:block">
        Herramientas
      </div>
      <div aria-hidden className="mx-2 my-2 h-px bg-border min-[1000px]:hidden" />
      <div role="group" aria-label="Herramientas" className="flex flex-col gap-0.5">
        <NavItem icon={MessageSquare} label="Chat" active={current === "chat"} onClick={() => setActiveView("chat")} />
        <NavItem icon={Network} label="Grafo" active={current === "graph"} onClick={() => setActiveView("graph")} />
        <NavItem
          icon={Globe}
          label="Investigación"
          active={isResearchPanelVisible(activeView, researchPanelOpen)}
          onClick={toggleResearch}
          hint={researchActive > 0 ? `${researchActive} ${researchActive === 1 ? "investigación en curso" : "investigaciones en curso"}` : undefined}
        >
          {researchActive > 0 && (
            <NavFlag icon={LoaderCircle} tone="info" spin>
              {researchActive}
            </NavFlag>
          )}
        </NavItem>
        <NavItem icon={Sparkles} label="Habilidades" active={current === "skills"} onClick={() => setActiveView("skills")} />
      </div>

      <div className="mt-auto flex flex-col gap-0.5 border-t pt-2.5">
        <NavItem
          icon={Settings}
          label="Ajustes"
          active={current === "settings"}
          onClick={() => setActiveView("settings")}
          hint={updateAvailable ? "Hay una versión nueva · Ctrl+," : "Ctrl+,"}
        >
          {updateAvailable && (
            <NavFlag icon={Sparkles} tone="info">
              versión nueva
            </NavFlag>
          )}
        </NavItem>
      </div>
    </nav>
  )
}

function NavItem({
  icon: Icon,
  label,
  active,
  onClick,
  hint,
  children,
}: {
  icon: LucideIcon
  label: string
  active: boolean
  onClick: () => void
  /** What the item says beyond its name, when the mouse is over it. */
  hint?: string
  children?: ReactNode
}) {
  return (
    <button
      type="button"
      aria-current={active ? "page" : undefined}
      onClick={onClick}
      title={hint ? `${label} · ${hint}` : label}
      className={cn(
        "relative flex h-9 w-full shrink-0 items-center gap-2.5 rounded-lg px-2.5 text-left font-ui text-sm font-semibold transition-colors",
        active ? "bg-brand-soft text-brand-ink" : "text-foreground hover:bg-hover hover:text-strong",
      )}
    >
      {active && <span aria-hidden className="absolute top-[7px] bottom-[7px] -left-3 w-[3px] rounded-r-[3px] bg-grad-principal" />}
      <Icon className={cn("size-[18px] shrink-0", active ? "text-brand-ink" : "text-muted-foreground")} strokeWidth={1.8} aria-hidden />
      <span className="hidden min-w-0 flex-1 truncate min-[1000px]:block">{label}</span>
      {children}
    </button>
  )
}

/** How many things wait, in the brand's round count. */
function NavCount({ children }: { children: ReactNode }) {
  return (
    <span className="absolute -top-1 -right-1 inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-[var(--nav-count-bg)] px-1.5 font-ui text-xs font-bold text-[var(--nav-count-fg)] min-[1000px]:static min-[1000px]:ml-auto min-[1000px]:min-w-[22px]">
      {children}
    </span>
  )
}

const FLAG_TONES = {
  warn: "border-warn-line bg-warn-bg text-warn",
  info: "border-info-line bg-info-bg text-info",
} as const

/** A short word with its icon where something needs the person or is going on. */
function NavFlag({
  icon: Icon,
  tone,
  spin = false,
  children,
}: {
  icon: LucideIcon
  tone: keyof typeof FLAG_TONES
  spin?: boolean
  children: ReactNode
}) {
  return (
    <span
      className={cn(
        "absolute -top-1 -right-1 inline-flex h-5 items-center gap-1 rounded-full border px-1 font-ui text-[11px] font-bold whitespace-nowrap min-[1000px]:static min-[1000px]:ml-auto min-[1000px]:px-1.5",
        FLAG_TONES[tone],
      )}
    >
      <Icon className={cn("size-3 shrink-0", spin && "animate-spin")} strokeWidth={2.2} aria-hidden />
      <span className="hidden min-[1000px]:inline">{children}</span>
    </span>
  )
}
