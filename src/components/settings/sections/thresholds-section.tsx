import { useEffect, useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  getConfidenceEffect,
  getThresholds,
  setThresholds,
  type ConfidenceEffect,
  type Thresholds,
} from "@/commands/docbase"
import { effectText } from "@/components/docbase/reading-quality"
import { useWikiStore } from "@/stores/wiki-store"

/**
 * Ajustes → Umbrales (E1 of 2 nov, P-542): every threshold that is a
 * setting of the base (`thresholds.rs`), with what it decides and an
 * example, its value, whether it is provisional, and what changing it
 * does before it is kept: whether something is asked again, and how much.
 */

type Key = keyof Thresholds

export interface ThresholdInfo {
  key: Key
  group: string
  name: string
  /** What it decides, in words, with an example. */
  decides: string
  provisional: boolean
  /** How it is written: a share as a percentage, or a whole number. */
  unit: "share" | "count" | "hours" | "minutes" | "letters"
  /** What changing it does; the two of the confidence say it with the documents (`effectText`). */
  change: string
}

/** The values by default, the same as `Thresholds::default()` in the base. */
export const BY_DEFAULT: Thresholds = {
  low: 0.9,
  high: 0.97,
  quality: 0.5,
  lowConfidenceType: 0.3,
  indicatorAttempts: 3,
  indicatorHours: 6,
  roundCap: 50,
  retryMinutes: [10, 60, 360, 1440],
  nameLetters: 2,
  nameShare: 0.15,
  groupSimilarity: 0.85,
  closeSimilarity: 0.85,
}

const CONFIDENCE = "Se calcula con las proporciones guardadas, sin leer ni preguntar nada."

