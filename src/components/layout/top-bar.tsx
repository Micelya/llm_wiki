import { useEffect, useRef, useState } from "react"
import { ArrowLeft, ArrowRight, Moon, Search, Sun } from "lucide-react"
import { useTranslation } from "react-i18next"
import { PRODUCT_NAME } from "@/lib/app-identity"
import { applyTheme } from "@/lib/theme"
import { saveTheme } from "@/lib/project-store"
import { isImeComposing } from "@/lib/keyboard-utils"
import { useGlobalShortcut } from "@/hooks/use-global-shortcut"
import { useNavigationHistory } from "@/hooks/use-navigation-history"
import { useNavigationHistoryStore } from "@/stores/navigation-history-store"
import { useWikiStore } from "@/stores/wiki-store"
import { useSearchStore } from "@/stores/search-store"
import { Kbd } from "@/components/ui/badge"
import { vocabulary } from "@/vocabulary"
import logo from "@/assets/brand/logo-micelya-1linea.png"

/** Whether the window is in dark mode now, following every change of it. */
function useDarkMode(): boolean {
  const [dark, setDark] = useState(() => document.documentElement.classList.contains("dark"))
  useEffect(() => {
    const root = document.documentElement
    const observer = new MutationObserver(() => setDark(root.classList.contains("dark")))
    observer.observe(root, { attributes: true, attributeFilter: ["class"] })
    return () => observer.disconnect()
  }, [])
  return dark
}

/** The look of the buttons that go back and forward: quiet, and faint when there is nowhere to go. */
const historyButton =
  "inline-flex size-[34px] items-center justify-center rounded-lg text-foreground transition-colors hover:bg-hover disabled:pointer-events-none disabled:opacity-35"

function isMac(): boolean {
  return document.documentElement.classList.contains("platform-macos")
}

/**
 * The bar at the top of every screen, as the manual asks (P-057, D-493):
 * the line of the brand's gradient above it all, the logo (it leads to
 * Inicio), the way back and forward on what the person opened (Alt+←,
 * Alt+→ and the mouse's side buttons), the archive that is open, the
 * search of the whole archive (Ctrl K from any screen; Enter opens its
 * results) and the switch between light and dark mode.
 */
export function TopBar() {
  const words = vocabulary()
  const { t } = useTranslation()
  const canGoBack = useNavigationHistoryStore((s) => s.index > 0)
  const canGoForward = useNavigationHistoryStore((s) => s.index < s.entries.length - 1)
  const goBack = useNavigationHistoryStore((s) => s.back)
  const goForward = useNavigationHistoryStore((s) => s.forward)
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const ask = useSearchStore((s) => s.ask)
  const input = useRef<HTMLInputElement>(null)
  const [query, setQuery] = useState("")
  const dark = useDarkMode()

  useNavigationHistory()

  useGlobalShortcut({
    k: {
      callback: () => {
        input.current?.focus()
        input.current?.select()
      },
      allowInTextInput: true,
    },
  })

  function search() {
    const asked = query.trim()
    if (!asked) return
    ask(asked)
    setActiveView("search")
  }

  async function switchMode() {
    const next = dark ? "light" : "dark"
    applyTheme(next)
    try {
      await saveTheme(next)
    } catch (err) {
      console.warn("[theme] the mode could not be saved:", err)
    }
  }

  return (
    <header className="relative z-40 flex h-[var(--topbar-height)] shrink-0 items-center gap-5 border-b bg-bar pr-5 pl-[18px]">
      <span aria-hidden className="absolute inset-x-0 top-0 h-[3px] bg-grad-vibrante" />
      <button
        type="button"
        className="flex shrink-0 items-center rounded-md min-[1000px]:w-[214px]"
        title={`${PRODUCT_NAME} · Inicio`}
        onClick={() => setActiveView("home")}
      >
        <img src={logo} alt="micelya" className="block h-[26px] w-auto [filter:var(--logo-filter)]" />
      </button>
      <nav aria-label={t("nav.history")} className="flex shrink-0 items-center gap-1">
        <button
          type="button"
          onClick={() => void goBack()}
          disabled={!canGoBack}
          aria-label={t("nav.back")}
          title={t("nav.backHint")}
          className={historyButton}
        >
          <ArrowLeft className="size-4" aria-hidden />
        </button>
        <button
          type="button"
          onClick={() => void goForward()}
          disabled={!canGoForward}
          aria-label={t("nav.forward")}
          title={t("nav.forwardHint")}
          className={historyButton}
        >
          <ArrowRight className="size-4" aria-hidden />
        </button>
      </nav>
      <span
        className="hidden shrink-0 truncate px-2.5 font-ui text-sm font-semibold text-strong min-[760px]:block"
        title={`${words.archive.name} abierto`}
      >
        {words.archive.shared}
      </span>
      <form
        role="search"
        className="flex h-9 max-w-[560px] min-w-0 flex-1 items-center gap-2 rounded-lg border bg-panel-sub pr-2.5 pl-3 text-muted-foreground focus-within:border-link focus-within:ring-3 focus-within:ring-ring"
        onSubmit={(event) => {
          event.preventDefault()
          search()
        }}
      >
        <Search className="size-4 shrink-0" aria-hidden />
        <input
          ref={input}
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (isImeComposing(event)) return
            if (event.key === "Escape") input.current?.blur()
          }}
          placeholder={`Buscar personas, ${words.knowledge.matters.toLowerCase()} y documentos…`}
          aria-label={`Buscar en el ${words.archive.name.toLowerCase()}`}
          className="min-w-0 flex-1 border-0 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground [&::-webkit-search-cancel-button]:hidden"
        />
        <Kbd className="hidden sm:inline-flex">{isMac() ? "⌘ K" : "Ctrl K"}</Kbd>
      </form>
      <div className="ml-auto flex shrink-0 items-center gap-2">
        <button
          type="button"
          onClick={() => void switchMode()}
          aria-label={dark ? "Cambiar a modo claro" : "Cambiar a modo oscuro"}
          className="inline-flex h-[34px] items-center gap-1.5 rounded-lg border bg-card px-2.5 font-ui text-[0.8125rem] font-semibold text-foreground transition-colors hover:bg-hover"
        >
          {dark ? <Sun className="size-4" aria-hidden /> : <Moon className="size-4" aria-hidden />}
          <span className="hidden md:inline">{dark ? "Modo claro" : "Modo oscuro"}</span>
        </button>
      </div>
    </header>
  )
}
