import { useEffect, useState } from "react"
import { BookOpen, Link2, Unlink } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  getGeneralProject,
  unlinkGeneralProject,
  type BridgedOrigin,
  type WikiSource,
} from "@/commands/docbase"
import {
  ingestWikiSources,
  isSameProjectPath,
  linkOpenProjectAsGeneral,
  pendingWikiSourcesByOrigin,
} from "@/lib/docbase-wiki-bridge"
import { useAppDialog } from "@/stores/app-dialog-store"
import { useWikiStore } from "@/stores/wiki-store"
import { ErrorNote, Panel, StatusBadge } from "./page"
import { plural } from "./format"
import { useDocbaseQuery } from "./use-docbase-query"

/**
 * The general project of the space: the wiki whose sources are all the
 * folders of the base. Assigning a wiki copies nothing and ingests
 * nothing; ingesting is asked for here, origin by origin.
 */
export function GeneralProjectPanel() {
  const general = useDocbaseQuery("general-project", getGeneralProject)
  const project = useWikiStore((s) => s.project)
  const dataVersion = useWikiStore((s) => s.dataVersion)
  const dialog = useAppDialog()
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

  if (!data) return general.error ? <ErrorNote>{general.error}</ErrorNote> : null

  async function handleLink() {
    if (!project) return
    const confirmed = await dialog.confirm({
      title: `Usar "${project.name}" como proyecto general`,
      message:
        "Las carpetas de la base pasan a ser las fuentes de este wiki, sin copiar ningún archivo. " +
        "Todavía no se incorpora nada: lo pedís vos después, carpeta por carpeta. " +
        "La incorporación automática de archivos nuevos queda desactivada para este proyecto; se puede volver a activar en Configuración.",
      confirmLabel: "Usar este proyecto",
      cancelLabel: "Cancelar",
    })
    if (!confirmed) return
    setBusy(true)
    setProblem(null)
    setNotice(null)
    try {
      await linkOpenProjectAsGeneral(project)
      general.reload()
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }

  async function handleUnlink() {
    const confirmed = await dialog.confirm({
      title: "Dejar de usarlo como proyecto general",
      message:
        "Las carpetas de la base dejan de ser fuentes de este wiki. Las páginas que ya se generaron quedan como están.",
      confirmLabel: "Dejar de usarlo",
      cancelLabel: "Cancelar",
      variant: "destructive",
    })
    if (!confirmed) return
    try {
      await unlinkGeneralProject()
      general.reload()
    } catch (cause) {
      setProblem(String(cause))
    }
  }

  async function handleIngest(origin: BridgedOrigin) {
    if (!project) return
    const sources = pending.get(origin.originId) ?? []
    if (sources.length === 0) return
    const confirmed = await dialog.confirm({
      title: `Incorporar "${origin.name}" al wiki`,
      message:
        `Se van a incorporar ${plural(sources.length, "documento", "documentos")} al wiki de "${project.name}". ` +
        "Cada documento pasa por el modelo configurado, así que consume cupo o costo de ese proveedor. " +
        "Las copias de un mismo documento se incorporan una sola vez. El avance se ve en el panel de actividad del proyecto.",
      confirmLabel: "Incorporar",
      cancelLabel: "Cancelar",
    })
    if (!confirmed) return
    setBusy(true)
    setProblem(null)
    try {
      const queued = await ingestWikiSources(project, sources)
      setNotice(
        queued > 0
          ? `${plural(queued, "documento quedó", "documentos quedaron")} en la cola de incorporación.`
          : "No se encoló nada: o no hay un modelo configurado para incorporar, o todos eran copias de documentos ya incorporados.",
      )
      setPending(await pendingWikiSourcesByOrigin(project))
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
          <BookOpen className="size-4" /> Proyecto general
        </>
      }
      action={
        data.path ? (
          <Button variant="ghost" size="sm" onClick={handleUnlink} disabled={busy}>
            <Unlink /> Dejar de usarlo
          </Button>
        ) : undefined
      }
    >
      <div className="flex flex-col gap-3 text-sm">
        {problem && <ErrorNote>{problem}</ErrorNote>}
        {notice && <p className="rounded-md bg-muted px-3 py-2">{notice}</p>}

        {!data.path && (
          <>
            <p className="text-muted-foreground">
              Es el wiki que abarca todas las carpetas de la base. Todavía no tiene un wiki asignado.
            </p>
            {project && (
              <div>
                <Button variant="outline" onClick={handleLink} disabled={busy}>
                  <Link2 /> Usar "{project.name}" como proyecto general
                </Button>
              </div>
            )}
          </>
        )}

        {data.path && (
          <>
            <div>
              <span className="font-medium">{data.name}</span>
              <div className="font-mono text-xs break-all text-muted-foreground">{data.path}</div>
            </div>
            {!isOpenProject && (
              <p className="text-muted-foreground">
                No es el proyecto que está abierto. Para incorporar documentos a su wiki, abrí ese proyecto.
              </p>
            )}
            <ul className="flex flex-col divide-y rounded-md border">
              {data.origins.map((origin) => {
                const count = pending.get(origin.originId)?.length ?? 0
                return (
                  <li key={origin.originId} className="flex flex-wrap items-center justify-between gap-2 px-3 py-2">
                    <div className="min-w-0">
                      <div className="font-medium">{origin.name}</div>
                      {origin.mountedAs ? (
                        <div className="text-xs text-muted-foreground">
                          Fuente del wiki como raw/sources/{origin.mountedAs}
                        </div>
                      ) : (
                        <div className="text-xs text-destructive">
                          No se pudo usar como fuente{origin.problem ? `: ${origin.problem}` : ""}
                        </div>
                      )}
                    </div>
                    {isOpenProject && origin.mountedAs && (
                      <div className="flex items-center gap-2">
                        {count === 0 ? (
                          <StatusBadge tone="ok">Nada pendiente de incorporar</StatusBadge>
                        ) : (
                          <Button variant="outline" size="sm" onClick={() => handleIngest(origin)} disabled={busy}>
                            Incorporar al wiki ({count})
                          </Button>
                        )}
                      </div>
                    )}
                  </li>
                )
              })}
            </ul>
          </>
        )}
      </div>
    </Panel>
  )
}
