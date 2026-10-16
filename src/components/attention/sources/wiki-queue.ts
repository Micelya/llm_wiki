import { useEffect, useState } from "react"
import type { AttentionCase, AttentionNotice } from "@/lib/attention/types"
import {
  cancelAllTasks,
  cancelTask,
  getQueue,
  getQueueSummary,
  movePendingTask,
  pauseProcessing,
  removeStoppedTasks,
  resumeProcessing,
  retryAllFailedTasks,
  retryTask,
  retryTasks,
  type IngestTask,
} from "@/lib/ingest-queue"
import { getFileName } from "@/lib/path-utils"
import { useActivityStore, type ActivityItem } from "@/stores/activity-store"
import { vocabulary, type Vocabulary } from "@/vocabulary"
import { formatCount, plural } from "@/components/docbase/format"
import { useOpenProjectScope } from "./wiki"

/**
 * The wiki's queue: what it is incorporating, what failed and why, and
 * that it stopped because no model is configured. It replaces the old
 * activity panel at the foot of the wiki's side panel, and it is in
 * Spanish and seen from any screen.
 */
export const QUEUE_SOURCE = "wiki-queue"

/** How often the queue is looked at: it keeps its state in memory, without events. */
export const QUEUE_POLL_MS = 1000
/** Tasks listed when a line is opened. */
export const QUEUE_CASES = 30

export type QueueSummary = ReturnType<typeof getQueueSummary>

const LLM_NOT_CONFIGURED = /LLM not configured/i

/** The error of a task, in words a person understands, with what to do about it. */
export function ingestErrorInWords(error: string | null): string {
  if (!error) return "No dio un motivo."
  const text = error.trim()
  if (LLM_NOT_CONFIGURED.test(text)) return "Falta configurar el modelo que escribe el wiki."
  if (/\b429\b|rate[_\s-]*limit|usage\s+limit|quota|too many requests/i.test(text)) {
    return "El proveedor del modelo llegó a su límite de uso. Se vuelve a intentar solo en unos minutos."
  }
  if (/timed?\s*out|timeout|ETIMEDOUT/i.test(text)) return "El modelo tardó demasiado en responder."
  if (/fetch failed|network|ECONNREFUSED|ENOTFOUND|connection/i.test(text)) {
    return "No se pudo conectar con el modelo. Revisá la conexión a internet o que el proveedor esté disponible."
  }
  if (/\b401\b|\b403\b|unauthori[sz]ed|invalid api key|forbidden/i.test(text)) {
    return "El proveedor rechazó la clave del modelo. Revisala en la configuración."
  }
  if (/Project not found/i.test(text)) return "No se encontró el proyecto del wiki (¿se borró o se movió su carpeta?)."
  if (/ENOENT|no such file|not found/i.test(text)) return "No se encontró el archivo: puede haberse movido o borrado."
  return `El modelo o la lectura del documento fallaron: ${text}`
}

export interface QueueFacts {
  summary: QueueSummary
  tasks: readonly IngestTask[]
  activity: readonly ActivityItem[]
  scope: string | undefined
}

