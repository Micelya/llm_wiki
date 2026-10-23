import { useEffect, useMemo, useState, type ReactNode } from "react"
import { ArrowLeft, ExternalLink, FileText, FolderOpen } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  generateDocument,
  getEntity,
  getMatter,
  getSimilarTemplate,
  listEntities,
  listMatters,
  openGenerated,
  type EntityDetail,
  type EntitySummary,
  type GenerateRequest,
  type GeneratedDocument,
  type GeneratedLine,
  type MatterParty,
  type MatterSummary,
  type Template,
} from "@/commands/docbase"
import { useAction, useOpenDocument } from "@/components/knowledge/shared"
import { cn } from "@/lib/utils"
import { entityTypeName, vocabulary, type Vocabulary } from "@/vocabulary"
import { formatDate, plural } from "./format"
import { DocbasePage, EmptyNote, ErrorNote } from "./page"
import { templateName } from "./template"
import {
  applySuggestions,
  dateStyleOf,
  entityOptions,
  formatDateIn,
  generationRequest,
  generationRows,
  initialValues,
  isoFromDate,
  matterValues,
  missingNames,
  preview,
  relatedRows,
  suggestedFileName,
  suggestionsFrom,
  todayIso,
  valuesFromDocument,
  withMatter,
  type EntityOption,
  type FieldValue,
  type FieldValues,
  type GenerationRow,
  type PreviewPiece,
} from "./template-generate"
import { useDocbaseQuery } from "./use-docbase-query"

const FIELD_CLASS =
  "w-full min-w-0 rounded-lg border border-input bg-transparent px-2.5 py-1 text-sm outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30"
const SELECT_CLASS =
  "h-8 max-w-full rounded-lg border border-input bg-transparent px-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 dark:bg-input/30"
/** The choice of a wording that is none of the documents'. */
const OTHER = "\u0000otra"
/** Letters typed before the people and organizations of the base are looked for. */
const SEARCH_FROM = 2

/** What the form asks the base about its people, organizations and matters (D-488). */
export interface BaseLookups {
  /** The people or organizations of a type (`null`: any) whose names have these words. */
  searchEntities?: (kind: string | null, query: string) => Promise<EntitySummary[]>
  /** What the base knows of one: its identifiers, its address. */
  entityCard?: (id: string) => Promise<EntityDetail | null>
  /** The people and organizations of a matter, offered first. */
  matterParties?: (id: string) => Promise<MatterParty[]>
}

/** The lookups of the base, the same functions on every render. */
const LOOKUPS: Required<BaseLookups> = {
  searchEntities: (kind, query) => listEntities(kind, query),
  entityCard: (id) => getEntity(id),
  matterParties: async (id) => (await getMatter(id))?.parties ?? [],
}

/**
 * Generating a document from a template (D-488), in the section
 * «Plantillas»: the fields to fill, the preview, and a simple Word in
 * «Generados» of the work folder. From a proposal too, with a notice.
 */
export function TemplateGenerateView({ principalLocationId, onBack }: { principalLocationId: number; onBack: () => void }) {
  const words = vocabulary()
  const template = useDocbaseQuery(`similar-template:${principalLocationId}`, () => getSimilarTemplate(principalLocationId))
  const matters = useDocbaseQuery("matters", listMatters)
  const action = useAction()
  const openDocument = useOpenDocument()
  const [result, setResult] = useState<GeneratedDocument | null>(null)
  const [round, setRound] = useState(0)
  return (
    <DocbasePage
      title={words.templates.name}
      actions={
        <Button size="sm" variant="ghost" onClick={onBack}>
          <ArrowLeft /> Volver a la {words.templates.one}
        </Button>
      }
    >
      <div className="mx-auto flex max-w-6xl flex-col gap-4">
        {(template.error || action.problem) && <ErrorNote>{template.error ?? action.problem}</ErrorNote>}
        {!template.loading && !template.data && !template.error && (
          <EmptyNote>Esta {words.templates.one} ya no está: su familia de documentos cambió.</EmptyNote>
        )}
        {template.data && (
          <GenerateForm
            key={`${template.data.principal}:${round}`}
            template={template.data}
            matters={matters.data ?? []}
            busy={action.busy}
            result={result}
            onGenerate={(request) =>
              void action.run(async () => {
                setResult(await generateDocument(request))
              })
            }
            onOpenResult={(reveal) => result && void action.run(() => openGenerated(result.id, reveal))}
            onShowDocument={(locationId) => openDocument(locationId)}
            {...LOOKUPS}
            onAgain={() => {
              setResult(null)
              setRound(round + 1)
            }}
            words={words}
          />
        )}
      </div>
    </DocbasePage>
  )
}

