import { create } from "zustand"
import type { WebSearchResult } from "@/lib/web-search"

export interface ResearchTask {
  id: string
  topic: string
  /** Review item that requested this research. It is resolved only after the
   * generated page has been written successfully. */
  rerunOfTaskId?: string
  searchQueries?: string[]
  /** The person, organization or matter it is of (D-414); none for a loose one. */
  objectId?: string
  objectName?: string
  status: "queued" | "searching" | "synthesizing" | "saving" | "done" | "error"
  webResults: WebSearchResult[]
  synthesis: string
  /** The number of the research kept in the base, once saved. */
  savedId: number | null
  error: string | null
  createdAt: number
}

export type ResearchTaskInput = Pick<
  ResearchTask,
  "topic" | "searchQueries" | "rerunOfTaskId" | "objectId" | "objectName"
>

interface ResearchState {
  tasks: ResearchTask[]
  panelOpen: boolean
  maxConcurrent: number

  addTask: (topic: string) => string
  addTasks: (inputs: Array<ResearchTaskInput>, options?: { openPanel?: boolean }) => string[]
  updateTask: (id: string, updates: Partial<ResearchTask>) => void
  removeTask: (id: string) => void
  setPanelOpen: (open: boolean) => void
  getRunningCount: () => number
  getNextQueued: () => ResearchTask | undefined
}

let counter = 0

const ACTIVE_RESEARCH_STATUSES = new Set<ResearchTask["status"]>([
  "queued",
  "searching",
  "synthesizing",
  "saving",
])

function researchLineageRoot(tasksById: ReadonlyMap<string, ResearchTask>, taskId: string): string {
  let current = taskId
  const visited = new Set<string>()
  while (!visited.has(current)) {
    visited.add(current)
    const parent = tasksById.get(current)?.rerunOfTaskId
    if (!parent) break
    current = parent
  }
  return current
}

export function hasActiveResearchRerun(tasks: readonly ResearchTask[], taskId: string): boolean {
  const tasksById = new Map(tasks.map((task) => [task.id, task]))
  const root = researchLineageRoot(tasksById, taskId)
  return tasks.some((task) =>
    task.id !== taskId &&
    ACTIVE_RESEARCH_STATUSES.has(task.status) &&
    researchLineageRoot(tasksById, task.id) === root,
  )
}

function createResearchTask(
  input: ResearchTaskInput,
): ResearchTask {
  return {
    id: `research-${++counter}`,
    topic: input.topic,
    ...(input.searchQueries?.length ? { searchQueries: input.searchQueries } : {}),
    ...(input.rerunOfTaskId ? { rerunOfTaskId: input.rerunOfTaskId } : {}),
    ...(input.objectId ? { objectId: input.objectId, objectName: input.objectName } : {}),
    status: "queued",
    webResults: [],
    synthesis: "",
    savedId: null,
    error: null,
    createdAt: Date.now(),
  }
}

export const useResearchStore = create<ResearchState>((set, get) => ({
  tasks: [],
  panelOpen: false,
  maxConcurrent: 3,

  addTask: (topic) => {
    const task = createResearchTask({ topic })
    set((state) => ({
      tasks: [...state.tasks, task],
      panelOpen: true,
    }))
    return task.id
  },

  addTasks: (inputs, options) => {
    const tasks = inputs.map(createResearchTask)
    if (tasks.length === 0) return []
    const openPanel = options?.openPanel ?? true
    set((state) => ({ tasks: [...state.tasks, ...tasks], panelOpen: openPanel || state.panelOpen }))
    return tasks.map((task) => task.id)
  },

  updateTask: (id, updates) =>
    set((state) => ({
      tasks: state.tasks.map((t) => (t.id === id ? { ...t, ...updates } : t)),
    })),

  removeTask: (id) =>
    set((state) => ({
      tasks: state.tasks.filter((t) => t.id !== id),
    })),

  setPanelOpen: (panelOpen) => set({ panelOpen }),

  getRunningCount: () => {
    const { tasks } = get()
    return tasks.filter((t) =>
      t.status === "searching" || t.status === "synthesizing" || t.status === "saving"
    ).length
  },

  getNextQueued: () => {
    const { tasks } = get()
    return tasks.find((t) => t.status === "queued")
  },
}))
