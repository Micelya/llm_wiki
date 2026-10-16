/**
 * The buttons the model proposes come with fixed English labels (the
 * ingest prompt asks for them); they are shown in Spanish. The action
 * they carry does not change.
 */
const OPTION_LABELS: Record<string, string> = {
  "create page": "Crear página",
  skip: "Omitir",
  approve: "Aprobar",
  dismiss: "Descartar",
  ignore: "Ignorar",
  open: "Abrir",
  "open page": "Abrir página",
  "keep existing": "Dejar la existente",
  merge: "Unir",
  research: "Investigar",
}

export function optionLabel(label: string): string {
  return OPTION_LABELS[label.trim().toLowerCase()] ?? label
}

/** What was done with a proposal, as it was recorded, in Spanish. */
const RESOLVED_LABELS: Record<string, string> = {
  "saved to wiki": "Guardada en el wiki",
  deleted: "Borrada",
  "bulk resolved": "Marcada como resuelta",
  skip: "Omitida",
  dismissed: "Descartada",
}

export function resolvedLabel(action: string | undefined): string {
  if (!action) return "Resuelta"
  const known = RESOLVED_LABELS[action.trim().toLowerCase()]
  if (known) return known
  if (action.startsWith("save:")) return "Guardada en el wiki"
  if (action.startsWith("open:")) return "Abierta"
  if (action.startsWith("delete:")) return "Borrada"
  return optionLabel(action)
}
