import { useCallback, useEffect, useMemo, useState } from "react"
import {
  Bot,
  FolderOpen,
  Globe,
  Languages,
  MessageSquareText,
  Palette,
  Info,
  Network,
  History,
  Plug,
  Settings,
  Sparkles,
} from "lucide-react"
import { useTranslation } from "react-i18next"
import { invoke } from "@tauri-apps/api/core"
import { disable as disableAutostart, enable as enableAutostart } from "@tauri-apps/plugin-autostart"
import i18n from "@/i18n"
import { Button } from "@/components/ui/button"
import { useWikiStore } from "@/stores/wiki-store"
import { useChatStore } from "@/stores/chat-store"
import { useUpdateStore, hasAvailableUpdate } from "@/stores/update-store"
import { useZoomStore } from "@/stores/zoom-store"
import {
  saveLanguage,
  saveTheme,
  loadTheme,
} from "@/lib/project-store"
import { applyTheme, type AppTheme } from "@/lib/theme"
import type { SettingsDraft, DraftSetter } from "./settings-types"
import { setWikiLanguage } from "@/commands/docbase"
import { wikiLanguageOf } from "@/lib/output-language"
import { LlmProviderSection } from "./sections/llm-provider-section"
import { WebSearchSection } from "./sections/web-search-section"
import { OutputSection } from "./sections/output-section"
import { InterfaceSection } from "./sections/interface-section"
import { NetworkSection } from "./sections/network-section"
import { GeneralSection } from "./sections/general-section"
import { ChangelogSection } from "./sections/changelog-section"
import { AboutSection } from "./sections/about-section"
import { WorkFolderSection } from "./sections/work-folder-section"
import { AiServicesSection } from "./sections/ai-services-section"
import { ModelQuestionsSection } from "./sections/model-questions-section"
import { ConnectionsSection } from "./sections/connections-section"

type CategoryId =
  | "general"
  | "work-folder"
  | "ai-services"
  | "connections"
  | "model-questions"
  | "llm"
  | "web-search"
  | "network"
  | "output"
  | "interface"
  | "changelog"
  | "about"

interface Category {
  id: CategoryId
  /** i18n key under settings.categories — resolved at render time so
   *  switching language in Settings → Interface takes effect without
   *  remounting this component (Bug #53). */
  labelKey: string
  /** Its name in Spanish when it has no translation key (Micelya's own sections). */
  label?: string
  icon: typeof Bot
}

const CATEGORIES: Category[] = [
  { id: "general", labelKey: "settings.categories.general", icon: Settings },
  { id: "work-folder", labelKey: "", label: "Carpeta de trabajo", icon: FolderOpen },
  // Which engine each AI service uses (P-141); the two after it keep the
  // data of each provider (the shared connections).
  { id: "ai-services", labelKey: "", label: "Servicios de IA", icon: Sparkles },
  // The data of the providers loaded once for every service: today
  // Micelya Core (D-463).
  { id: "connections", labelKey: "", label: "Conexiones", icon: Plug },
  // What the base and the chat ask the model, read only (P-278).
  { id: "model-questions", labelKey: "", label: "Lo que se le pregunta al modelo", icon: MessageSquareText },
  { id: "llm", labelKey: "settings.categories.llm", label: "Conexiones con modelos", icon: Bot },
  { id: "web-search", labelKey: "settings.categories.webSearch", label: "Búsqueda en internet", icon: Globe },
  { id: "network", labelKey: "settings.categories.network", icon: Network },
  { id: "output", labelKey: "settings.categories.output", icon: Languages },
  { id: "interface", labelKey: "settings.categories.interface", icon: Palette },
  { id: "changelog", labelKey: "settings.categories.changelog", icon: History },
  { id: "about", labelKey: "settings.categories.about", icon: Info },
]

function initialDraft(
  llm: ReturnType<typeof useWikiStore.getState>["llmConfig"],
  outputLanguage: ReturnType<typeof useWikiStore.getState>["outputLanguage"],
  proxy: ReturnType<typeof useWikiStore.getState>["proxyConfig"],
  generalConfig: ReturnType<typeof useWikiStore.getState>["generalConfig"],
  maxHistoryMessages: number,
  uiLanguage: string,
  theme?: AppTheme,
  zoomLevel?: number,
): SettingsDraft {
  return {
    provider: llm.provider,
    apiKey: llm.apiKey,
    model: llm.model,
    ollamaUrl: llm.ollamaUrl,
    customEndpoint: llm.customEndpoint,
    azureApiVersion: llm.azureApiVersion ?? "2024-10-21",
    azureModelFamily: llm.azureModelFamily ?? "auto",
    maxContextSize: llm.maxContextSize ?? 204800,
    apiMode: llm.apiMode,
    reasoning: llm.reasoning,
    ingestReasoning: llm.ingestReasoning,
    localCliIsolation: llm.localCliIsolation === true,
    outputLanguage,
    maxHistoryMessages,
    proxyEnabled: proxy.enabled,
    proxyUrl: proxy.url,
    proxyBypassLocal: proxy.bypassLocal,
    proxyAcceptInvalidCerts: proxy.acceptInvalidCerts === true,
    autostart: generalConfig.autostart,
    closeBehavior: generalConfig.closeBehavior,
    uiLanguage,
    theme: theme ?? "system",
    zoomLevel: zoomLevel ?? useZoomStore.getState().level,
  }
}