export const THRESHOLDS: ThresholdInfo[] = [
  {
    key: "high",
    group: "La lectura",
    name: "Renglón firme",
    decides:
      "Desde qué confianza del lector un renglón es firme: el único texto que da datos (nombres, DNI, fechas, importes). Por ejemplo, con 97 %, un renglón leído con 98 % da datos y uno con 95 % queda aproximado.",
    provisional: false,
    unit: "share",
    change:
      "Se aplica a lo que se lea desde ahora. Lo ya leído no se vuelve a leer ni se recalcula (está pendiente recalcular las franjas sin volver a leer, P-551): 0 consultas.",
  },
  {
    key: "low",
    group: "La lectura",
    name: "Renglón aproximado",
    decides:
      "Debajo de esta confianza el renglón se descarta; entre este y el firme queda aproximado: sirve para entender y para buscar, pero no da datos. Por ejemplo, con 90 %, un renglón leído con 85 % se descarta.",
    provisional: false,
    unit: "share",
    change:
      "Se aplica a lo que se lea desde ahora. Lo ya leído no se vuelve a leer ni se recalcula (P-551): 0 consultas.",
  },
  {
    key: "quality",
    group: "La calidad de la lectura",
    name: "No legible",
    decides:
      "Debajo de esta proporción de palabras seguras un documento no se puede leer: sirve solo para consultas y no da datos. Por ejemplo, con 50 %, una partida escaneada con 40 % de palabras seguras queda no legible.",
    provisional: true,
    unit: "share",
    change: CONFIDENCE,
  },
  {
    key: "lowConfidenceType",
    group: "La calidad de la lectura",
    name: "Tipo de confianza baja",
    decides:
      "Debajo de esta proporción de palabras seguras el tipo del documento se pregunta igual, con las palabras aproximadas, pero queda de confianza baja: lo que sale de él no se usa para nada derivado. Por ejemplo, con 30 %, un documento con 25 % queda de confianza baja. Manda el mayor de los dos umbrales de la calidad.",
    provisional: true,
    unit: "share",
    change: CONFIDENCE,
  },
  {
    key: "nameLetters",
    group: "La identidad",
    name: "Letras de diferencia en el nombre",
    decides:
      "Dos menciones con el mismo DNI, CUIT o matrícula, o uno a una cifra, se unen como la misma persona solo si sus nombres difieren en estas letras o menos. Por ejemplo, con 2, «Ana Ejemplo» y «Ana Ejempla» se unen.",
    provisional: true,
    unit: "letters",
    change:
      "La identidad se vuelve a calcular en el equipo, sin preguntar. Con más letras se unen más menciones; con menos, menos. Las páginas de las personas que se unan o se separen se vuelven a escribir: una consulta cada una a «Preguntar al modelo».",
  },
  {
    key: "nameShare",
    group: "La identidad",
    name: "Parte del nombre que puede cambiar",
    decides:
      "Las letras de diferencia no pueden pasar de esta parte del nombre, para que un nombre corto no admita dos cambios. Por ejemplo, con 15 %, un nombre de 10 letras admite 1 cambio.",
    provisional: true,
    unit: "share",
    change:
      "La identidad se vuelve a calcular en el equipo, sin preguntar. Las páginas de las personas que se unan o se separen se vuelven a escribir: una consulta cada una a «Preguntar al modelo».",
  },
  {
    key: "groupSimilarity",
    group: "El tipo por parecido",
    name: "Grupos por parecido",
    decides:
      "Los documentos sin familia cuyo texto se parece desde este valor (por sus vectores) van juntos en una sola pregunta de tipo, a lo sumo 10, con una respuesta para cada uno: cada uno recibe su propio tipo. Por ejemplo, con 85, dos partidas que se parecen 90 se preguntan juntas.",
    provisional: true,
    unit: "share",
    change:
      "Cambia cómo se juntan las preguntas de tipo que quedan; lo ya clasificado no se vuelve a preguntar. Más bajo junta más y ahorra preguntas, con más riesgo de juntar tipos distintos: 0 consultas nuevas.",
  },
  {
    key: "closeSimilarity",
    group: "El tipo por parecido",
    name: "Vecinos muy cercanos",
    decides:
      "Si los 3 documentos ya clasificados más parecidos a uno se parecen desde este valor, coinciden en un tipo del catálogo o confirmado y el modelo eligió otro, el tipo queda marcado como dudoso y va a Atención; no se pregunta de nuevo. Por ejemplo, con 85, tres actas de nacimiento a 90 y una respuesta «recibo» se marcan.",
    provisional: true,
    unit: "share",
    change: "Se aplica a las respuestas que lleguen desde ahora: 0 consultas.",
  },
  {
    key: "indicatorAttempts",
    group: "Las fallas",
    name: "Intentos antes del aviso",
    decides:
      "Cuántas veces puede fallar lo mismo antes de que Atención lo marque como una falla que se repite. Por ejemplo, con 3, al tercer intento fallido aparece el aviso.",
    provisional: false,
    unit: "count",
    change: "Cambia solo cuándo aparece el aviso: 0 consultas.",
  },
  {
    key: "indicatorHours",
    group: "Las fallas",
    name: "Horas antes del aviso",
    decides:
      "Cuánto tiempo puede seguir fallando lo mismo antes del aviso, aunque no llegue a los intentos. Por ejemplo, con 6, una falla que sigue después de 6 horas se avisa.",
    provisional: false,
    unit: "hours",
    change: "Cambia solo cuándo aparece el aviso: 0 consultas.",
  },
  {
    key: "retryMinutes",
    group: "Las fallas",
    name: "Esperas de los reintentos",
    decides:
      "Cuántos minutos se espera para volver a probar una falla pasajera (un corte de la red, un servicio ocupado), uno por intento; pasada la última, se repite la última. Las fallas permanentes y las desconocidas no se reintentan solas.",
    provisional: false,
    unit: "minutes",
    change: "Las consultas son las mismas, más pronto o más tarde: 0 consultas nuevas.",
  },
  {
    key: "roundCap",
    group: "Las preguntas solas",
    name: "Tope de la ronda",
    decides:
      "Cuántas preguntas solas (el corte dudoso, quién es quién, los resúmenes de las páginas) se hacen seguidas antes de que la app pida permiso para seguir. Por ejemplo, con 50, la pregunta 51 espera a que digas «Seguir».",
    provisional: false,
    unit: "count",
    change: "No suma consultas: cambia cuántas se hacen seguidas antes de pedirte permiso.",
  },
]

/** A value as the screen writes it. */
export function shown(info: ThresholdInfo, value: number | number[]): string {
  if (Array.isArray(value)) return value.join(", ")
  return info.unit === "share" ? String(Math.round(value * 1000) / 10).replace(".", ",") : String(value)
}

