import { useCallback, useEffect, useState } from "react"
import { Archive, FolderOpen } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useAppDialog } from "@/stores/app-dialog-store"
import {
  closeApp,
  getDataStatus,
  openDataFolder,
  restoreBackup,
  type BackupSummary,
  type DataStatus,
} from "@/commands/docbase"

/** A size in words: «47,3 MB», «12 KB». */
export function sizeInWords(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1).replace(".", ",")} MB`
  return `${Math.max(1, Math.round(bytes / 1024))} KB`
}

/** The backups this version of the app can go back to. */
export function backupsToGoBackTo(status: DataStatus): BackupSummary[] {
  return status.backups.filter((backup) => backup.complete && backup.baseVersion <= status.appBaseVersion)
}

interface DataViewProps {
  status: DataStatus | null
  problem: string | null
  busy: boolean
  onOpen: (which: "data" | "backups") => void
  onRestore: (backup: BackupSummary) => void
  onClose: () => void
}

/** What «Datos y respaldos» shows, from the status of the data. */
export function DataView({ status, problem, busy, onOpen, onRestore, onClose }: DataViewProps) {
  const opening = status?.opening
  const goBack = status && opening?.newer && !status.restored ? backupsToGoBackTo(status) : []
  return (
    <div className="space-y-6">
      <div>
        <h2 className="font-ui text-[1.75rem] leading-tight font-bold tracking-[-0.01em] text-strong">Datos y respaldos</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Dónde guarda Micelya lo que hacés en la app (la base, los ajustes y el chat), separado del programa: instalar una
          versión nueva no lo toca. Antes de cada actualización que cambia la forma de la base, la app guarda sola un
          respaldo.
        </p>
      </div>

      {status?.restored && (
        <div className="space-y-2 rounded-md border border-ok-line bg-ok-bg p-4 text-sm" data-data="restored">
          <p>
            Se volvió al respaldo «{status.restored}». Para usarlo, cerrá Micelya y volvé a abrirla. La base que estaba en
            uso quedó aparte en la carpeta de respaldos.
          </p>
          <Button size="sm" onClick={onClose}>
            Cerrar Micelya
          </Button>
        </div>
      )}

      {opening?.problem && !status?.restored && (
        <div className="rounded-md border border-err-line bg-destructive/5 p-4 text-sm" data-data="problem">
          <p className="font-medium text-destructive">La base no se pudo abrir</p>
          <p className="mt-1">{opening.problem}</p>
        </div>
      )}

      {status && opening?.newer && !status.restored && (
        <section className="space-y-3 rounded-md border p-4 text-sm" data-data="go-back">
          <h3 className="font-ui text-[0.9375rem] font-bold text-strong">Volver a la versión de antes</h3>
          <p className="text-muted-foreground">
            Esta versión de Micelya ({status.appVersion}) es más vieja que la que escribió la base. Para seguir con esta
            versión, volvé al respaldo que se hizo antes de la actualización: se pierde lo que se hizo en la app después de
            su fecha. La base de ahora no se borra: queda aparte en la carpeta de respaldos.
          </p>
          {goBack.length === 0 ? (
            <p>
              No hay un respaldo que esta versión pueda abrir. Instalá de nuevo la versión más nueva de Micelya: tus datos
              siguen ahí.
            </p>
          ) : (
            <ul className="space-y-2">
              {goBack.map((backup, index) => (
                <li key={backup.name} className="flex items-center justify-between gap-3">
                  <span>
                    {backup.madeAt} — la base en la versión {backup.baseVersion}
                    {index === 0 && <span className="text-muted-foreground"> (el más reciente)</span>}
                  </span>
                  <Button size="sm" variant="outline" disabled={busy} onClick={() => onRestore(backup)}>
                    Volver a este respaldo
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      <section className="space-y-3 rounded-md border p-4" data-data="folder">
        <h3 className="font-ui text-[0.9375rem] font-bold text-strong">Tus datos</h3>
        <div className="flex items-start gap-3">
          <FolderOpen className="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground" />
          <div className="min-w-0 flex-1 space-y-1 text-sm">
            <div className="break-all font-mono">{status?.dataFolder ?? "…"}</div>
            {status && (
              <p className="text-muted-foreground">
                Versión de la base: {status.baseVersion ?? "sin base"}
                {status.baseVersion === status.appBaseVersion
                  ? ` (la de esta versión de la app, ${status.appVersion})`
                  : `; esta versión de la app (${status.appVersion}) usa la ${status.appBaseVersion}`}
                .
              </p>
            )}
            {opening?.backup && (
              <p data-data="updated">
                Al abrir, la app actualizó la base de la versión {opening.foundVersion} a la {status?.appBaseVersion}.
                Antes guardó el respaldo «{opening.backup}».
                {opening.pruned.length > 0 &&
                  ` Se ${opening.pruned.length === 1 ? "borró el respaldo más viejo" : `borraron los ${opening.pruned.length} respaldos más viejos`}.`}
              </p>
            )}
          </div>
        </div>
        <div className="pl-8">
          <Button variant="outline" size="sm" onClick={() => onOpen("data")} disabled={!status?.dataFolder}>
            Abrir la carpeta de datos
          </Button>
        </div>
      </section>

      <section className="space-y-3 rounded-md border p-4" data-data="backups">
        <h3 className="font-ui text-[0.9375rem] font-bold text-strong">Respaldos</h3>
        <div className="flex items-start gap-3">
          <Archive className="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground" />
          <div className="min-w-0 flex-1 space-y-2 text-sm">
            <p className="text-muted-foreground">
              Antes de cada actualización que cambia la forma de la base, la app copia sola la base, los ajustes y el chat,
              con la huella de cada archivo. Si no puede hacer el respaldo, no actualiza nada. Se conservan los{" "}
              {status?.keep ?? 3} últimos; uno viejo se borra recién cuando hay uno nuevo y la actualización salió bien.
            </p>
            <div className="break-all font-mono">{status?.backupsFolder ?? "…"}</div>
            {status && status.backups.length === 0 && (
              <p>Todavía no hay respaldos: el primero se hace cuando una actualización cambie la forma de la base.</p>
            )}
            {status && status.backups.length > 0 && (
              <ul className="space-y-1">
                {status.backups.map((backup) => (
                  <li key={backup.name}>
                    <span>{backup.madeAt}</span>
                    <span className="text-muted-foreground">
                      {" "}
                      — la base en la versión {backup.baseVersion}, antes de pasar a la {backup.forVersion} ·{" "}
                      {sizeInWords(backup.bytes)}
                      {backup.writtenByApp && ` · la había escrito la app ${backup.writtenByApp}`}
                    </span>
                    {!backup.complete && <span className="text-destructive"> · incompleto: le faltan archivos</span>}
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
        <div className="pl-8">
          <Button variant="outline" size="sm" onClick={() => onOpen("backups")} disabled={!status?.backupsFolder}>
            Abrir la carpeta de respaldos
          </Button>
        </div>
      </section>

      <p className="text-xs text-muted-foreground">
        Al desinstalar Micelya, no marques «Eliminar los datos de aplicación» (en inglés, «Delete the application data»):
        borra tus datos y también los respaldos.
      </p>

      {problem && <p className="text-xs text-destructive">{problem}</p>}
    </div>
  )
}

/**
 * Ajustes → Datos y respaldos (D-496, P-297): where Micelya keeps what a
 * person does, the version of the base, and the backups the app makes by
 * itself before an update changes the shape of the base. When this version
 * cannot open the base because a newer one wrote it, it offers to go back
 * to a backup (`src-tauri/src/docbase/upgrade.rs`).
 */
export function DataSection() {
  const dialog = useAppDialog()
  const [status, setStatus] = useState<DataStatus | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const load = useCallback(async () => {
    try {
      setStatus(await getDataStatus())
    } catch (cause) {
      setProblem(String(cause))
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const open = (which: "data" | "backups") => {
    setProblem(null)
    openDataFolder(which).catch((cause) => setProblem(String(cause)))
  }

  async function restore(backup: BackupSummary) {
    setProblem(null)
    const confirmed = await dialog.confirm({
      title: "Volver a este respaldo",
      message:
        `La base vuelve a como estaba el ${backup.madeAt}: se pierde lo que se hizo en la app después de esa fecha. ` +
        "La base de ahora no se borra: queda aparte en la carpeta de respaldos. Después hay que cerrar Micelya y volver a abrirla.",
      confirmLabel: "Volver a este respaldo",
      cancelLabel: "Cancelar",
    })
    if (!confirmed) return
    setBusy(true)
    try {
      await restoreBackup(backup.name)
      await load()
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }

  const close = () => {
    closeApp().catch((cause) => setProblem(String(cause)))
  }

  return (
    <DataView
      status={status}
      problem={problem}
      busy={busy}
      onOpen={open}
      onRestore={(backup) => void restore(backup)}
      onClose={close}
    />
  )
}
