import { useEffect, useState } from "react"
import { BookOpen, Pause, Play } from "lucide-react"
import { Button } from "@/components/ui/button"
import { getGeneralProject, type WikiSource } from "@/commands/docbase"
import {
  isSameProjectPath,
  pendingWikiSourcesByOrigin,
  setWikiFeeding,
  waitingInQueue,
} from "@/lib/docbase-wiki-bridge"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useWikiStore } from "@/stores/wiki-store"
import { vocabulary } from "@/vocabulary"
import { ErrorNote, Panel, StatusBadge } from "./page"
import { plural } from "./format"
import { useDocbaseQuery } from "./use-docbase-query"

/**
 * The wiki of the general project: the one whose sources are all the
 * folders of the base. It does not start by itself, because feeding it
 * uses the model; once turned on it is fed by itself.
 */
export function GeneralProjectPanel() {
  const general = useDocbaseQuery("general-project", getGeneralProject)
  const project = useWikiStore((s) => s.project)
  const dataVersion = useWikiStore((s) => s.dataVersion)
  const dialog = useAppDialog()
  const words = vocabulary()
  const [pending, setPending] = useState<Map<string, WikiSource[]>>(new Map())
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)

  const data = general.data
  const isOpenProject = isSameProjectPath(data?.path, project?.path)

  // What is pending changes as the wiki ingests, so it is recounted when
  // the base or the project's data changes.
  useEffect(() => {
    if (!project || !isOpenProject) {
      setPending(new Map())
      return
    }
    let stale = false
    pendingWikiSourcesByOrigin(project)
      .then((result) => {
        if (!stale) setPending(result)
      })
      .catch((cause) => {
        if (!stale) setProblem(String(cause))
      })
    return () => {
      stale = true
    }
  }, [project, isOpenProject, data, dataVersion])

  if (!data || !data.path) return general.error ? <ErrorNote>{general.error}</ErrorNote> : null

  const pendingTotal = [...pending.values()].reduce((sum, list) => sum + list.length, 0)
  const waiting = project && isOpenProject ? waitingInQueue(project) : 0

  async function toggle(active: boolean) {
    if (!project) return
    const confirmed = await dialog.confirm(
      active
        ? {
            title: `Activar el ${words.archive.wiki.toLowerCase()}`,
            message:
              (pendingTotal + waiting > 0
                ? `Se van a incorporar ${plural(pendingTotal + waiting, "documento", "documentos")} ahora. `
                : "") +
              "Desde ahora todo lo que se agregue o cambie en las carpetas incorporadas pasa al wiki solo. " +
              "Cada documento pasa por el modelo configurado, así que consume cupo o costo de ese proveedor. " +
              "Las copias de un mismo documento se incorporan una sola vez.",
            confirmLabel: "Activar",
            cancelLabel: "Cancelar",
          }
        : {
            title: `Pausar el ${words.archive.wiki.toLowerCase()}`,
            message:
              "No se manda nada más al modelo: deja de incorporarse lo nuevo y lo que esperaba en la cola queda detenido, " +
              "incluido el documento que estuviera en curso. Las páginas que ya existen quedan como están. " +
              "Al activarlo de nuevo sigue desde donde quedó.",
            confirmLabel: "Pausar",
            cancelLabel: "Cancelar",
          },
    )
    if (!confirmed) return
    setBusy(true)
    setProblem(null)
    setNotice(null)
    try {
      const queued = await setWikiFeeding(project, active)
      if (active) {
        setNotice(
          queued + waiting > 0
            ? `${plural(queued + waiting, "documento quedó", "documentos quedaron")} en la cola de incorporación.`
            : "Activado. No había documentos pendientes de incorporar, o no hay un modelo configurado.",
        )
      }
      general.reload()
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Panel
      title={
        <>
          <BookOpen className="size-4" /> {words.archive.wiki}
          {data.wikiActive ? (
            <StatusBadge tone="ok">Activo</StatusBadge>
          ) : (
            <StatusBadge tone="neutral">{waiting > 0 ? "En pausa" : "Sin activar"}</StatusBadge>
          )}
        </>
      }
      action={
        isOpenProject ? (
          data.wikiActive ? (
            <Button variant="ghost" size="sm" onClick={() => toggle(false)} disabled={busy}>
              <Pause /> Pausar
            </Button>
          ) : (
            <Button size="sm" onClick={() => toggle(true)} disabled={busy}>
              <Play /> Activar
            </Button>
          )
        ) : undefined
      }
    >
      <div className="flex flex-col gap-3 text-sm">
        {problem && <ErrorNote>{problem}</ErrorNote>}
        {notice && <p className="rounded-md bg-muted px-3 py-2">{notice}</p>}

        <p className="text-muted-foreground">
          {words.archive.wikiDescription}{" "}
          {data.wikiActive
            ? "Se alimenta solo: lo que se agrega, cambia o se quita en las carpetas pasa al wiki."
            : waiting > 0
              ? "Está en pausa: no se manda nada al modelo hasta que lo actives de nuevo."
              : "Todavía no se armó: activarlo usa el modelo configurado, por eso no arranca solo."}
        </p>

        {!isOpenProject && (
          <p className="text-muted-foreground">
            Ahora está abierto otro proyecto. Para activarlo o pausarlo, volvé al proyecto «{words.archive.name}».
          </p>
        )}

        {data.origins.some((origin) => !origin.mountedAs) && (
          <ul className="flex flex-col gap-1">
            {data.origins
              .filter((origin) => !origin.mountedAs)
              .map((origin) => (
                <li key={origin.originId} className="text-xs text-destructive">
                  «{origin.name}» no se pudo usar como fuente del wiki{origin.problem ? `: ${origin.problem}` : ""}
                </li>
              ))}
          </ul>
        )}

        {isOpenProject && waiting > 0 && (
          <p>
            <StatusBadge tone={data.wikiActive ? "busy" : "neutral"}>
              {plural(waiting, "documento", "documentos")} en la cola{data.wikiActive ? "" : ", detenidos"}
            </StatusBadge>
          </p>
        )}

        {isOpenProject && pendingTotal > 0 && (
          <p>
            <StatusBadge tone={data.wikiActive ? "busy" : "neutral"}>
              {plural(pendingTotal, "documento", "documentos")} sin incorporar
            </StatusBadge>
          </p>
        )}
      </div>
    </Panel>
  )
}
