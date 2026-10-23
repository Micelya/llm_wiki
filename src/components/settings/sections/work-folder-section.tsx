import { useCallback, useEffect, useState } from "react"
import { open } from "@tauri-apps/plugin-dialog"
import { FolderOpen } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useAppDialog } from "@/stores/app-dialog-store"
import { vocabulary } from "@/vocabulary"
import {
  getClipServerState,
  getWorkFolder,
  openWorkFolder,
  setWorkFolder,
  type ClipServerState,
  type WorkFolderStatus,
} from "@/commands/work-folder"

const SERVER_STATE: Record<ClipServerState, string> = {
  starting: "Arrancando…",
  running: "Esperando recortes en esta computadora (puerto 19827).",
  "port-in-use": "No pudo arrancar: otro programa usa el puerto 19827. Cerralo y volvé a abrir Micelya.",
  stopped: "Se detuvo. Volvé a abrir Micelya.",
}

/**
 * Ajustes → Carpeta de trabajo (D-450): where Micelya keeps what it brings
 * by itself, and the web clipper that saves pages there. Changes apply at
 * once, without the Save bar.
 */
export function WorkFolderSection() {
  const dialog = useAppDialog()
  const words = vocabulary()
  const [status, setStatus] = useState<WorkFolderStatus | null>(null)
  const [server, setServer] = useState<ClipServerState | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const load = useCallback(async () => {
    try {
      setStatus(await getWorkFolder())
    } catch (cause) {
      setProblem(String(cause))
    }
    getClipServerState().then(setServer).catch(() => setServer(null))
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  async function change() {
    setProblem(null)
    const selected = await open({ directory: true, multiple: false, title: "Elegí la carpeta de trabajo de Micelya" })
    if (typeof selected !== "string") return
    const confirmed = await dialog.confirm({
      title: "Cambiar la carpeta de trabajo",
      message:
        `La base va a leer ${selected} en lugar de la carpeta actual, y ahí van a ir los recortes y los expedientes nuevos. ` +
        "Lo que quede en la carpeta anterior deja de estar en la base (los archivos no se tocan): si querés conservarlo, " +
        "movelo antes a la carpeta nueva con las mismas subcarpetas.",
      confirmLabel: "Cambiar",
      cancelLabel: "Cancelar",
    })
    if (!confirmed) return
    setBusy(true)
    try {
      setStatus(await setWorkFolder(selected))
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setBusy(false)
    }
  }

  const openFolder = (which?: "clips" | "records" | "generated") => {
    setProblem(null)
    openWorkFolder(which).catch((cause) => setProblem(String(cause)))
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="font-ui text-[1.75rem] leading-tight font-bold tracking-[-0.01em] text-strong">Carpeta de trabajo</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          La carpeta donde Micelya guarda lo que trae por su cuenta: los recortes de páginas web, los expedientes de los
          portales y los documentos que genera desde una {words.templates.one}. Es un origen de la base: lo que se guarda
          ahí se lee como cualquier documento. Los cambios se aplican al momento.
        </p>
      </div>

      <div className="space-y-3 rounded-md border p-4">
        <div className="flex items-start gap-3">
          <FolderOpen className="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground" />
          <div className="min-w-0 flex-1 space-y-1">
            <div className="break-all font-mono text-sm">{status?.path ?? "…"}</div>
            {status && !status.error && (
              <p className="text-xs text-muted-foreground">
                {status.originOwn
                  ? `Es el origen «${status.originName}» de la base.`
                  : `Está dentro del origen «${status.originName}»: la base ya la lee.`}
              </p>
            )}
            {status?.error && <p className="text-xs text-destructive">No es un origen de la base: {status.error}</p>}
          </div>
        </div>
        {status && (
          <ul className="space-y-1 pl-8 text-sm">
            <li className="flex items-center justify-between gap-2">
              <span>
                «Recortes web» <span className="text-xs text-muted-foreground">— las páginas guardadas con Micelya Clipper</span>
              </span>
              <Button variant="ghost" size="sm" onClick={() => openFolder("clips")}>
                Abrir
              </Button>
            </li>
            <li className="flex items-center justify-between gap-2">
              <span>
                «Expedientes» <span className="text-xs text-muted-foreground">— lo que se trae de «Portales»</span>
              </span>
              <Button variant="ghost" size="sm" onClick={() => openFolder("records")}>
                Abrir
              </Button>
            </li>
            <li className="flex items-center justify-between gap-2">
              <span>
                «Generados»{" "}
                <span className="text-xs text-muted-foreground">
                  — los documentos generados desde «{words.templates.name}»
                </span>
              </span>
              <Button variant="ghost" size="sm" onClick={() => openFolder("generated")}>
                Abrir
              </Button>
            </li>
          </ul>
        )}
        <div className="flex flex-wrap gap-2 pl-8">
          <Button variant="outline" size="sm" onClick={() => openFolder()}>
            Abrir la carpeta
          </Button>
          <Button variant="outline" size="sm" onClick={() => void change()} disabled={busy}>
            {busy ? "Cambiando…" : "Cambiar…"}
          </Button>
        </div>
        {problem && <p className="pl-8 text-xs text-destructive">{problem}</p>}
      </div>

      <div className="space-y-2">
        <h3 className="font-ui text-[0.9375rem] font-bold text-strong">Recortes web desde el navegador</h3>
        <p className="text-sm text-muted-foreground">
          Con la extensión «Micelya Clipper», la página que estás leyendo se guarda como un archivo en «Recortes web», con
          su dirección y la fecha al principio; la base la lee y «Atención» avisa de los recortes nuevos. Micelya tiene que
          estar abierta.
        </p>
        <p className="text-sm">
          <span className="text-muted-foreground">El recortador: </span>
          {server ? SERVER_STATE[server] : "no se sabe cómo está."}
        </p>
        <ol className="list-decimal space-y-1 pl-5 text-sm text-muted-foreground">
          <li>En Chrome o Edge, abrí chrome://extensions (en Edge, edge://extensions).</li>
          <li>Activá «Modo de desarrollador».</li>
          <li>
            Tocá «Cargar descomprimida» y elegí la carpeta <span className="font-mono">extension</span> de Micelya Desktop.
          </li>
          <li>En cualquier página, tocá el ícono de Micelya Clipper o usá Alt+Mayús+L.</li>
        </ol>
      </div>
    </div>
  )
}
