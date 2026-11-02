import { listFailures, type FailureView } from "@/commands/docbase"
import type { AttentionCase, AttentionNotice, NoticePriority } from "@/lib/attention/types"
import { plural } from "@/components/docbase/format"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"

/**
 * The failures of the background work still open (D-714), from the record
 * of failures of the base (`docbase::failures`). One line per cause, with
 * each failure as a case, in plain words: the cause, what the app did
 * («se va a reintentar el día tal» or «no se reintenta»), what the person
 * can do and whom to turn to: the support of Micelya. Sending a failure to
 * the support comes later (stage 2c): its button is drawn disabled.
 */
export const FAILURES_SOURCE = "failures"

/** Failures listed when a line is opened; the rest are counted. */
const LISTED = 30

export const SUPPORT = "el soporte de Micelya"

const SEND_LATER = "Todavía no se puede mandar una falla al soporte desde acá: llega en una próxima versión."

/** Each cause in words: what happened (one and many) and what a person can do. */
const CAUSES: Record<string, { one: string; many: string; todo: string }> = {
  "db-busy": { one: "vez la base estaba ocupada", many: "veces la base estaba ocupada", todo: "No hace falta hacer nada: pasa cuando otro programa la usa al mismo tiempo." },
  "disk-full": { one: "vez el disco estaba lleno", many: "veces el disco estaba lleno", todo: "Liberá espacio en el disco del equipo." },
  "db-damaged": { one: "vez la base estaba dañada", many: "veces la base estaba dañada", todo: "Volvé a una copia de respaldo de la base." },
  "pdf-encrypted": { one: "PDF tiene contraseña", many: "PDF tienen contraseña", todo: "Guardá en la carpeta una copia sin contraseña." },
  "pdf-broken": { one: "PDF está dañado o no es un PDF válido", many: "PDF están dañados o no son PDF válidos", todo: "Abrilo en otro programa: si abre, guardalo de nuevo como PDF." },
  "file-locked": { one: "archivo está abierto en otro programa", many: "archivos están abiertos en otro programa", todo: "Cerralo en ese programa." },
  "cloud-not-downloaded": { one: "archivo está en la nube y no se bajó al equipo", many: "archivos están en la nube y no se bajaron al equipo", todo: "Marcalo para tenerlo siempre en el equipo, o abrí la carpeta de la nube para que se baje." },
  "file-unavailable": { one: "archivo no se pudo abrir", many: "archivos no se pudieron abrir", todo: "Fijate que siga en su carpeta y que la carpeta esté conectada." },
  "zones-shape": { one: "lectura por zonas vino con otra forma", many: "lecturas por zonas vinieron con otra forma", todo: "Es un error del programa." },
  "out-of-memory": { one: "vez faltó memoria", many: "veces faltó memoria", todo: "Cerrá otros programas que estén abiertos." },
  "zones-model-missing": { one: "vez faltó el modelo de la lectura por zonas", many: "veces faltó el modelo de la lectura por zonas", todo: "Es un problema de la instalación." },
  "models-downloading": { one: "escaneo espera los modelos del lector de esta computadora, que se están bajando", many: "escaneos esperan los modelos del lector de esta computadora, que se están bajando", todo: "Esperá: se lee solo cuando terminen de bajarse." },
  "models-missing": { one: "escaneo espera porque faltan los modelos del lector de esta computadora", many: "escaneos esperan porque faltan los modelos del lector de esta computadora", todo: "Es un problema de la instalación: volvé a instalar la app." },
  "rate-limit": { one: "motor de IA llegó a su límite de uso", many: "motores de IA llegaron a su límite de uso", todo: "Esperá: el límite se libera solo. Si pasa seguido, revisá el plan del motor." },
  "no-connection": { one: "vez no hubo conexión con el motor de IA", many: "veces no hubo conexión con el motor de IA", todo: "Revisá la conexión a internet." },
  timeout: { one: "vez el motor de IA no contestó a tiempo", many: "veces el motor de IA no contestó a tiempo", todo: "No hace falta hacer nada." },
  "engine-missing": { one: "motor de IA no está instalado o no arranca", many: "motores de IA no están instalados o no arrancan", todo: "Revisá el motor elegido en Ajustes." },
  credential: { one: "motor de IA rechazó la credencial", many: "motores de IA rechazaron la credencial", todo: "Revisá la cuenta o la clave del motor en Ajustes." },
  "engine-no-images": { one: "motor de IA elegido no mira imágenes", many: "motores de IA elegidos no miran imágenes", todo: "Elegí en Ajustes un motor que mire imágenes: sin eso no se puede decidir dónde empieza cada documento." },
  "too-large": { one: "pregunta es demasiado grande para el servicio", many: "preguntas son demasiado grandes para el servicio", todo: "No se puede arreglar desde acá." },
  "bad-answer": { one: "respuesta del modelo vino con otra forma", many: "respuestas del modelo vinieron con otra forma", todo: "No hace falta hacer nada; si se repite, se deja de intentar y se avisa." },
  "folder-disconnected": { one: "archivo está en una carpeta que no está conectada", many: "archivos están en carpetas que no están conectadas", todo: "Conectá la carpeta o el disco donde está." },
  "before-record": { one: "falla quedó de antes, sin su causa", many: "fallas quedaron de antes, sin su causa", todo: "No hace falta hacer nada: se vuelve a intentar para conocer la causa." },
  "goes-back": { one: "documento volvió más de dos veces a un paso anterior", many: "documentos volvieron más de dos veces a un paso anterior", todo: "Miralo: puede ser una respuesta que cambia cada vez. Se deja aparte hasta que lo revises." },
  "program-error": { one: "error del programa", many: "errores del programa", todo: "No se puede arreglar desde acá." },
  unknown: { one: "falla desconocida", many: "fallas desconocidas", todo: "No se puede arreglar desde acá." },
}

