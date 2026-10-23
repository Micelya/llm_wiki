import { useCallback } from "react"
import { Sparkles, X, Download } from "lucide-react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { openUrl } from "@tauri-apps/plugin-opener"
import { useUpdateStore, shouldShowUpdateBanner } from "@/stores/update-store"
import { saveUpdateCheckState } from "@/lib/project-store"
import { toLatestReleaseUrl } from "@/lib/update-check"

/**
 * App-wide update-available banner.
 *
 * Sits under the top bar, above the main flex row (menu + content). Renders ONLY
 * when `shouldShowUpdateBanner(state)` returns true — that helper
 * already gates on:
 *   - we have a successful "available" check result
 *   - the user hasn't dismissed THIS exact remote version
 * so a closed banner stays closed for the dismissed version, and
 * automatically reappears the moment a NEWER release ships
 * (dismissedVersion no longer matches the new remote).
 *
 * Persistence: the X handler writes the dismissed version through
 * `saveUpdateCheckState` so the close decision survives a restart.
 *
 * Why not use the same component as the About-page banner: that one
 * is opt-in (user navigated to settings), so it shows full release
 * notes and a "Later" button. This one is uninvited and must stay
 * compact / single-line so it doesn't disrupt the working layout —
 * just version + one CTA + close.
 */
export function UpdateBanner() {
  const { t } = useTranslation()
  const visible = useUpdateStore((s) => shouldShowUpdateBanner(s))
  const result = useUpdateStore((s) => s.lastResult)

  const handleOpen = useCallback(async () => {
    if (!result || result.kind !== "available") return
    // Send the user to `/releases/latest`, NOT the tag-specific
    // page from `release.html_url`. /latest always follows GitHub's
    // redirect to whatever is currently the most recent release —
    // robust to (a) a newer release shipping between notification
    // and click, and (b) the bare `/releases` listing's default
    // sort not putting newest at the top.
    try {
      await openUrl(toLatestReleaseUrl(result.release.html_url))
    } catch (err) {
      console.error("[update-banner] openUrl failed:", err)
    }
  }, [result])

  const handleDismiss = useCallback(async () => {
    if (!result || result.kind !== "available") return
    // Mark THIS remote version as dismissed; helper hides banner.
    // Future newer releases re-pop because their version doesn't
    // match dismissedVersion anymore. Persist so the choice
    // survives a restart — without that, the banner re-appears
    // every cold start, which is the user complaint we're fixing.
    useUpdateStore.getState().setDismissed(result.remote)
    await saveUpdateCheckState({
      enabled: useUpdateStore.getState().enabled,
      lastCheckedAt: useUpdateStore.getState().lastCheckedAt ?? Date.now(),
      dismissedVersion: result.remote,
    })
  }, [result])

  if (!visible || !result || result.kind !== "available") return null

  return (
    // A notice of information of the style guide (D-493): its icon and
    // its words, the way to the download as the main button of the line,
    // and the close quiet.
    <div className="flex shrink-0 items-center gap-3 border-b border-info-line bg-info-bg px-4 py-2 text-sm">
      <div className="flex min-w-0 flex-1 items-center gap-2.5">
        <Sparkles className="size-4 shrink-0 text-info" aria-hidden />
        <span className="truncate font-ui font-semibold text-strong">
          {t("updateBanner.message", {
            version: result.remote.replace(/^v/, ""),
            defaultValue: `Está disponible la versión ${result.remote.replace(/^v/, "")}`,
          })}
        </span>
      </div>
      <Button size="sm" onClick={handleOpen}>
        <Download />
        {t("updateBanner.openDownload", { defaultValue: "Abrir la página de descarga" })}
      </Button>
      <Button
        size="icon-sm"
        variant="ghost"
        onClick={handleDismiss}
        aria-label={t("updateBanner.dismiss", { defaultValue: "Ahora no" })}
        title={t("updateBanner.dismiss", { defaultValue: "Ahora no" })}
      >
        <X />
      </Button>
    </div>
  )
}