export function SettingsView() {
  const { t } = useTranslation()
  const llmConfig = useWikiStore((s) => s.llmConfig)
  const setLlmConfig = useWikiStore((s) => s.setLlmConfig)
  const outputLanguage = useWikiStore((s) => s.outputLanguage)
  const setOutputLanguage = useWikiStore((s) => s.setOutputLanguage)
  const proxyConfig = useWikiStore((s) => s.proxyConfig)
  const setProxyConfig = useWikiStore((s) => s.setProxyConfig)
  const generalConfig = useWikiStore((s) => s.generalConfig)
  const setGeneralConfig = useWikiStore((s) => s.setGeneralConfig)
  const maxHistoryMessages = useChatStore((s) => s.maxHistoryMessages)
  const setMaxHistoryMessages = useChatStore((s) => s.setMaxHistoryMessages)
  // Drives the red dot next to the "About" row in the settings
  // sidebar. Uses `hasAvailableUpdate` (NOT `shouldShowUpdateBanner`)
  // so the indicator remains even after the user dismisses the
  // top banner — the user explicitly asked for the gear/About dots
  // to keep showing as a signpost so they can find the update
  // again later. The top banner stays gated by the dismiss
  // preference so the more aggressive interruption only fires once
  // per version.
  const updateAvailable = useUpdateStore((s) => hasAvailableUpdate(s))

  const [active, setActive] = useState<CategoryId>("ai-services")
  const [saved, setSaved] = useState(false)
  const [saveError, setSaveError] = useState<string | null>(null)
  const [currentTheme, setCurrentTheme] = useState<AppTheme>("system")
  const [draft, setDraftState] = useState<SettingsDraft>(() =>
    initialDraft(
      llmConfig,
      outputLanguage,
      proxyConfig,
      generalConfig,
      maxHistoryMessages,
      i18n.language,
    ),
  )

  // Load theme on mount
  useEffect(() => {
    loadTheme().then((theme) => {
      if (theme) {
        setCurrentTheme(theme)
        setDraftState((prev) => ({ ...prev, theme }))
      }
    }).catch(() => {})
  }, [])

  // Resync draft from store if it changes out-of-band.
  // IMPORTANT: keep the current draft.uiLanguage instead of re-reading
  // `i18n.language`. handleSave calls multiple zustand setters before it
  // calls `i18n.changeLanguage` at the end, and each setter triggers this
  // effect mid-save — which used to clobber the user's pending language
  // pick with the still-stale `i18n.language`. The next save would then
  // see draft.uiLanguage out of sync with i18n.language and silently
  // revert the UI to the previous language.
  // Same applies to zoomLevel — preserve the user's pending value through
  // the resync so mid-save store updates don't revert the input.
  useEffect(() => {
    setDraftState((prev) =>
      initialDraft(
        llmConfig,
        outputLanguage,
        proxyConfig,
        generalConfig,
        maxHistoryMessages,
        prev.uiLanguage,
        prev.theme,
        prev.zoomLevel,
      ),
    )
  }, [
    llmConfig,
    outputLanguage,
    proxyConfig,
    generalConfig,
    maxHistoryMessages,
  ])

  const setDraft: DraftSetter = useCallback((key, value) => {
    setSaveError(null)
    setDraftState((prev) => ({ ...prev, [key]: value }))
  }, [])

  useEffect(() => {
    setSaveError(null)
  }, [active])

  const handleSave = useCallback(async () => {
    setSaveError(null)
    const {
      saveLlmConfig,
      loadLlmConfig,
      saveOutputLanguage,
      loadOutputLanguage,
      saveProxyConfig,
      loadProxyConfig,
      saveGeneralConfig,
      loadGeneralConfig,
      saveZoomLevel,
      loadZoomLevel,
    } = await import("@/lib/project-store")

    const newLlm = {
      provider: draft.provider,
      apiKey: draft.apiKey,
      model: draft.model,
      ollamaUrl: draft.ollamaUrl,
      customEndpoint: draft.customEndpoint,
      azureApiVersion: draft.provider === "azure" ? draft.azureApiVersion.trim() : undefined,
      azureModelFamily: draft.provider === "azure" ? draft.azureModelFamily : undefined,
      maxContextSize: draft.maxContextSize,
      apiMode: draft.provider === "custom" ? draft.apiMode : undefined,
      reasoning: draft.reasoning,
      ingestReasoning: draft.ingestReasoning,
      localCliIsolation: draft.localCliIsolation,
    }
    const newProxy = {
      enabled: draft.proxyEnabled,
      url: draft.proxyUrl.trim(),
      bypassLocal: draft.proxyBypassLocal,
      acceptInvalidCerts: draft.proxyAcceptInvalidCerts,
    }
    const newGeneralConfig = {
      autostart: draft.autostart,
      closeBehavior: draft.closeBehavior,
    }

    // Push all config values to zustand before any awaited save below. The
    // settings draft resync effect runs after store updates; if any config stays
    // stale until later in the save sequence, that resync can briefly restore
    // the old value and make the UI look like saving reverted the user's edit.
    const languageChanged = draft.outputLanguage !== outputLanguage
    setLlmConfig(newLlm)
    setOutputLanguage(draft.outputLanguage as typeof outputLanguage)
    setProxyConfig(newProxy)
    setMaxHistoryMessages(draft.maxHistoryMessages)
    setGeneralConfig(newGeneralConfig)

    try {
      await saveLlmConfig(newLlm)
      await saveOutputLanguage(draft.outputLanguage as typeof outputLanguage)
      // The base writes the articles in this language; only a change made
      // here tells it, so opening the app never changes what it asks.
      if (languageChanged) await setWikiLanguage(wikiLanguageOf(draft.outputLanguage))
      await saveProxyConfig(newProxy)
      // Apply the proxy env vars LIVE so the next outbound request
      // picks them up — no app restart needed. tauri-plugin-http
      // builds a fresh reqwest client per fetch and reqwest reads
      // env vars at build time, so changing them here is enough.
      try {
        await invoke<string>("set_proxy_env", { config: newProxy })
      } catch (err) {
        console.warn("[proxy] live update failed; restart will still apply:", err)
      }

      await saveGeneralConfig(newGeneralConfig)
      try {
        if (newGeneralConfig.autostart) {
          await enableAutostart()
        } else {
          await disableAutostart()
        }
      } catch (err) {
        console.warn("[general] failed to update autostart:", err)
      }
      try {
        await invoke<string>("set_close_behavior", { value: newGeneralConfig.closeBehavior })
      } catch (err) {
        console.warn("[general] failed to update close behavior:", err)
      }

      if (draft.uiLanguage !== i18n.language) {
        await i18n.changeLanguage(draft.uiLanguage)
        await saveLanguage(draft.uiLanguage)
      }

      // Save theme
      if (draft.theme !== currentTheme) {
        await saveTheme(draft.theme)
        setCurrentTheme(draft.theme)
        // Apply theme immediately
        applyTheme(draft.theme)
      }

      // Apply zoom level
      useZoomStore.getState().setLevel(draft.zoomLevel)
      await saveZoomLevel(draft.zoomLevel)

      setSaved(true)
      setTimeout(() => setSaved(false), 2000)
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      console.error("[settings] failed to save settings:", err)
      const resultValue = <T,>(result: PromiseSettledResult<T>, fallback: T): T =>
        result.status === "fulfilled" ? result.value : fallback
      try {
        const [
          persistedLlm,
          persistedOutputLanguage,
          persistedProxy,
          persistedGeneral,
          persistedZoom,
        ] = await Promise.allSettled([
          loadLlmConfig(),
          loadOutputLanguage(),
          loadProxyConfig(),
          loadGeneralConfig(),
          loadZoomLevel(),
        ] as const)
        setLlmConfig(resultValue(persistedLlm, null) ?? llmConfig)
        setOutputLanguage((resultValue(persistedOutputLanguage, null) ?? outputLanguage) as typeof outputLanguage)
        setProxyConfig(resultValue(persistedProxy, null) ?? proxyConfig)
        setMaxHistoryMessages(maxHistoryMessages)
        setGeneralConfig(resultValue(persistedGeneral, generalConfig))
        useZoomStore.getState().setLevel(resultValue(persistedZoom, useZoomStore.getState().level))
      } catch (reloadErr) {
        console.warn("[settings] failed to reload persisted settings after save failure:", reloadErr)
      }
      setSaveError(message || "unknown error")
    }
  }, [
    draft,
    llmConfig,
    outputLanguage,
    proxyConfig,
    generalConfig,
    maxHistoryMessages,
    setLlmConfig,
    setOutputLanguage,
    setProxyConfig,
    setGeneralConfig,
    setMaxHistoryMessages,
    currentTheme,
  ])

  const body = useMemo(() => {
    switch (active) {
      case "general":
        return <GeneralSection draft={draft} setDraft={setDraft} />
      case "work-folder":
        // Applies at once: the folder becomes an origin of the base.
        return <WorkFolderSection />
      case "ai-services":
        // Applies at once: the base reads the registry at every request.
        return <AiServicesSection onOpen={setActive} />
      case "connections":
        // Saves on its own button: the data go to the system keychain.
        return <ConnectionsSection />
      case "model-questions":
        // Read only: nothing to save.
        return <ModelQuestionsSection />
      case "llm":
        // The LLM section manages its own store state (per-provider
        // configs + active preset) and persists directly — it bypasses
        // the shared draft / global Save button.
        return <LlmProviderSection />
      case "web-search":
        return <WebSearchSection />
      case "network":
        return <NetworkSection draft={draft} setDraft={setDraft} />
      case "output":
        return <OutputSection draft={draft} setDraft={setDraft} />
      case "interface":
        return <InterfaceSection draft={draft} setDraft={setDraft} onThemeChange={applyTheme} />
      case "changelog":
        return <ChangelogSection />
      case "about":
        return <AboutSection />
    }
  }, [active, draft, setDraft])

  return (
    <div className="flex h-full overflow-hidden">
      {/* Sidebar — category nav. Matches the IconSidebar's pill-on-accent
          pattern so the two navigational surfaces feel like one app. */}
      <aside className="flex w-56 shrink-0 flex-col border-r bg-muted/30">
        <div className="px-4 pb-2 pt-4 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
          {t("settings.title")}
        </div>
        <nav className="flex-1 overflow-y-auto px-2 pb-3">
          {CATEGORIES.map((c) => {
            const Icon = c.icon
            const isActive = c.id === active
            // Mirror the gear-icon dot inside the settings sidebar
            // so the user can find which sub-section the update
            // notification is pointing at. Update info lives in
            // the About panel, so the dot follows the About row.
            // Same store, same gating — once dismissed, both
            // disappear together.
            const showUpdateDot =
              c.id === "about" && updateAvailable
            return (
              <button
                key={c.id}
                type="button"
                onClick={() => setActive(c.id)}
                aria-current={isActive ? "page" : undefined}
                className={`group mb-0.5 flex w-full items-center gap-2.5 rounded-md px-2.5 py-1.5 text-sm transition-colors ${
                  isActive
                    ? "bg-foreground/[0.08] font-medium text-foreground ring-1 ring-border/70"
                    : "text-muted-foreground hover:bg-accent/50 hover:text-accent-foreground"
                }`}
              >
                <Icon
                  className={`h-4 w-4 shrink-0 transition-colors ${
                    isActive ? "text-primary" : "text-muted-foreground/80 group-hover:text-accent-foreground"
                  }`}
                />
                <span className="truncate">{c.label ?? t(c.labelKey)}</span>
                {showUpdateDot && (
                  <span
                    className="ml-auto h-2 w-2 shrink-0 rounded-full bg-red-500"
                    aria-label={t("nav.updateAvailable")}
                    title={t("nav.updateAvailable")}
                  />
                )}
              </button>
            )
          })}
        </nav>
      </aside>

      {/* Content */}
      <div className="flex flex-1 flex-col overflow-hidden">
        <div className="flex-1 overflow-y-auto px-8 py-6">
          <div className="mx-auto max-w-2xl">{body}</div>
        </div>

        {/* Global Save bar hidden for sections that persist inline:
            - "llm" saves per-row on every edit (independent per-preset state)
            - "about" has no draft-bound fields */}
        {active !== "about" && active !== "llm" && active !== "work-folder" && active !== "ai-services" && active !== "connections" && active !== "model-questions" && (
          <div className="shrink-0 border-t bg-background/80 backdrop-blur px-8 py-3">
            <div className="mx-auto flex max-w-2xl items-center justify-between gap-4">
              <p className={`text-xs ${saveError ? "text-destructive" : "text-muted-foreground"}`}>
                {saveError
                  ? t("settings.saveFailed")
                  : saved
                    ? t("settings.savedTick")
                    : t("settings.changeHint")}
              </p>
              <Button onClick={handleSave}>
                {saved ? t("settings.saved") : t("settings.save")}
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}
