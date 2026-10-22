import type { DocumentState } from "@/commands/docbase"
import { StatusBadge, type Tone } from "./page"

interface StateInfo {
  label: string
  tone: Tone
  /** What it means and what happens next, in plain words. */
  hint: string
}

export const DOCUMENT_STATES: Record<DocumentState, StateInfo> = {
  processed: {
    label: "Procesado",
    tone: "ok",
    hint: "Su texto ya está en la base y se puede buscar.",
  },
  unprocessed: {
    label: "Identificado",
    tone: "busy",
    hint: "La base ya sabe qué documento es. Falta obtener su texto.",
  },
  unread: {
    label: "Pendiente de leer",
    tone: "busy",
    hint: "Está en el inventario. La base todavía no lo leyó; lo hace de a poco, en segundo plano.",
  },
  "needs-recognition": {
    label: "Texto en imagen",
    tone: "warning",
    hint: "Es un escaneo o una foto: su texto se reconoce con el modelo, solo mientras la lectura automática está activa o cuando lo pedís con «Reconocer el texto».",
  },
  excluded: {
    label: "Excluido",
    tone: "neutral",
    hint: "Lo dejaste afuera: sigue en su lugar, pero la base no lo procesa.",
  },
  link: {
    label: "Detectado, no procesable",
    tone: "neutral",
    hint: "Es un documento propio de un disco en la nube. En la carpeta solo hay un acceso directo, sin contenido.",
  },
  unsupported: {
    label: "Sin texto",
    tone: "neutral",
    hint: "Es un formato del que no se obtiene texto (por ejemplo audio o video).",
  },
  "too-large": {
    label: "Demasiado grande",
    tone: "warning",
    hint: "Supera el tamaño que la base lee. Queda en el inventario, sin comparar con otros.",
  },
  unreadable: {
    label: "No se pudo leer",
    tone: "error",
    hint: "El archivo no se pudo abrir. Se vuelve a intentar en cada barrido.",
  },
  error: {
    label: "Error al procesar",
    tone: "error",
    hint: "Se pudo leer, pero falló la obtención de su texto.",
  },
}

/** States in the order the filter offers them. */
export const DOCUMENT_STATE_ORDER: DocumentState[] = [
  "processed",
  "unprocessed",
  "unread",
  "needs-recognition",
  "link",
  "excluded",
  "unsupported",
  "too-large",
  "unreadable",
  "error",
]

export function DocumentStateBadge({ state }: { state: DocumentState }) {
  const info = DOCUMENT_STATES[state] ?? DOCUMENT_STATES.unread
  return (
    <StatusBadge tone={info.tone} title={info.hint}>
      {info.label}
    </StatusBadge>
  )
}