export function queueNotices(facts: QueueFacts, words: Vocabulary = vocabulary()): AttentionNotice[] {
  const { summary, tasks, scope } = facts
  const out: AttentionNotice[] = []
  const wiki = words.archive.wiki.toLowerCase()
  const notice = (patch: Omit<AttentionNotice, "source" | "id" | "scope">): AttentionNotice => ({
    source: QUEUE_SOURCE,
    id: `${QUEUE_SOURCE}:${patch.topic}`,
    scope,
    ...patch,
  })
  const pending = tasks.filter((task) => task.status === "pending")
  const processing = tasks.filter((task) => task.status === "processing")
  const failed = tasks.filter((task) => task.status === "failed")
  const cancelled = tasks.filter((task) => task.status === "cancelled")

  // ── Problems ────────────────────────────────────────────────────────
  if (summary.blockedOnLlmConfig && pending.length > 0) {
    out.push(
      notice({
        kind: "problem",
        topic: "llm-config",
        priority: "high",
        count: pending.length,
        title: "Falta configurar el modelo que escribe el wiki",
        explanation: `La incorporación al wiki está detenida: ${plural(pending.length, "documento espera", "documentos esperan")} un modelo que los lea.`,
        todo: "Elegí un proveedor y un modelo en la configuración; después reanudá la incorporación.",
        resolvedWhen: "Desaparece cuando hay un modelo configurado y la incorporación sigue.",
        actions: [
          { kind: "go", label: "Ir a la configuración", target: { view: "settings" } },
          { kind: "run", label: "Reanudar", run: () => resumeProcessing() },
        ],
      }),
    )
  }
  if (failed.length > 0) {
    const reasons = new Map<string, number>()
    for (const task of failed) {
      const reason = ingestErrorInWords(task.error)
      reasons.set(reason, (reasons.get(reason) ?? 0) + 1)
    }
    const [mostCommon] = [...reasons.entries()].sort((a, b) => b[1] - a[1])[0]
    out.push(
      notice({
        kind: "problem",
        topic: "ingest-failed",
        priority: "normal",
        count: failed.length,
        title: plural(failed.length, "documento no se pudo incorporar al wiki", "documentos no se pudieron incorporar al wiki"),
        explanation:
          reasons.size === 1
            ? `${mostCommon} Lo que dicen esos documentos todavía no está en el ${wiki}.`
            : `El motivo más común: ${mostCommon} Lo que dicen esos documentos todavía no está en el ${wiki}.`,
        todo: "Reintentá cuando el motivo esté resuelto, o quitá de la cola los que no hace falta incorporar.",
        resolvedWhen: "Desaparece cuando se incorporan o se quitan de la cola.",
        actions: [{ kind: "run", label: "Reintentar todos", run: () => retryAllFailedTasks() }],
        cases: failed.slice(0, QUEUE_CASES).map((task) => ({
          id: task.id,
          title: getFileName(task.sourcePath),
          detail: ingestErrorInWords(task.error),
          actions: [
            { kind: "run", label: "Reintentar", run: () => retryTask(task.id) },
            {
              kind: "run",
              label: "Quitar",
              danger: true,
              confirm: `«${getFileName(task.sourcePath)}» sale de la cola y no se incorpora al wiki hasta que se vuelva a encolar (por ejemplo, al cambiar el archivo). El documento no se toca.`,
              run: () => removeStoppedTasks([task.id]),
            },
          ],
        })),
        moreCases: Math.max(0, failed.length - QUEUE_CASES),
      }),
    )
  }

  // ── In progress ─────────────────────────────────────────────────────
  const active = pending.length + processing.length
  if (active > 0) {
    const done = summary.completed + summary.failed + summary.cancelled
    const stopped = summary.paused || summary.blockedOnLlmConfig
    out.push(
      notice({
        kind: "ongoing",
        topic: "ingest",
        priority: "normal",
        count: active,
        title: stopped
          ? `Incorporación al wiki en pausa: ${formatCount(done)} de ${plural(summary.total, "documento", "documentos")}`
          : `Incorporando al wiki: ${formatCount(done)} de ${plural(summary.total, "documento", "documentos")}`,
        explanation: stopped
          ? `${plural(active, "documento espera", "documentos esperan")}. Mientras esté en pausa no se usa el modelo.`
          : `El modelo lee cada documento y escribe sus páginas. ${plural(active, "documento falta", "documentos faltan")}.`,
        todo: stopped ? "Reanudá cuando quieras seguir." : "No hace falta hacer nada. Podés pausarla para dejar de usar el modelo.",
        resolvedWhen: "Desaparece cuando la cola termina.",
        progress: { done, total: summary.total },
        actions: [
          stopped
            ? { kind: "run", label: "Reanudar", run: () => resumeProcessing() }
            : { kind: "run", label: "Pausar", run: () => pauseProcessing() },
          ...(active >= 2
            ? [
                {
                  kind: "run" as const,
                  label: "Cancelar todo",
                  danger: true,
                  confirm: `Se cancelan ${plural(active, "documento", "documentos")} de la cola. Quedan en ella como cancelados y se pueden volver a encolar. Lo que el modelo ya escribió se conserva.`,
                  run: () => cancelAllTasks(),
                },
              ]
            : []),
        ],
        cases: [...processing, ...pending].slice(0, QUEUE_CASES).map((task, index) => queuedCase(task, index - processing.length)),
        moreCases: Math.max(0, active - QUEUE_CASES),
      }),
    )
  }
  if (cancelled.length > 0) {
    out.push(
      notice({
        kind: "ongoing",
        topic: "ingest-cancelled",
        priority: "low",
        count: cancelled.length,
        title: plural(cancelled.length, "documento cancelado quedó en la cola", "documentos cancelados quedaron en la cola"),
        explanation: "Se cancelaron a pedido: no se incorporan al wiki mientras sigan así.",
        todo: "Volvé a encolarlos para incorporarlos, o quitalos de la cola.",
        resolvedWhen: "Desaparece cuando se vuelven a encolar o se quitan.",
        actions: [
          { kind: "run", label: "Volver a encolar", run: () => retryTasks(cancelled.map((task) => task.id)) },
          {
            kind: "run",
            label: "Quitar de la cola",
            danger: true,
            confirm: "Salen de la cola y no se incorporan al wiki hasta que se vuelvan a encolar. Los documentos no se tocan.",
            run: () => removeStoppedTasks(cancelled.map((task) => task.id)),
          },
        ],
        cases: cancelled.slice(0, QUEUE_CASES).map((task) => ({
          id: task.id,
          title: getFileName(task.sourcePath),
          actions: [{ kind: "run", label: "Volver a encolar", run: () => retryTask(task.id) }],
        })),
        moreCases: Math.max(0, cancelled.length - QUEUE_CASES),
      }),
    )
  }

  // What else the wiki is doing with the model, outside the queue.
  for (const item of facts.activity) {
    if (item.status !== "running" || item.type === "ingest") continue
    const what =
      item.type === "lint"
        ? {
            title: "Revisando el wiki con el modelo",
            explanation: "Busca contradicciones y huecos entre las páginas, a pedido de una persona.",
          }
        : {
            title: "Repasando las propuestas del wiki",
            explanation: "El modelo mira si las propuestas pendientes ya quedaron resueltas por lo que se incorporó.",
          }
    out.push(
      notice({
        kind: "ongoing",
        topic: `activity-${item.id}`,
        priority: "normal",
        count: 1,
        ...what,
        todo: "No hace falta hacer nada.",
        resolvedWhen: "Desaparece cuando termina.",
        actions: [],
      }),
    )
  }
  return out
}

/** A task waiting or running, with what can be done with it. `position` < 0 while it runs. */
function queuedCase(task: IngestTask, position: number): AttentionCase {
  const running = task.status === "processing"
  return {
    id: task.id,
    title: getFileName(task.sourcePath),
    detail: running ? "Incorporándose ahora" : task.folderContext || undefined,
    actions: [
      ...(!running && position > 0
        ? [{ kind: "run" as const, label: "Subir", run: () => movePendingTask(task.id, "up") }]
        : []),
      { kind: "run", label: "Cancelar", run: () => cancelTask(task.id) },
    ],
  }
}

/** The source itself: looks at the queue every second, as the old activity panel did. */
export function useQueueNotices(): AttentionNotice[] {
  const [tick, setTick] = useState(0)
  useEffect(() => {
    const timer = setInterval(() => setTick((value) => value + 1), QUEUE_POLL_MS)
    return () => clearInterval(timer)
  }, [])
  const activity = useActivityStore((s) => s.items)
  const scope = useOpenProjectScope()
  void tick
  return queueNotices({ summary: getQueueSummary(), tasks: [...getQueue()], activity, scope })
}
