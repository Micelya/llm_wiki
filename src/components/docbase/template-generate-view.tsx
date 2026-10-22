import { useMemo, useState, type ReactNode } from "react"
import { ArrowLeft, ExternalLink, FileText, FolderOpen } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  generateDocument,
  getSimilarTemplate,
  listMatters,
  openGenerated,
  type GenerateRequest,
  type GeneratedDocument,
  type MatterSummary,
  type Template,
} from "@/commands/docbase"
import { useAction, useOpenDocument } from "@/components/knowledge/shared"
import { cn } from "@/lib/utils"
import { vocabulary, type Vocabulary } from "@/vocabulary"
import { plural } from "./format"
import { DocbasePage, EmptyNote, ErrorNote } from "./page"
import { templateName } from "./template"
import {
  generationRequest,
  generationRows,
  initialValues,
  missingNames,
  preview,
  suggestedFileName,
  todayIso,
  valuesFromDocument,
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
  today = todayIso(),
  words = vocabulary(),
}: {
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
  const [values, setValues] = useState<FieldValues>(() => initialValues())
  const [matterId, setMatterId] = useState<string | null>(null)
  const [fromDocument, setFromDocument] = useState<number | null>(null)
  const [fileName, setFileName] = useState<string | null>(null)
  const name = templateName(template, template.documents[0]?.doc.name ?? "", words)
  const shown = preview(template, rows, values)
  const missing = missingNames(rows, values)
  const suggested = suggestedFileName(name, rows, values, today)
  const matterWord = words.knowledge.matter.toLowerCase()
  const asked = rows.filter((row) => row.input !== "fixed")
  const set = (row: GenerationRow, value: FieldValue) => setValues({ ...values, [row.index]: value })

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
      {template.state === "proposed" && (
        <Notice>
          Esta {words.templates.one} es una propuesta: nadie confirmó todavía que sus documentos son de la misma familia. Se
          puede generar igual; revisá el texto antes de usarlo.
        </Notice>
      )}
      {template.format === "scan" && (
        <Notice>El texto salió de un escaneo: lo leyó el reconocimiento y puede tener errores. Revisalo en el Word.</Notice>
      )}

      <div className="flex flex-wrap items-end gap-4 rounded-md border bg-muted/20 px-4 py-3 text-sm">
        <label className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground">Para qué {matterWord} (opcional)</span>
          <select
            className={SELECT_CLASS}
            value={matterId ?? ""}
            onChange={(event) => setMatterId(event.target.value || null)}
            aria-label={`Para qué ${matterWord}`}
          >
            <option value="">Ninguno</option>
            {matters
              .filter((matter) => matter.confirmation !== "discarded")
              .map((matter) => (
                <option key={matter.id} value={matter.id}>
                  {matter.name}
                </option>
              ))}
          </select>
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground">Empezar con los datos de un documento</span>
          <select
            className={SELECT_CLASS}
            value={fromDocument === null ? "" : String(fromDocument)}
            onChange={(event) => {
              const column = event.target.value === "" ? null : Number(event.target.value)
              setFromDocument(column)
              setValues(column === null ? initialValues() : valuesFromDocument(template, rows, column))
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
            <FieldLine key={row.index} row={row} value={values[row.index]} onChange={(value) => set(row, value)} />
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
function FieldLine({ row, value, onChange }: { row: GenerationRow; value: FieldValue | undefined; onChange: (value: FieldValue) => void }) {
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
      ) : (
        <Input
          value={value?.value ?? ""}
          placeholder={row.example ? `Por ejemplo: ${row.example}` : undefined}
          onChange={(event) => onChange({ value: event.target.value, source: "hand", reference: null })}
        />
      )}
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

