import type { Progress, ProgressStage, StageId } from "@/commands/docbase"

/** Made-up figures of the stages of «En curso», for the tests. */

export const IDS: StageId[] = ["origins", "text", "recognition", "similar", "types", "reading", "resolve", "matters", "pages"]

export function stage(id: StageId, patch: Partial<ProgressStage> = {}): ProgressStage {
  const unit = id === "origins" ? "origins" : id === "text" ? "files" : id === "pages" ? "pages" : "documents"
  return {
    id,
    state: "done",
    unit,
    total: 10,
    done: 10,
    pending: 0,
    failed: 0,
    out: 0,
    otherWork: false,
    current: null,
    error: null,
    extra: {},
    ...patch,
  }
}

export function progress(states: Partial<Record<StageId, Partial<ProgressStage>>> = {}, patch: Partial<Progress> = {}): Progress {
  return {
    stages: IDS.map((id) => stage(id, states[id])),
    inProcess: 0,
    paused: 0,
    backgroundReading: true,
    engineChosen: true,
    recognitionEngine: "codex-cli:modelo-de-prueba",
    lastError: null,
    ...patch,
  }
}
