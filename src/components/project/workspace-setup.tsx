import { useEffect, useState } from "react"
import { open } from "@tauri-apps/plugin-dialog"
import { FolderOpen } from "lucide-react"
import { Button } from "@/components/ui/button"
import { PRODUCT_NAME } from "@/lib/app-identity"
import { suggestWorkspaceDir } from "@/lib/default-project"

/**
 * First run: asks where the app keeps the projects it creates by
 * itself. Shown once; the answer is remembered.
 */
export function WorkspaceSetup({
  onChosen,
  onSkip,
}: {
  onChosen: (dir: string) => Promise<void>
  onSkip: () => void
}) {
  const [dir, setDir] = useState("")
  const [busy, setBusy] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  useEffect(() => {
    suggestWorkspaceDir()
      .then(setDir)
      .catch(() => {})
  }, [])

  async function pick() {
    const selected = await open({ directory: true, multiple: false, title: "Carpeta de trabajo" })
    if (typeof selected === "string") setDir(selected)
  }

  async function confirm() {
    setBusy(true)
    setProblem(null)
    try {
      await onChosen(dir)
    } catch (cause) {
      setProblem(String(cause))
      setBusy(false)
    }
  }

  return (
    <div className="flex h-full items-center justify-center bg-background">
      <div className="flex w-full max-w-lg flex-col gap-5 px-6">
        <div>
          <h1 className="text-2xl font-bold">{PRODUCT_NAME}</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            Elegí la carpeta de trabajo: ahí la app guarda lo que genera (las páginas del wiki y sus datos).
            Tus documentos no se mueven ni se copian ahí. Conviene una carpeta local, que no esté en un disco de
            red ni sincronizada con la nube.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <div className="min-w-0 flex-1 rounded-lg border px-3 py-2 font-mono text-sm break-all">{dir || "…"}</div>
          <Button variant="outline" onClick={pick} disabled={busy}>
            <FolderOpen /> Cambiar
          </Button>
        </div>
        {problem && <p className="text-sm text-destructive">{problem}</p>}
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onSkip} disabled={busy}>
            Abrir otro proyecto
          </Button>
          <Button onClick={confirm} disabled={busy || !dir}>
            Usar esta carpeta
          </Button>
        </div>
      </div>
    </div>
  )
}
