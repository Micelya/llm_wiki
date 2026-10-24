import { useMemo } from "react"
import { stopPortal, type PortalStatus } from "@/commands/portals"
import type { AttentionAction, AttentionNotice } from "@/lib/attention/types"
import { plural } from "@/components/docbase/format"
import { usePortalStatus } from "@/components/portals/use-portal-status"
import { usePortalStore } from "@/stores/portal-store"
import { vocabulary, type Vocabulary } from "@/vocabulary"

/**
 * What the judicial portals raise: the walk over a record that waits for
 * a person (the portal's challenge, signing in), the walk in course, and
 * how the last one ended while the person was in other screens, until she
 * opens «Portales» (D-461: the walk goes on with the portal's window
 * hidden).
 */
export const PORTALS_SOURCE = "portals"

/** The walk's figures in words: "página 3 · 12 documentos bajados · 4 ya estaban". */
export function walkFigures(status: PortalStatus): string {
  const p = status.progress
  return [
    p.page ? `página ${p.page}` : null,
    plural(p.seen, "actuación vista", "actuaciones vistas"),
    plural(p.downloaded, "documento bajado", "documentos bajados"),
    p.already > 0 ? `${p.already} ya estaban` : null,
    p.failed > 0 ? `${p.failed} no se pudieron bajar` : null,
  ]
    .filter(Boolean)
    .join(" · ")
}

export function portalNotices(status: PortalStatus | null, endUnseen: boolean, words: Vocabulary = vocabulary()): AttentionNotice[] {
  if (!status) return []
  const p = status.progress
  const terms = words.portals
  const named = `${terms.record.toLowerCase()} ${p.reference ?? status.running ?? ""}`.trim()
  const record = `el ${named}`
  const goPortals: AttentionAction = { kind: "go", label: `Ir a ${terms.name}`, target: { view: "portals" } }
  const notice = (patch: Omit<AttentionNotice, "source" | "id" | "count" | "actions"> & { actions?: AttentionAction[] }): AttentionNotice => ({
    source: PORTALS_SOURCE,
    id: `${PORTALS_SOURCE}:${patch.topic}`,
    count: 1,
    actions: [goPortals],
    ...patch,
  })
  if (status.running) {
    if (p.state === "needs-person") {
      const login = p.person === "login"
      return [
        notice({
          kind: "problem",
          topic: "needs-person",
          priority: "high",
          title: `El portal te necesita para seguir trayendo ${record}`,
          explanation: login
            ? "El portal pide iniciar sesión, y eso solo lo puede hacer una persona. Mientras tanto Micelya espera."
            : "El portal pide resolver su desafío, y eso solo lo puede hacer una persona. Mientras tanto Micelya espera.",
          todo: login
            ? `Andá a «${terms.name}» e iniciá sesión en la ventana del portal; Micelya sigue sola cuando entres.`
            : `Andá a «${terms.name}» y resolvé el desafío en el recuadro resaltado; después tocá «Consultar». Micelya sigue sola.`,
          resolvedWhen: "Desaparece cuando la página del portal cambia.",
        }),
      ]
    }
    return [
      notice({
        kind: "ongoing",
        topic: "walk",
        priority: "normal",
        title: `Trayendo ${record} del portal`,
        explanation: `${walkFigures(status)}. ${p.message}`,
        todo: "Nada: podés seguir trabajando en otras pantallas. Micelya avisa al terminar.",
        resolvedWhen: "Desaparece cuando termina.",
        actions: [goPortals, { kind: "run", label: "Detener", run: () => stopPortal() }],
        progress: p.withDocument > 0 ? { done: p.downloaded + p.already + p.failed, total: p.withDocument } : undefined,
      }),
    ]
  }
  if (!endUnseen) return []
  const warnings = p.warnings.length > 0 ? ` Además: ${p.warnings.join(" ")}` : ""
  switch (p.state) {
    case "error":
      return [
        notice({
          kind: "problem",
          topic: "walk-failed",
          priority: "normal",
          title: `No se pudo terminar de traer ${record}`,
          explanation: `${p.message}${warnings}`,
          todo: `Mirá en «${terms.name}» qué pasó y volvé a pedirlo: lo que se bajó queda guardado y sigue desde ahí.`,
          resolvedWhen: `Desaparece cuando abrís «${terms.name}».`,
        }),
      ]
    case "stopped":
      return [
        notice({
          kind: "ongoing",
          topic: "walk-ended",
          priority: "low",
          title: `Se detuvo el recorrido del ${named}`,
          explanation: p.message,
          todo: `Para seguir, pedilo de nuevo en «${terms.name}».`,
          resolvedWhen: `Desaparece cuando abrís «${terms.name}».`,
        }),
      ]
    case "done":
      return [
        notice({
          kind: "ongoing",
          topic: "walk-ended",
          priority: "low",
          title: `Terminó de traer ${record}`,
          explanation: `${p.message}${warnings}`,
          todo: `Sus documentos están en la base. En «${terms.name}» está el detalle.`,
          resolvedWhen: `Desaparece cuando abrís «${terms.name}».`,
        }),
      ]
    default:
      return []
  }
}

export function usePortalNotices(): AttentionNotice[] | null {
  const status = usePortalStatus()
  const endUnseen = usePortalStore((s) => s.endUnseen)
  return useMemo(() => portalNotices(status, endUnseen), [status, endUnseen])
}
