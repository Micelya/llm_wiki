import { useState, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"
import { disable as disableAutostart, enable as enableAutostart, isEnabled as isAutostartEnabled } from "@tauri-apps/plugin-autostart"
import i18n from "@/i18n"
import { useWikiStore } from "@/stores/wiki-store"
import { useChatStore } from "@/stores/chat-store"
import { BASE_FONT_SIZE_PX, useZoomStore } from "@/stores/zoom-store"
import { loadLlmConfig, loadLanguage, loadSearchApiConfig, loadOutputLanguage, loadProviderConfigs, loadCustomLlmPresets, loadActivePresetId, loadTaskModelRouting, loadProxyConfig, loadGeneralConfig, loadZoomLevel } from "@/lib/project-store"
import { loadChatHistory, loadChatPreferences } from "@/lib/persist"
import { setupAutoSave, resumeAutoSave } from "@/lib/auto-save"
import { RELEASES_REPO } from "@/lib/app-identity"
import { useGlobalShortcut } from "@/hooks/use-global-shortcut"
import { AppLayout } from "@/components/layout/app-layout"

function applyDocumentZoom(level: number) {
  document.documentElement.style.fontSize = `${BASE_FONT_SIZE_PX * level}px`
}

function App() {
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const zoomLevel = useZoomStore((s) => s.level)
  const [loading, setLoading] = useState(true)

  // Auto-save starts suspended: the chat history is written only after it
  // has been read, so an empty store never replaces what was saved.
  useEffect(() => {
    setupAutoSave()
  }, [])

  // Register global keyboard shortcuts
  // Cmd+, on macOS or Ctrl+, on Windows/Linux opens settings
  useGlobalShortcut({
    ",": {
      callback: () => setActiveView("settings"),
      allowInTextInput: true,
    },
  })

  useEffect(() => {
    // Apply interface zoom globally, including welcome/settings screens. We
    // scale the rem base instead of using transform: scale() so layout and
    // pointer coordinates remain native; fixed-pixel panels keep their caps.
    applyDocumentZoom(zoomLevel)
  }, [zoomLevel])

  // Dev-only helper for visually testing the update-banner UX.
  // Open dev tools and run:
  //   __llmwiki_testUpdateBanner()
  // to inject a fake "available" result into the update store —
  // banner appears at the top + red dot lights up the gear icon.
  // Run again with arg `false` (or call setDismissed via the store)
  // to clear. Gated on `import.meta.env.DEV` so the helper never
  // ships in production builds.
  useEffect(() => {
    if (!import.meta.env.DEV) return
    ;(async () => {
      const storeMod = await import("@/stores/update-store")
      const { useUpdateStore } = storeMod
      // Expose the live store getter on window so you can inspect
      // state from devtools when debugging banner behavior.
      ;(window as unknown as { __llmwiki_updateStore?: typeof useUpdateStore }).__llmwiki_updateStore = useUpdateStore
      ;(window as unknown as { __llmwiki_testUpdateBanner?: (clear?: boolean) => void }).__llmwiki_testUpdateBanner = (clear = false) => {
        if (clear) {
          useUpdateStore.getState().setResult(
            { kind: "up-to-date", local: __APP_VERSION__, remote: __APP_VERSION__ },
            Date.now(),
          )
          useUpdateStore.getState().setDismissed(null)
          console.log("[test] update banner cleared")
          return
        }
        useUpdateStore.getState().setResult(
          {
            kind: "available",
            local: __APP_VERSION__,
            remote: "v999.0.0",
            release: {
              name: "v999.0.0 (test)",
              tag_name: "v999.0.0",
              body:
                "Test release for banner-UX verification.\n\n" +
                "- Bigger red dot on the Settings icon\n" +
                "- Top banner with one-click dismiss\n" +
                "- Once dismissed, won't reappear for this version",
              html_url: "https://github.com/nashsu/llm_wiki/releases",
              published_at: new Date().toISOString(),
            },
          },
          Date.now(),
        )
        useUpdateStore.getState().setDismissed(null)
        console.log(
          "[test] update banner injected. Run __llmwiki_testUpdateBanner(true) to clear.",
        )
      }
    })()
  }, [])

  // Background update check — hydrate persisted user preferences, then
  // hit GitHub at most once every UPDATE_CHECK_CACHE_MS. Runs 1.5 s
  // after mount so it doesn't contend with the heaviest startup work
  // (project load, file tree, vector store init) but still surfaces
  // a new release in time for the user to notice it during their
  // first interaction. Silent on failure; the UI in Settings → About
  // lets the user retry manually.
  useEffect(() => {
    let cancelled = false
    const timer = setTimeout(async () => {
      if (cancelled) return
      try {
        const { loadUpdateCheckState, saveUpdateCheckState } = await import(
          "@/lib/project-store"
        )
        const { useUpdateStore } = await import("@/stores/update-store")
        const { checkForUpdates, UPDATE_CHECK_CACHE_MS } = await import(
          "@/lib/update-check"
        )

        const persisted = await loadUpdateCheckState()
        if (persisted) useUpdateStore.getState().hydrate(persisted)

        const state = useUpdateStore.getState()
        if (!state.enabled) {
          console.log("[update-check] skipped: user disabled auto-check in settings")
          return
        }

        const now = Date.now()
        // Cache hit requires BOTH the timestamp AND the in-memory
        // result to be present. `lastCheckedAt` is persisted to
        // disk but `lastResult` deliberately is not — keeping the
        // GitHub payload out of the persisted store keeps disk
        // size + privacy footprint small. The downside: a fresh
        // cold start has `lastResult === null` even when
        // `lastCheckedAt` is recent, in which case we MUST refetch
        // — otherwise we'd skip the check AND have no result to
        // display, leaving the banner permanently stuck off.
        // (This was the user-reported bug: "kind=none, no banner".)
        const fresh =
          state.lastCheckedAt !== null &&
          state.lastResult !== null &&
          now - state.lastCheckedAt < UPDATE_CHECK_CACHE_MS
        if (fresh) {
          const ageMin = Math.round((now - (state.lastCheckedAt ?? 0)) / 60_000)
          console.log(
            `[update-check] skipped: cache hit (last check ${ageMin} min ago, ` +
              `cache window ${UPDATE_CHECK_CACHE_MS / 60_000} min). ` +
              `Last result: kind=${state.lastResult?.kind ?? "none"}`,
          )
          return
        }

        useUpdateStore.getState().setChecking(true)
        console.log(
          `[update-check] fetching GitHub releases (local=${__APP_VERSION__})`,
        )
        const result = await checkForUpdates({
          currentVersion: __APP_VERSION__,
          repo: RELEASES_REPO,
        })
        if (cancelled) return
        useUpdateStore.getState().setResult(result, Date.now())
        if (result.kind === "available") {
          console.log(
            `[update-check] update available: local=${result.local} → remote=${result.remote}`,
          )
        } else if (result.kind === "up-to-date") {
          console.log(
            `[update-check] up to date: local=${result.local}, remote latest=${result.remote}`,
          )
        } else {
          console.log(`[update-check] error: ${result.message}`)
        }
        await saveUpdateCheckState({
          enabled: useUpdateStore.getState().enabled,
          lastCheckedAt: Date.now(),
          dismissedVersion: useUpdateStore.getState().dismissedVersion,
        })
      } catch {
        // Silent — Settings → About lets the user retry manually.
      }
    }, 1500)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [])

  // Load the settings and the chat once, on startup. The app opens on
  // Inicio: there are no projects to choose.
  useEffect(() => {
    async function init() {
      try {
        const savedZoom = await loadZoomLevel()
        applyDocumentZoom(savedZoom)
        useZoomStore.getState().setLevel(savedZoom)

        const savedConfig = await loadLlmConfig()
        if (savedConfig) {
          useWikiStore.getState().setLlmConfig(savedConfig)
        }
        const savedProviderConfigs = await loadProviderConfigs()
        if (savedProviderConfigs) {
          useWikiStore.getState().setProviderConfigs(savedProviderConfigs)
        }
        const savedCustomLlmPresets = await loadCustomLlmPresets()
        useWikiStore.getState().setCustomLlmPresets(savedCustomLlmPresets)
        const savedActivePreset = await loadActivePresetId()
        if (savedActivePreset) {
          useWikiStore.getState().setActivePresetId(savedActivePreset)
          // Re-resolve the active preset's LlmConfig from (preset defaults
          // + saved overrides). Without this, preset default updates
          // (e.g. a corrected Anthropic model ID shipped in a release)
          // never reach users who are relying on defaults: their stored
          // `llmConfig` snapshot from a previous launch would keep the
          // old value. Overrides still win, so an explicit user choice
          // is preserved.
          const { findLlmPreset } = await import("@/components/settings/llm-presets")
          const { resolveConfig } = await import("@/components/settings/preset-resolver")
          const preset = findLlmPreset(savedActivePreset, savedCustomLlmPresets)
          if (preset) {
            const currentFallback = useWikiStore.getState().llmConfig
            const override = (savedProviderConfigs ?? {})[savedActivePreset]
            const resolved = resolveConfig(preset, override, currentFallback)
            useWikiStore.getState().setLlmConfig(resolved)
            const { saveLlmConfig } = await import("@/lib/project-store")
            await saveLlmConfig(resolved)
          }
        }
        const savedTaskModelRouting = await loadTaskModelRouting()
        if (savedTaskModelRouting) {
          useWikiStore.getState().setTaskModelRouting(savedTaskModelRouting)
        }
        const savedSearchConfig = await loadSearchApiConfig()
        if (savedSearchConfig) {
          useWikiStore.getState().setSearchApiConfig(savedSearchConfig)
        }
        const savedProxy = await loadProxyConfig()
        if (savedProxy) {
          useWikiStore.getState().setProxyConfig(savedProxy)
        }
        const savedOutputLanguage = await loadOutputLanguage()
        useWikiStore.getState().setOutputLanguage(savedOutputLanguage ?? "auto")
        const savedGeneral = await loadGeneralConfig()
        useWikiStore.getState().setGeneralConfig(savedGeneral)
        try {
          await invoke<string>("set_close_behavior", { value: savedGeneral.closeBehavior })
        } catch (err) {
          console.warn("[general] failed to hydrate close behavior:", err)
        }
        try {
          const currentAutostart = await isAutostartEnabled()
          if (savedGeneral.autostart && !currentAutostart) {
            await enableAutostart()
          } else if (!savedGeneral.autostart && currentAutostart) {
            await disableAutostart()
          }
        } catch (err) {
          console.warn("[general] failed to sync autostart:", err)
        }
        const savedLang = await loadLanguage()
        if (savedLang) {
          await i18n.changeLanguage(savedLang)
        }
      } catch {
        // ignore init errors
      } finally {
        setLoading(false)
      }
      await hydrateChat()
    }
    init()
  }, [])

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center bg-background text-muted-foreground">
        Cargando...
      </div>
    )
  }

  return <AppLayout />
}