/** A value from what a person typed; null when it is not one. */
export function parsed(info: ThresholdInfo, typed: string): number | number[] | null {
  if (info.unit === "minutes") {
    const waits = typed.split(",").map((w) => Number(w.trim()))
    return waits.length > 0 && waits.every((w) => Number.isInteger(w) && w > 0) ? waits : null
  }
  const n = Number(typed.trim().replace(",", "."))
  if (!Number.isFinite(n) || typed.trim() === "") return null
  if (info.unit === "share") return n / 100
  return Number.isInteger(n) ? n : null
}

/** Why these thresholds cannot be kept, as the base says it; null when they can. */
export function invalid(t: Thresholds): string | null {
  if (!(t.low > 0 && t.low < t.high && t.high <= 1)) {
    return "El umbral bajo tiene que ser mayor que 0 y menor que el alto, y el alto, a lo sumo 1."
  }
  for (const share of [t.quality, t.lowConfidenceType, t.nameShare]) {
    if (!(share >= 0 && share <= 1)) return "La calidad es una proporción: entre 0 y 1."
  }
  for (const likeness of [t.groupSimilarity, t.closeSimilarity]) {
    if (!(likeness >= 0 && likeness <= 1)) return "El parecido va de 0 a 1."
  }
  if (t.indicatorAttempts < 1 || t.indicatorHours < 1 || t.roundCap < 1) {
    return "Los intentos, las horas y el tope de la ronda tienen que ser 1 o más."
  }
  if (t.retryMinutes.length === 0 || t.retryMinutes.some((m) => m < 1)) {
    return "Las esperas de los reintentos son minutos, 1 o más, separados por comas."
  }
  return null
}

function same(a: number | number[], b: number | number[]): boolean {
  return JSON.stringify(a) === JSON.stringify(b)
}

/** The thresholds a person changed and not kept yet. */
export function changed(saved: Thresholds, draft: Thresholds): Key[] {
  return THRESHOLDS.map((t) => t.key).filter((key) => !same(saved[key], draft[key]))
}

/** What changing a threshold does, as the screen says it before keeping it. */
export function changeText(info: ThresholdInfo, effect: ConfidenceEffect | null): string {
  if ((info.key === "quality" || info.key === "lowConfidenceType") && effect) return `${info.change} ${effectText(effect)}`
  return info.change
}

interface ViewProps {
  saved: Thresholds
  draft: Thresholds
  /** What the two of the confidence would change, when one of them changed. */
  effect: ConfidenceEffect | null
  problem: string | null
  saving: boolean
  onChange: (draft: Thresholds) => void
  onSave: () => void
  onDiscard: () => void
  onQuality: () => void
}

/** What «Umbrales» shows. */
export function ThresholdsView({ saved, draft, effect, problem, saving, onChange, onSave, onDiscard, onQuality }: ViewProps) {
  const pending = changed(saved, draft)
  const why = invalid(draft)
  const groups = [...new Set(THRESHOLDS.map((t) => t.group))]
  return (
    <div className="space-y-6">
      <div>
        <h2 className="font-ui text-[1.75rem] leading-tight font-bold tracking-[-0.01em] text-strong">Umbrales</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Los valores con los que la base decide qué se lee como firme, qué documento no se puede leer, cuándo se unen dos
          personas y cuándo se avisa una falla. Son de esta base. Los provisorios se eligen midiendo: la lista de los
          documentos y cuántos quedan debajo de cada valor están en «Calidad de la lectura».
        </p>
        <Button className="mt-2" size="sm" variant="outline" onClick={onQuality}>
          Ver la calidad de la lectura
        </Button>
      </div>
      {groups.map((group) => (
        <section key={group} className="space-y-3" data-thresholds-group={group}>
          <h3 className="font-ui text-[0.9375rem] font-bold text-strong">{group}</h3>
          {THRESHOLDS.filter((t) => t.group === group).map((info) => (
            <ThresholdRow
              key={info.key}
              info={info}
              saved={saved[info.key]}
              value={draft[info.key]}
              effect={effect}
              onValue={(value) => onChange({ ...draft, [info.key]: value })}
            />
          ))}
        </section>
      ))}
      <div className="flex flex-wrap items-center gap-3 border-t pt-4" data-thresholds="save">
        <Button disabled={pending.length === 0 || why !== null || saving} onClick={onSave}>
          Guardar los umbrales
        </Button>
        <Button variant="ghost" disabled={pending.length === 0 || saving} onClick={onDiscard}>
          Descartar los cambios
        </Button>
        {why && <span className="text-sm text-destructive">{why}</span>}
        {problem && <span className="text-sm text-destructive">{problem}</span>}
        {!why && !problem && pending.length > 0 && (
          <span className="text-sm text-muted-foreground">
            {pending.length === 1 ? "1 umbral cambiado" : `${pending.length} umbrales cambiados`}, sin guardar.
          </span>
        )}
      </div>
    </div>
  )
}

