import type { AttentionNotice } from "@/lib/attention/types"
import {
  ignoreFileChangeTask,
  rescanProjectFiles,
  retryFileChangeTask,
  type FileChangeKind,
  type FileChangeTask,
} from "@/commands/file-sync"
import { getFileName, normalizePath } from "@/lib/path-utils"
import { useFileSyncStore } from "@/stores/file-sync-store"
import { useWikiStore } from "@/stores/wiki-store"
import { plural } from "@/components/docbase/format"
import { useOpenProjectScope } from "./wiki"

/**
 * The changes to the open project's files that the wiki has to learn
 * about (a source that appeared, changed or was deleted). Before, they
 * were at the foot of the wiki's side panel, in English.
 */
export const FILE_SYNC_SOURCE = "file-sync"

const CASES = 30

const KIND_WORDS: Record<FileChangeKind, string> = {
  created: "apareció",
  modified: "cambió",
  deleted: "se borró",
}

/** Runs an operation on the open project's change queue and keeps the store in step. */
async function onOpenProject(
  operation: (projectId: string, projectPath: string) => Promise<{ tasks: FileChangeTask[] }>,
): Promise<void> {
  const project = useWikiStore.getState().project
  if (!project) return
  const store = useFileSyncStore.getState()
  try {
    const queue = await operation(project.id, normalizePath(project.path))
    store.setTasks(queue.tasks)
    store.setLastError(null)
  } catch (error) {
    store.setLastError(String(error))
    throw error
  }
}

const retry = (taskId: string) => onOpenProject((id, path) => retryFileChangeTask(id, path, taskId))
const ignore = (taskId: string) => onOpenProject((id, path) => ignoreFileChangeTask(id, path, taskId))
const rescan = () =>
  onOpenProject(async (id, path) => (await rescanProjectFiles(id, path, useWikiStore.getState().sourceWatchConfig)).queue)

export function fileSyncNotices(
  tasks: readonly FileChangeTask[],
  lastError: string | null,
  scope: string | undefined,
): AttentionNotice[] {
  const out: AttentionNotice[] = []
  const failed = tasks.filter((task) => task.status === "failed")
  const waiting = tasks.filter((task) => task.status === "pending" || task.status === "processing")
  if (failed.length > 0) {
    out.push({
      source: FILE_SYNC_SOURCE,
      id: `${FILE_SYNC_SOURCE}:failed`,
      kind: "problem",
      topic: "file-changes-failed",
      priority: "normal",
      count: failed.length,
      title: plural(failed.length, "cambio de archivo no se pudo registrar", "cambios de archivos no se pudieron registrar"),
      explanation:
        "El wiki no se enteró de que esos archivos aparecieron, cambiaron o se borraron: sus páginas pueden decir algo que ya no es así.",
      todo: "Reintentá. Si no hace falta, ignoralo: ese cambio se pierde y el wiki sigue como estaba hasta que el archivo vuelva a cambiar.",
      resolvedWhen: "Desaparece cuando cada cambio se registra o se ignora.",
      actions: [{ kind: "run", label: "Volver a buscar cambios", run: rescan }],
      cases: failed.slice(0, CASES).map((task) => ({
        id: task.id,
        title: getFileName(task.path),
        detail: [`${KIND_WORDS[task.kind] ?? task.kind}`, task.error].filter(Boolean).join(" · "),
        actions: [
          { kind: "run", label: "Reintentar", run: () => retry(task.id) },
          {
            kind: "run",
            label: "Ignorar",
            danger: true,
            confirm: `Si lo ignorás, el cambio de «${getFileName(task.path)}» se pierde: el wiki no se entera de que ${KIND_WORDS[task.kind] ?? "cambió"}, y sus páginas siguen como estaban hasta que el archivo vuelva a cambiar.`,
            run: () => ignore(task.id),
          },
        ],
      })),
      moreCases: Math.max(0, failed.length - CASES),
      scope,
    })
  }
  if (lastError && failed.length === 0) {
    out.push({
      source: FILE_SYNC_SOURCE,
      id: `${FILE_SYNC_SOURCE}:error`,
      kind: "problem",
      topic: "file-changes-error",
      priority: "normal",
      count: 1,
      title: "No se pudieron buscar los cambios de los archivos del wiki",
      explanation: `Mientras tanto el wiki puede no enterarse de lo que cambia en sus fuentes. El error fue: ${lastError}`,
      todo: "Volvé a buscar cambios. Si sigue fallando, reiniciá la app.",
      resolvedWhen: "Desaparece cuando la búsqueda funciona.",
      actions: [{ kind: "run", label: "Volver a buscar cambios", run: rescan }],
      scope,
    })
  }
  if (waiting.length > 0) {
    out.push({
      source: FILE_SYNC_SOURCE,
      id: `${FILE_SYNC_SOURCE}:waiting`,
      kind: "ongoing",
      topic: "file-changes",
      priority: "normal",
      count: waiting.length,
      title: `Registrando ${plural(waiting.length, "cambio de archivo", "cambios de archivos")}`,
      explanation: "El wiki se entera de lo que apareció, cambió o se borró en sus fuentes.",
      todo: "No hace falta hacer nada.",
      resolvedWhen: "Desaparece cuando termina de registrarlos.",
      actions: [],
      scope,
    })
  }
  return out
}

export function useFileSyncNotices(): AttentionNotice[] {
  const tasks = useFileSyncStore((s) => s.tasks)
  const lastError = useFileSyncStore((s) => s.lastError)
  return fileSyncNotices(tasks, lastError, useOpenProjectScope())
}