/**
 * The chat's preferences and history, from the app's data folder (D-415).
 * Auto-save of the history resumes only once it was read: an unreadable
 * history stays as it is on disk instead of being replaced by an empty one.
 */
async function hydrateChat(): Promise<void> {
  let historyRead = false
  try {
    const preferences = await loadChatPreferences()
    const chat = useChatStore.getState()
    chat.setUseWebSearch(preferences.useWebSearch)
    chat.setUseAnyTxtSearch(preferences.useAnyTxtSearch)
    chat.setAgentMode(preferences.agentMode)
    chat.setRetrievalMode(preferences.retrievalMode)
    chat.setDisabledSkills(preferences.disabledSkills)
  } catch {
    // ignore, start with the defaults
  }
  try {
    const activeBeforeLoad = useChatStore.getState().activeConversationId
    const saved = await loadChatHistory()
    historyRead = true
    const state = useChatStore.getState()
    const activeChangedDuringLoad = state.activeConversationId !== activeBeforeLoad
    if (saved.conversations.length > 0) {
      // A conversation started while the history was loading stays.
      const conversations = [
        ...state.conversations,
        ...saved.conversations.filter((conversation) => !state.conversations.some((existing) => existing.id === conversation.id)),
      ]
      const messages = [
        ...state.messages,
        ...saved.messages.filter((message) => !state.messages.some((existing) => existing.id === message.id)),
      ]
      state.setConversations(conversations)
      state.setMessages(messages)
      const latest = [...saved.conversations].sort((a, b) => b.updatedAt - a.updatedAt)[0]
      if (!activeChangedDuringLoad && latest) state.setActiveConversation(latest.id)
    }
  } catch (err) {
    console.warn("[startup] failed to load chat history:", err)
  }
  resumeAutoSave({ history: historyRead })
}

export default App