/** Where it happened, in words. */
const STAGES: Record<string, string> = {
  shapes: "al mirar las páginas del PDF",
  cut: "al buscar dónde empieza cada documento",
  zones: "al leer las páginas por zonas",
  parts: "al separar los documentos del archivo",
  similar: "al preparar la búsqueda de parecidos",
  "digital-signatures": "al leer las firmas digitales",
  "cut-question": "al preguntarle a la IA dónde empieza cada documento",
  types: "al preguntar qué es cada documento",
  data: "al leer los datos de un documento",
  templates: "al nombrar los campos de las plantillas",
  reading: "al leer los documentos",
  resolve: "al reconocer a las personas y entidades",
  matters: "al proponer asuntos",
  pages: "al escribir las páginas",
  recognition: "al leer un escaneo",
  sweep: "al revisar una carpeta",
  thread: "en el trabajo de fondo",
  "process-back": "al volver a un paso anterior del proceso",
}

function when(at: number): string {
  return new Date(at).toLocaleString("es-AR", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" })
}

/** What the app did with the failure. */
export function whatTheAppDid(failure: FailureView): string {
  if (failure.class === "transient" && failure.nextAt !== null) {
    return `Se va a reintentar el ${when(failure.nextAt)}.`
  }
  return "No se reintenta."
}

function priorityOf(failure: FailureView): NoticePriority {
  if (failure.class !== "transient") return "high"
  return failure.indicator ? "normal" : "low"
}

const RANK: Record<NoticePriority, number> = { high: 0, normal: 1, low: 2 }

function caseOf(failure: FailureView): AttentionCase {
  const what = failure.path ?? (failure.itemKind === "engine" ? `El motor «${failure.item}»` : `${STAGES[failure.stage] ?? failure.stage}`)
  const stage = STAGES[failure.stage] ?? failure.stage
  const attempts = failure.attempt > 1 ? ` Van ${failure.attempt} intentos.` : ""
  return {
    id: `failure:${failure.id}`,
    title: what,
    detail: `Falló ${stage}. ${whatTheAppDid(failure)}${attempts}`,
    actions:
      failure.locationId === null
        ? []
        : [{ kind: "go", label: "Abrir", target: { view: "document", locationId: failure.locationId } }],
  }
}

export function failureNotices(failures: FailureView[] | null): AttentionNotice[] {
  if (!failures || failures.length === 0) return []
  const byCause = new Map<string, FailureView[]>()
  for (const failure of failures) {
    const list = byCause.get(failure.cause) ?? []
    list.push(failure)
    byCause.set(failure.cause, list)
  }
  const out: AttentionNotice[] = []
  for (const [cause, list] of byCause) {
    const words = CAUSES[cause] ?? CAUSES.unknown
    const priority = list.map(priorityOf).sort((a, b) => RANK[a] - RANK[b])[0]
    const retried = list.some((f) => f.class === "transient")
    out.push({
      source: FAILURES_SOURCE,
      id: `${FAILURES_SOURCE}:${cause}`,
      kind: "problem",
      topic: cause,
      priority,
      count: list.length,
      title: plural(list.length, words.one, words.many),
      explanation: retried
        ? "La app lo vuelve a intentar sola, cada vez más espaciado: a los 10 minutos, a la hora, a las 6 horas y después una vez por día. Mientras tanto, ese archivo espera: no se lee entero ni se saltea la IA."
        : "La app no lo vuelve a intentar sola: es un error para corregir. Mientras tanto, ese archivo espera: no se lee entero ni se saltea la IA.",
      todo: `${words.todo} Si sigue, pedí ayuda a ${SUPPORT}.`,
      resolvedWhen: retried ? "Desaparece cuando un nuevo intento sale bien." : "Desaparece cuando se corrige la causa y el paso sale bien.",
      actions: [{ kind: "run", label: "Mandar al soporte", run: () => undefined, disabled: SEND_LATER }],
      cases: list.slice(0, LISTED).map(caseOf),
      moreCases: Math.max(0, list.length - LISTED),
    })
  }
  return out
}

export function useFailureNotices(): AttentionNotice[] | null {
  const query = useDocbaseQuery("attention:failures", listFailures)
  if (query.error) throw new Error(query.error)
  if (query.data === null) return null
  return failureNotices(query.data)
}