/** The form to generate a document, apart from the base so it can be tested. */
export function GenerateForm({
  template,
  matters,
  busy = false,
  result = null,
  onGenerate,
  onOpenResult,
  onShowDocument,
  onAgain,
  searchEntities,
  entityCard,
  matterParties,
  today = todayIso(),
  words = vocabulary(),
}: BaseLookups & {
  template: Template
  matters: MatterSummary[]
  busy?: boolean
  /** What was generated, once it was. */
  result?: GeneratedDocument | null
  onGenerate: (request: GenerateRequest) => void
  onOpenResult?: (reveal: boolean) => void
  onShowDocument?: (locationId: number) => void
  onAgain?: () => void
  /** Today, as the calendar writes it ("2026-10-22"). */
  today?: string
  words?: Vocabulary
}) {
  const rows = useMemo(() => generationRows(template, words), [template, words])
  const [values, setValues] = useState<FieldValues>(() => initialValues(rows, today))
  const [matterId, setMatterId] = useState<string | null>(null)
  const [parties, setParties] = useState<MatterParty[]>([])
  /** The names of the people and organizations chosen, by id: what a suggestion says it came from. */
  const [chosen, setChosen] = useState<Record<string, string>>({})
  const [fromDocument, setFromDocument] = useState<number | null>(null)
  const [fileName, setFileName] = useState<string | null>(null)
  const name = templateName(template, template.documents[0]?.doc.name ?? "", words)
  const shown = preview(template, rows, values)
  const missing = missingNames(rows, values)
  const suggested = suggestedFileName(name, rows, values, today)
  const matterWord = words.knowledge.matter.toLowerCase()
  const asked = rows.filter((row) => row.input !== "fixed")
  const open = matters.filter((matter) => matter.confirmation !== "discarded")
  const set = (row: GenerationRow, value: FieldValue) => setValues((current) => ({ ...current, [row.index]: value }))

  // The people and organizations of the matter chosen are offered first.
  useEffect(() => {
    if (!matterId || !matterParties) {
      setParties([])
      return
    }
    let stale = false
    matterParties(matterId)
      .then((found) => !stale && setParties(found))
      .catch(() => !stale && setParties([]))
    return () => {
      stale = true
    }
  }, [matterId, matterParties])

  function chooseMatter(id: string | null) {
    setMatterId(id)
    setValues((current) => withMatter(current, rows, open.find((matter) => matter.id === id) ?? null))
  }

  /** Someone of the base for a field: its name there, and its DNI, CUIT and address in the fields that ask for them. */
  function chooseEntity(row: GenerationRow, option: EntityOption) {
    set(row, { value: option.name, source: "entity", reference: `entity:${option.id}` })
    setChosen((current) => ({ ...current, [option.id]: option.name }))
    const related = relatedRows(rows, row, words)
    if (related.length === 0 || !entityCard) return
    entityCard(option.id)
      .then((card) => card && setValues((current) => applySuggestions(current, suggestionsFrom(card, related))))
      .catch(() => {})
  }

  if (result) {
    return <GeneratedNote result={result} onOpen={onOpenResult} onShowDocument={onShowDocument} onAgain={onAgain} words={words} />
  }

  return (
    <>
      <div className="flex flex-col gap-1">
        <h2 className="font-serif text-2xl break-words">Generar un documento</h2>
        <p className="text-sm text-muted-foreground">
          Desde la {words.templates.one} «{name}»:{" "}
          {asked.length === 0
            ? "no hay campos para completar."
            : `${plural(asked.length, "campo para completar", "campos para completar")}.`}{" "}
          Sale un Word simple, sin membrete: el texto de la {words.templates.one} con los valores. No conserva el formato del
          original (negritas, sangrías, tablas).
        </p>
      </div>
      {template.format === "scan" && (
        <Notice>El texto salió de un escaneo: lo leyó el reconocimiento y puede tener errores. Revisalo en el Word.</Notice>
      )}

      <div className="flex flex-wrap items-end gap-4 rounded-md border bg-muted/20 px-4 py-3 text-sm">
        <label className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground">Para qué {matterWord} (opcional)</span>
          <select
            className={SELECT_CLASS}
            value={matterId ?? ""}
            onChange={(event) => chooseMatter(event.target.value || null)}
            aria-label={`Para qué ${matterWord}`}
          >
            <option value="">Ninguno</option>
            {open.map((matter) => (
              <option key={matter.id} value={matter.id}>
                {matter.name}
              </option>
            ))}
          </select>
          {matterId && <span className="text-xs text-muted-foreground">El documento entra a ese {matterWord}.</span>}
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground">Empezar con los datos de un documento</span>
          <select
            className={SELECT_CLASS}
            value={fromDocument === null ? "" : String(fromDocument)}
            onChange={(event) => {
              const column = event.target.value === "" ? null : Number(event.target.value)
              setFromDocument(column)
              const matter = open.find((m) => m.id === matterId) ?? null
              setValues(withMatter(column === null ? initialValues(rows, today) : valuesFromDocument(template, rows, column), rows, matter))
            }}
          >
            <option value="">Ninguno: empezar vacío</option>
            {template.documents.map((d, column) => (
              <option key={d.doc.contentId} value={column}>
                {d.doc.name}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="grid gap-6 lg:grid-cols-[minmax(0,26rem)_minmax(0,1fr)]">
        <div className="flex flex-col gap-3">
          <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">Los campos</h3>
          {rows.length === 0 && <p className="text-sm text-muted-foreground">Ningún campo cambia entre sus documentos.</p>}
          {rows.map((row) => (
            <FieldLine
              key={row.index}
              row={row}
              value={values[row.index]}
              onChange={(value) => set(row, value)}
              onChooseEntity={(option) => chooseEntity(row, option)}
              matters={open}
              parties={parties}
              searchEntities={searchEntities}
              chosen={chosen}
              words={words}
            />
          ))}
        </div>
        <div className="flex flex-col gap-2">
          <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">Vista previa</h3>
          <PreviewText pieces={shown.text} />
          {shown.form.length > 0 && (
            <div className="rounded-md border bg-background px-4 py-3 text-sm">
              <p className="mb-1 text-xs text-muted-foreground">Al final, los campos del formulario:</p>
              {shown.form.map(({ row, piece }) => (
                <p key={row.index}>
                  {row.name}: <PreviewValue piece={piece} />
                </p>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="flex flex-col gap-3 rounded-md border px-4 py-3 text-sm">
        <label className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground">Nombre del archivo (se guarda en «Generados», en la carpeta de trabajo)</span>
          <span className="flex items-center gap-2">
            <Input className="max-w-xl flex-1" value={fileName ?? suggested} onChange={(event) => setFileName(event.target.value)} />
            <span className="text-muted-foreground">.docx</span>
          </span>
        </label>
        {missing.length > 0 && (
          <p className="text-amber-800 dark:text-amber-300">
            {missing.length === 1 ? `Falta «${missing[0]}»` : `Faltan ${missing.length} campos: ${missing.map((m) => `«${m}»`).join(", ")}`}.
            Se puede generar igual: el Word {missing.length === 1 ? "lo muestra" : "los muestra"} entre corchetes, resaltado,
            para completar{missing.length === 1 ? "lo" : "los"} ahí.
          </p>
        )}
        <div>
          <Button
            disabled={busy}
            onClick={() =>
              onGenerate(
                generationRequest(template, name, rows, values, {
                  matterId,
                  fileName: fileName ?? suggested,
                  fromDocument: fromDocument === null ? null : (template.documents[fromDocument]?.doc.contentId ?? null),
                }),
              )
            }
          >
            <FileText /> {busy ? "Generando…" : "Generar el Word"}
          </Button>
        </div>
      </div>
    </>
  )
}

function Notice({ children }: { children: ReactNode }) {
  return (
    <div className="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm text-amber-800 dark:text-amber-300">
      {children}
    </div>
  )
}

/** One field to fill: its name, and what fills it. */
function FieldLine({
  row,
  value,
  onChange,
  onChooseEntity,
  matters,
  parties,
  searchEntities,
  chosen,
  words,
}: {
  row: GenerationRow
  value: FieldValue | undefined
  onChange: (value: FieldValue) => void
  onChooseEntity: (option: EntityOption) => void
  matters: MatterSummary[]
  parties: MatterParty[]
  searchEntities?: BaseLookups["searchEntities"]
  chosen: Record<string, string>
  words: Vocabulary
}) {
  const from = value?.reference?.startsWith("entity:") ? chosen[value.reference.slice("entity:".length)] : undefined
  const notes = [
    row.places > 1 ? plural(row.places, "lugar", "lugares") + " del texto" : null,
    row.optional && row.input !== "fixed" ? "puede quedar vacío" : null,
  ]
    .filter(Boolean)
    .join(" · ")
  return (
    <div className="flex flex-col gap-1 text-sm">
      <span className="flex flex-wrap items-baseline gap-2">
        <span className="font-medium">{row.name}</span>
        {notes && <span className="text-xs text-muted-foreground">{notes}</span>}
      </span>
      {row.input === "fixed" ? (
        <span className="text-muted-foreground">Fijo: «{row.field.fixedValue || "(nada)"}»</span>
      ) : row.input === "wording" ? (
        <WordingInput row={row} value={value} onChange={onChange} />
      ) : row.input === "entity" ? (
        <EntityInput row={row} value={value} onChange={onChange} onChoose={onChooseEntity} parties={parties} search={searchEntities} words={words} />
      ) : row.input === "matter" ? (
        <MatterInput row={row} value={value} onChange={onChange} matters={matters} words={words} />
      ) : row.input === "date" ? (
        <DateInput row={row} value={value} onChange={onChange} />
      ) : (
        <Input
          value={value?.value ?? ""}
          placeholder={row.example ? `Por ejemplo: ${row.example}` : undefined}
          onChange={(event) => onChange({ value: event.target.value, source: "hand", reference: null })}
        />
      )}
      {value?.source === "card" && (
        <span className="text-xs text-muted-foreground">
          Sugerido con lo que la base sabe {from ? `de «${from}»` : "de quien se eligió"}: revisalo.
        </span>
      )}
    </div>
  )
}

/**
 * A person or organization: one of the base, those of the matter first,
 * or written by hand. Choosing one fills its DNI, CUIT and address where
 * the template asks for them.
 */
function EntityInput({
  row,
  value,
  onChange,
  onChoose,
  parties,
  search,
  words,
}: {
  row: GenerationRow
  value: FieldValue | undefined
  onChange: (value: FieldValue) => void
  onChoose: (option: EntityOption) => void
  parties: MatterParty[]
  search?: BaseLookups["searchEntities"]
  words: Vocabulary
}) {
  const typed = value?.value ?? ""
  const [focused, setFocused] = useState(false)
  const [found, setFound] = useState<EntitySummary[]>([])
  const fromBase = value?.source === "entity"
  useEffect(() => {
    const query = typed.trim()
    if (!search || fromBase || query.length < SEARCH_FROM) {
      setFound([])
      return
    }
    let stale = false
    const timer = setTimeout(() => {
      search(row.entityKind, query)
        .then((list) => !stale && setFound(list))
        .catch(() => !stale && setFound([]))
    }, 250)
    return () => {
      stale = true
      clearTimeout(timer)
    }
  }, [typed, search, fromBase, row.entityKind])
  const options = focused && !fromBase ? entityOptions(row.entityKind, typed, parties, found) : []
  const where = row.entityKind ? entityTypeName(row.entityKind, true, words).toLowerCase() : words.knowledge.entities.toLowerCase()
  return (
    <div className="relative flex flex-col gap-1">
      <Input
        value={typed}
        placeholder={row.example ? `Por ejemplo: ${row.example}` : `Buscá entre ${where} o escribilo`}
        onFocus={() => setFocused(true)}
        onBlur={() => setTimeout(() => setFocused(false), 150)}
        onChange={(event) => onChange({ value: event.target.value, source: "hand", reference: null })}
      />
      <span className="text-xs text-muted-foreground">
        {fromBase ? `De «${words.knowledge.entities}».` : `Elegí de «${words.knowledge.entities}» o escribilo a mano.`}
      </span>
      {options.length > 0 && (
        <ul className="absolute top-9 z-10 flex w-full flex-col divide-y rounded-md border bg-popover text-sm shadow-md">
          {options.map((option) => (
            <li key={option.id}>
              <button
                type="button"
                className="flex w-full items-baseline justify-between gap-2 px-3 py-1.5 text-left hover:bg-muted"
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => {
                  onChoose(option)
                  setFocused(false)
                }}
              >
                <span className="break-words">{option.name}</span>
                <span className="shrink-0 text-xs text-muted-foreground">
                  {option.ofMatter ? `del ${words.knowledge.matter.toLowerCase()}` : entityTypeName(option.kind, false, words)}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

/** A matter: its name or the number of its record, from the matters of the base, or written by hand. */
function MatterInput({
  row,
  value,
  onChange,
  matters,
  words,
}: {
  row: GenerationRow
  value: FieldValue | undefined
  onChange: (value: FieldValue) => void
  matters: MatterSummary[]
  words: Vocabulary
}) {
  const choices = matters.flatMap((matter) => matterValues(matter).map((choice) => ({ ...choice, matter })))
  const current = choices.findIndex((choice) => `matter:${choice.matter.id}` === value?.reference && choice.value === value?.value)
  return (
    <div className="flex flex-col gap-1">
      <select
        className={SELECT_CLASS}
        value={current < 0 ? "" : String(current)}
        onChange={(event) => {
          const choice = choices[Number(event.target.value)]
          if (event.target.value !== "" && choice) onChange({ value: choice.value, source: "matter", reference: `matter:${choice.matter.id}` })
        }}
      >
        <option value="">Elegir un {words.knowledge.matter.toLowerCase()}…</option>
        {choices.map((choice, index) => (
          <option key={`${choice.matter.id}:${choice.label}`} value={index}>
            {choice.label === "su nombre" ? choice.value : `${choice.matter.name}: ${choice.label} (${choice.value})`}
          </option>
        ))}
      </select>
      <Input
        value={value?.value ?? ""}
        placeholder={row.example ? `Por ejemplo: ${row.example}` : undefined}
        onChange={(event) => onChange({ value: event.target.value, source: "hand", reference: null })}
      />
    </div>
  )
}

/** A date: written as the documents of the template write it, or chosen in the calendar. */
function DateInput({ row, value, onChange }: { row: GenerationRow; value: FieldValue | undefined; onChange: (value: FieldValue) => void }) {
  const style = dateStyleOf(row.field.values)
  const text = value?.value ?? ""
  return (
    <div className="flex items-center gap-2">
      <Input
        className="flex-1"
        value={text}
        placeholder={row.example ? `Por ejemplo: ${row.example}` : undefined}
        onChange={(event) => onChange({ value: event.target.value, source: "hand", reference: null })}
      />
      <input
        type="date"
        aria-label={`Elegir ${row.name} en el calendario`}
        className={cn(SELECT_CLASS, "w-36")}
        value={isoFromDate(text) ?? ""}
        onChange={(event) => event.target.value && onChange({ value: formatDateIn(event.target.value, style), source: "date", reference: null })}
      />
    </div>
  )
}

/** A wording that changes: one of the versions the documents have, none, or another one. */
function WordingInput({ row, value, onChange }: { row: GenerationRow; value: FieldValue | undefined; onChange: (value: FieldValue) => void }) {
  const current = value?.value ?? ""
  const [other, setOther] = useState(current !== "" && !row.versions.includes(current))
  const chosen = other ? OTHER : current
  return (
    <div className="flex flex-col gap-1">
      <select
        className={SELECT_CLASS}
        value={chosen}
        onChange={(event) => {
          const picked = event.target.value
          setOther(picked === OTHER)
          if (picked !== OTHER) onChange({ value: picked, source: "hand", reference: null })
        }}
      >
        <option value="">Ninguna: queda afuera</option>
        {row.versions.map((version) => (
          <option key={version} value={version}>
            {version.length > 80 ? `${version.slice(0, 80)}…` : version}
          </option>
        ))}
        <option value={OTHER}>Otra…</option>
      </select>
      {(other || (current !== "" && current.length > 80)) && (
        <textarea
          className={cn(FIELD_CLASS, "min-h-20")}
          value={current}
          placeholder="Lo que dice el documento en este lugar"
          onChange={(event) => onChange({ value: event.target.value, source: "hand", reference: null })}
        />
      )}
    </div>
  )
}

/** The text of the template with the values in place: what is given, highlighted; what is missing, in red. */
function PreviewText({ pieces }: { pieces: PreviewPiece[] }) {
  return (
    <div className="max-h-[36rem] overflow-y-auto rounded-md border bg-background px-4 py-3 font-serif text-[15px] leading-relaxed break-words whitespace-pre-wrap">
      {pieces.map((piece, index) => (piece.kind === "text" ? <span key={index}>{piece.text}</span> : <PreviewValue key={index} piece={piece} />))}
    </div>
  )
}

function PreviewValue({ piece }: { piece: PreviewPiece }) {
  switch (piece.kind) {
    case "text":
      return <>{piece.text}</>
    case "value":
      return <mark className="rounded bg-amber-500/20 px-0.5 text-inherit">{piece.text}</mark>
    case "missing":
      return <mark className="rounded bg-destructive/10 px-0.5 font-sans text-[13px] text-destructive">{piece.text}</mark>
    case "empty":
      return (
        <span className="font-sans text-xs text-muted-foreground" title="Queda afuera del documento">
          (nada)
        </span>
      )
  }
}

/**
 * In the screen of a document generated from a template (D-488): when,
 * from which template and for which matter it was generated.
 */
export function GeneratedNotice({
  line,
  onOpenTemplate,
  words = vocabulary(),
}: {
  line: GeneratedLine
  onOpenTemplate?: (principalLocationId: number) => void
  words?: Vocabulary
}) {
  return (
    <div className="mb-4 flex flex-wrap items-center justify-between gap-2 rounded-md border bg-muted/20 px-3 py-2 text-sm">
      <span>
        Generado el {formatDate(line.generatedAt)} desde la {words.templates.one} «{line.templateName}»
        {line.matterName ? ` para el ${words.knowledge.matter.toLowerCase()} «${line.matterName}»` : ""}.
      </span>
      {onOpenTemplate && line.templateLocation !== null && (
        <Button size="sm" variant="ghost" onClick={() => onOpenTemplate(line.templateLocation as number)}>
          Ver la {words.templates.one}
        </Button>
      )}
    </div>
  )
}

/** What was generated: where it is, and what can be done with it. */
function GeneratedNote({
  result,
  onOpen,
  onShowDocument,
  onAgain,
  words,
}: {
  result: GeneratedDocument
  onOpen?: (reveal: boolean) => void
  onShowDocument?: (locationId: number) => void
  onAgain?: () => void
  words: Vocabulary
}) {
  return (
    <div className="flex flex-col gap-3 rounded-md border border-emerald-500/40 bg-emerald-500/10 px-4 py-3 text-sm">
      <p className="font-medium">Listo: «{result.fileName}» quedó en «Generados», en la carpeta de trabajo.</p>
      {result.missing.length > 0 && (
        <p>
          Quedaron para completar en el Word, entre corchetes: {result.missing.map((m) => `«${m}»`).join(", ")}.
        </p>
      )}
      <p className="text-muted-foreground">
        Es un documento más de la base: se lee como cualquier otro
        {result.inMatter ? ` y ya está en su ${words.knowledge.matter.toLowerCase()}` : ""}.
      </p>
      <div className="flex flex-wrap gap-2">
        {onOpen && (
          <>
            <Button size="sm" variant="outline" onClick={() => onOpen(false)}>
              <ExternalLink /> Abrir en Word
            </Button>
            <Button size="sm" variant="outline" onClick={() => onOpen(true)}>
              <FolderOpen /> Mostrar en la carpeta
            </Button>
          </>
        )}
        {onShowDocument && result.locationId !== null && (
          <Button size="sm" variant="ghost" onClick={() => onShowDocument(result.locationId as number)}>
            Ver el documento
          </Button>
        )}
        {onAgain && (
          <Button size="sm" variant="ghost" onClick={onAgain}>
            Generar otro
          </Button>
        )}
      </div>
    </div>
  )
}

