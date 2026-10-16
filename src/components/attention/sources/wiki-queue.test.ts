import { describe, expect, it, vi } from "vitest"

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }))

import type { IngestTask } from "@/lib/ingest-queue"
import type { FileChangeTask } from "@/commands/file-sync"
import type { ActivityItem } from "@/stores/activity-store"
import { vocabularyFor } from "@/vocabulary"
import { activityLine } from "@/components/layout/activity-link"
import { ingestErrorInWords, queueNotices, type QueueSummary } from "./wiki-queue"
import { fileSyncNotices } from "./file-sync"

const words = vocabularyFor("legal", "es")

function task(id: string, status: IngestTask["status"], error: string | null = null): IngestTask {
  return {
    id,
    projectId: "p",
    sourcePath: `raw/sources/casos/${id}.pdf`,
    folderContext: "",
    status,
    addedAt: 0,
    error,
    retryCount: 0,
  }
}

function summary(tasks: IngestTask[], patch: Partial<QueueSummary> = {}): QueueSummary {
  const count = (status: IngestTask["status"]) => tasks.filter((t) => t.status === status).length
  return {
    pending: count("pending"),
    processing: count("processing"),
    failed: count("failed"),
    cancelled: count("cancelled"),
    completed: 0,
    total: tasks.length,
    paused: false,
    userPaused: false,
    blockedOnLlmConfig: false,
    ...patch,
  }
}

function notices(tasks: IngestTask[], patch: Partial<QueueSummary> = {}, activity: ActivityItem[] = []) {
  return queueNotices({ summary: summary(tasks, patch), tasks, activity, scope: undefined }, words)
}

describe("the wiki's queue in the attention view", () => {
  it("says nothing when the queue is empty", () => {
    expect(notices([])).toEqual([])
  })

  it("shows the progress as something in progress, with pause and cancel", () => {
    const tasks = [task("a", "processing"), task("b", "pending"), task("c", "pending")]
    const [line] = notices(tasks, { completed: 9, total: 12 })
    expect(line).toMatchObject({ kind: "ongoing", topic: "ingest", progress: { done: 9, total: 12 } })
    expect(line.title).toBe("Incorporando al wiki: 9 de 12 documentos")
    expect(line.actions.map((a) => a.label)).toEqual(["Pausar", "Cancelar todo"])
    expect(line.cases?.map((c) => [c.title, c.actions?.map((a) => a.label)])).toEqual([
      ["a.pdf", ["Cancelar"]],
      ["b.pdf", ["Cancelar"]],
      ["c.pdf", ["Subir", "Cancelar"]],
    ])
  })

  it("offers to resume a paused queue", () => {
    const [line] = notices([task("a", "pending")], { paused: true, userPaused: true })
    expect(line.title.startsWith("Incorporación al wiki en pausa")).toBe(true)
    expect(line.actions.map((a) => a.label)).toEqual(["Reanudar"])
  })

  it("raises the problem no one saw: the queue stopped because no model is configured", () => {
    const lines = notices([task("a", "pending"), task("b", "pending")], { blockedOnLlmConfig: true })
    expect(lines[0]).toMatchObject({ kind: "problem", priority: "high", topic: "llm-config" })
    expect(lines[0].title).toBe("Falta configurar el modelo que escribe el wiki")
    expect(lines[0].actions[0]).toMatchObject({ kind: "go", label: "Ir a la configuración", target: { view: "settings" } })
    expect(lines[1].actions[0].label).toBe("Reanudar")
  })

  it("raises what failed, each one with its error in words and its own retry", () => {
    const lines = notices([task("a", "failed", "HTTP 429 Too Many Requests"), task("b", "failed", "fetch failed")])
    const failed = lines.find((line) => line.topic === "ingest-failed")!
    expect(failed).toMatchObject({ kind: "problem", count: 2 })
    expect(failed.title).toBe("2 documentos no se pudieron incorporar al wiki")
    expect(failed.cases?.[0].detail).toContain("límite de uso")
    expect(failed.cases?.[1].detail).toContain("No se pudo conectar")
    expect(failed.cases?.[0].actions?.map((a) => a.label)).toEqual(["Reintentar", "Quitar"])
    const remove = failed.cases?.[0].actions?.[1]
    expect(remove?.kind === "run" && remove.confirm).toContain("no se incorpora al wiki")
  })

  it("leaves cancelled tasks as a low line that does not count", () => {
    const [line] = notices([task("a", "cancelled")])
    expect(line).toMatchObject({ kind: "ongoing", priority: "low", topic: "ingest-cancelled" })
  })

  it("shows what else the wiki does with the model while it runs", () => {
    const activity: ActivityItem[] = [
      { id: "x", type: "lint", title: "Semantic wiki lint", status: "running", detail: "", filesWritten: [], createdAt: 0 },
      { id: "y", type: "query", title: "Review cleanup", status: "done", detail: "", filesWritten: [], createdAt: 0 },
    ]
    expect(notices([], {}, activity).map((line) => line.title)).toEqual(["Revisando el wiki con el modelo"])
  })

  it("puts the errors it knows in words, and keeps the rest as they came", () => {
    expect(ingestErrorInWords("LLM not configured — set an ingest model in Settings, then resume")).toBe(
      "Falta configurar el modelo que escribe el wiki.",
    )
    expect(ingestErrorInWords("Request timed out")).toBe("El modelo tardó demasiado en responder.")
    expect(ingestErrorInWords("weird")).toBe("El modelo o la lectura del documento fallaron: weird")
    expect(ingestErrorInWords(null)).toBe("No dio un motivo.")
  })
})

describe("changes of the wiki's files in the attention view", () => {
  function change(id: string, status: FileChangeTask["status"]): FileChangeTask {
    return { id, projectId: "p", path: `raw/sources/${id}.pdf`, kind: "modified", status, createdAt: 0, updatedAt: 0, retryCount: 0, error: "locked", needsRerun: false }
  }

  it("raises failed changes, and says that ignoring one loses it", () => {
    const [line] = fileSyncNotices([change("a", "failed"), change("b", "done")], null, undefined)
    expect(line).toMatchObject({ kind: "problem", count: 1, title: "1 cambio de archivo no se pudo registrar" })
    const ignore = line.cases?.[0].actions?.find((a) => a.label === "Ignorar")
    expect(ignore?.kind === "run" && ignore.confirm).toContain("se pierde")
    expect(line.cases?.[0].detail).toBe("cambió · locked")
  })

  it("shows waiting changes as in progress, and nothing once they are done", () => {
    expect(fileSyncNotices([change("a", "pending")], null, undefined)[0]).toMatchObject({ kind: "ongoing" })
    expect(fileSyncNotices([change("a", "done")], null, undefined)).toEqual([])
  })
})

describe("the line left at the foot of the wiki's side panel", () => {
  it("says the first problem, otherwise what is in progress, otherwise nothing", () => {
    const running = notices([task("a", "processing")])
    expect(activityLine(running)).toEqual({ text: "Incorporando al wiki: 0 de 1 documento", problem: false })
    const failing = notices([task("a", "processing"), task("b", "failed", "fetch failed")])
    expect(activityLine(failing)).toEqual({ text: "1 documento no se pudo incorporar al wiki", problem: true })
    expect(activityLine(notices([task("a", "cancelled")]))).toBeNull()
  })
})
