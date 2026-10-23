import { useEffect, useState } from "react"
import { listProgramRequests, type OperationCard } from "@/commands/operations"
import { DocbasePage, EmptyNote, ErrorNote } from "@/components/docbase/page"
import { plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { useWikiStore } from "@/stores/wiki-store"
import { OperationCardView } from "./operation-card"

/**
 * «Pedidos de otros programas» (D-495, the contract's design 6.4): the
 * changes an assistant or a script connected to Micelya asked for, each
 * with the same card as the chat's. Nothing was done: a person confirms,
 * unticks or cancels each one here. A card decided on this screen stays
 * on it, with «Deshacer», until the person leaves.
 */
export function ProgramRequestsView() {
  const requests = useDocbaseQuery("program-requests", listProgramRequests)
  const setActiveView = useWikiStore((s) => s.setActiveView)
  // The cards seen on this visit, in their order: one decided leaves the
  // list of the base but stays here with what was done.
  const [seen, setSeen] = useState<OperationCard[]>([])
  useEffect(() => {
    if (!requests.data) return
    setSeen((before) => {
      const known = new Set(before.map((card) => card.id))
      return [...before, ...requests.data!.filter((card) => !known.has(card.id))]
    })
  }, [requests.data])
  const waiting = requests.data?.length ?? 0

  return (
    <DocbasePage
      title="Pedidos de otros programas"
      subtitle={requests.data ? (waiting > 0 ? `${plural(waiting, "pedido espera", "pedidos esperan")} tu confirmación.` : "Ningún pedido espera.") : undefined}
    >
      <div className="mx-auto flex max-w-3xl flex-col gap-4">
        {requests.error && <ErrorNote>{requests.error}</ErrorNote>}
        <p className="text-sm text-muted-foreground">
          Lo que pidió cambiar un programa conectado a Micelya, por ejemplo un asistente. No se hizo nada: revisá lo que va a
          tocar cada pedido, destildá lo que no quieras y confirmalo o cancelalo. Un pedido que nadie decide vence a los 7 días.
          Qué programas pueden pedir cambios se elige en{" "}
          <button type="button" className="text-link hover:underline" onClick={() => setActiveView("settings")}>
            Ajustes → Programas conectados
          </button>
          .
        </p>
        {requests.data && seen.length === 0 && <EmptyNote>No hay pedidos de otros programas.</EmptyNote>}
        {seen.map((card) => (
          <OperationCardView key={card.id} card={card} />
        ))}
      </div>
    </DocbasePage>
  )
}
