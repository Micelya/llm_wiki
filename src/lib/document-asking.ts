/**
 * Which engine answers the base's questions about documents (their type,
 * their stage, their data, the reading).
 *
 * The questions happen in Rust, behind an engine-neutral contract, like
 * text recognition (`text-recognition.ts`). The engine is the one of
 * «Preguntar al modelo» in the registry of AI services (Ajustes →
 * Servicios de IA, D-443), when the base can run it: today only Codex.
 */
import type { AskConfig } from "@/commands/docbase"
import { askConfigOf } from "@/lib/ai-services"
import { useAiServicesStore } from "@/stores/ai-services-store"

export function getAskConfig(): AskConfig | null {
  return askConfigOf(useAiServicesStore.getState().services)
}

/** The same, for a screen that shows it: it changes with the registry. */
export function useAskConfig(): AskConfig | null {
  const services = useAiServicesStore((s) => s.services)
  return askConfigOf(services)
}

/** What a screen says when no engine can answer. */
export const MISSING_ASK_ENGINE =
  "Falta elegir el motor de «Preguntar al modelo» en Ajustes → Servicios de IA (hoy la base pregunta con Codex)."