function ThresholdRow({
  info,
  saved,
  value,
  effect,
  onValue,
}: {
  info: ThresholdInfo
  saved: number | number[]
  value: number | number[]
  effect: ConfidenceEffect | null
  onValue: (value: number | number[]) => void
}) {
  const [typed, setTyped] = useState(shown(info, value))
  useEffect(() => setTyped(shown(info, value)), [info, value])
  const byDefault = BY_DEFAULT[info.key]
  const unit = { share: "%", count: "", hours: "horas", minutes: "minutos", letters: "letras" }[info.unit]
  return (
    <div className="space-y-2 rounded-md border p-4 text-sm" data-threshold={info.key}>
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium">{info.name}</span>
        {info.provisional && (
          <span className="rounded border border-warn-line bg-warn-bg px-1.5 py-0.5 text-xs font-medium">Provisorio</span>
        )}
      </div>
      <p className="text-muted-foreground">{info.decides}</p>
      <div className="flex flex-wrap items-center gap-2">
        <Input
          className="w-40"
          value={typed}
          aria-label={info.name}
          onChange={(event) => {
            setTyped(event.target.value)
            const next = parsed(info, event.target.value)
            if (next !== null) onValue(next)
          }}
        />
        <span>{unit}</span>
        <span className="text-muted-foreground">Por omisión: {shown(info, byDefault)} {unit}</span>
        {!same(value, byDefault) && (
          <Button size="sm" variant="ghost" onClick={() => onValue(byDefault)}>
            Volver al valor por omisión
          </Button>
        )}
      </div>
      {!same(value, saved) && (
        <p className="rounded-md border border-warn-line bg-warn-bg px-3 py-2" data-threshold-change={info.key}>
          <strong>Si lo cambiás de {shown(info, saved)} a {shown(info, value)} {unit}:</strong> {changeText(info, effect)}
        </p>
      )}
    </div>
  )
}

/** The section of Ajustes: loads the thresholds, says what a change does, keeps them. */
export function ThresholdsSection() {
  const [saved, setSaved] = useState<Thresholds | null>(null)
  const [draft, setDraft] = useState<Thresholds | null>(null)
  const [effect, setEffect] = useState<ConfidenceEffect | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const setActiveView = useWikiStore((s) => s.setActiveView)

  useEffect(() => {
    getThresholds()
      .then((t) => {
        setSaved(t)
        setDraft(t)
      })
      .catch((cause) => setProblem(String(cause)))
  }, [])

  const quality = draft?.quality
  const lowType = draft?.lowConfidenceType
  useEffect(() => {
    if (quality === undefined || lowType === undefined || !saved) return
    if (quality === saved.quality && lowType === saved.lowConfidenceType) {
      setEffect(null)
      return
    }
    let gone = false
    getConfidenceEffect(quality, lowType)
      .then((found) => !gone && setEffect(found))
      .catch((cause) => !gone && setProblem(String(cause)))
    return () => {
      gone = true
    }
  }, [quality, lowType, saved])

  if (!saved || !draft) {
    return problem ? <p className="text-sm text-destructive">{problem}</p> : <p className="text-sm text-muted-foreground">Cargando…</p>
  }

  async function save() {
    if (!draft) return
    setSaving(true)
    setProblem(null)
    try {
      await setThresholds(draft)
      setSaved(draft)
      setEffect(null)
    } catch (cause) {
      setProblem(String(cause))
    } finally {
      setSaving(false)
    }
  }

  return (
    <ThresholdsView
      saved={saved}
      draft={draft}
      effect={effect}
      problem={problem}
      saving={saving}
      onChange={setDraft}
      onSave={() => void save()}
      onDiscard={() => {
        setDraft(saved)
        setProblem(null)
      }}
      onQuality={() => setActiveView("reading-quality")}
    />
  )
}

