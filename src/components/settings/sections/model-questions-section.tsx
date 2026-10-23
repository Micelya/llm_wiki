import { useEffect, useState } from "react"
import {
  lastModelQuestion,
  listModelQuestions,
  type AskedQuestion,
  type ModelQuestion,
  type QuestionCatalog,
  type QuestionSent,
} from "@/commands/docbase"
import { useDocbaseQuery } from "@/components/docbase/use-docbase-query"
import { formatCount } from "@/components/docbase/format"
import { choiceName } from "@/lib/ai-services"
import { useAiServicesStore } from "@/stores/ai-services-store"

/**
 * Ajustes → Lo que se le pregunta al modelo (P-278): read only. One row
 * per question the base and the chat ask the model, with its version.
 * Opened: what it is for and when it is asked, the catalogs it is written
 * with (the data folder's or the app's) and the engine of Servicios de IA
 * that answers it; how it is built, by the same functions as the reading
 * over an invented document, with what changes in each case marked; and
 * the last one the base kept, which carries the text of the firm's
 * documents and is read only when asked for.
 */
export function ModelQuestionsSection() {
  const query = useDocbaseQuery("model-questions", listModelQuestions)
  const catalog = useAiServicesStore((s) => s.catalog)

  return (
    <div className="space-y-6">
      <div>
        <h2 className="font-ui text-[1.75rem] leading-tight font-bold tracking-[-0.01em] text-strong">Lo que se le pregunta al modelo</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Cada pregunta que la base y el chat le hacen al modelo, tal como sale, con su versión. Esta pantalla es solo para
          mirar: nada de lo que muestra cambia lo que se pregunta.
        </p>
        <p className="mt-2 text-sm text-muted-foreground">
          «Cómo se arma» la escriben las mismas funciones que usa la lectura, sobre un documento inventado y con los
          catálogos en uso. Lo <mark className={MARK}>resaltado</mark> cambia en cada caso (el texto del documento, los
          candidatos); lo demás sale igual en todas.
        </p>
      </div>

      {query.error && (
        <p className="rounded-md border border-err-line px-3 py-2 text-sm text-destructive">
          No se pudieron leer las preguntas: {query.error}
        </p>
      )}
      {!query.data && !query.error && <p className="text-sm text-muted-foreground">Leyendo las preguntas…</p>}

      {query.data && (
        <div className="space-y-2">
          {query.data.map((question) => (
            <details key={question.id} className="group rounded-md border">
              <summary className="flex cursor-pointer flex-wrap items-baseline gap-x-3 gap-y-1 px-4 py-3">
                <span className="font-medium">{question.name}</span>
                <span className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs text-muted-foreground">
                  {question.version ?? "sin versión"}
                </span>
                <span className="ml-auto text-xs text-muted-foreground">
                  {choiceName(catalog, question.service, question.engine)}
                </span>
              </summary>
              <QuestionDetail question={question} engineName={choiceName(catalog, question.service, question.engine)} />
            </details>
          ))}
        </div>
      )}
    </div>
  )
}

const MARK = "rounded bg-warn-bg px-0.5 text-inherit ring-1 ring-warn-line"

function QuestionDetail({ question, engineName }: { question: ModelQuestion; engineName: string }) {
  return (
    <div className="space-y-4 border-t px-4 py-3 text-sm">
      <div className="space-y-1">
        <p>{question.purpose}</p>
        <p className="text-muted-foreground">
          <span className="font-medium text-foreground">Cuándo se hace: </span>
          {question.when}
        </p>
        {question.note && <p className="text-muted-foreground">{question.note}</p>}
      </div>

      <dl className="grid gap-x-4 gap-y-1 rounded-md bg-muted/30 px-3 py-2 text-xs sm:grid-cols-[max-content_1fr]">
        <dt className="font-medium">Versión</dt>
        <dd className="font-mono text-muted-foreground">{question.version ?? "no tiene"}</dd>
        <dt className="font-medium">Motor</dt>
        <dd className="text-muted-foreground">
          {engineName} <span>(Servicios de IA, «{question.service === "recognize" ? "Reconocer escaneos" : "Preguntar al modelo"}»)</span>
          {question.engineNote && <span className="block text-warn">{question.engineNote}</span>}
        </dd>
        <dt className="font-medium">Catálogos</dt>
        <dd className="text-muted-foreground">
          {question.catalogs.length === 0 ? (
            "ninguno"
          ) : (
            <ul className="space-y-0.5">
              {question.catalogs.map((catalog) => (
                <li key={catalog.file}>
                  <CatalogLine catalog={catalog} />
                </li>
              ))}
            </ul>
          )}
        </dd>
      </dl>

      <section className="space-y-2">
        <h4 className="font-semibold">Cómo se arma</h4>
        {question.noExample && <p className="text-muted-foreground">{question.noExample}</p>}
        {question.example.map((sent) => (
          <SentText key={sent.title} sent={sent} single={question.example.length === 1} />
        ))}
        {question.answerShape && (
          <details className="rounded-md bg-muted/30 px-3 py-2">
            <summary className="cursor-pointer text-xs">La forma de la respuesta que pide</summary>
            <pre className="mt-2 max-h-80 overflow-auto whitespace-pre-wrap break-words font-mono text-xs">{question.answerShape}</pre>
          </details>
        )}
      </section>

      <section className="space-y-2">
        <h4 className="font-semibold">La última que salió</h4>
        {question.kept ? (
          <>
            <p className="text-muted-foreground">{keptText(question)}</p>
            <LastAsked id={question.id} />
          </>
        ) : (
          <p className="text-muted-foreground">Esta pregunta no se guarda: no hay una última para mostrar.</p>
        )}
      </section>
    </div>
  )
}

function CatalogLine({ catalog }: { catalog: QuestionCatalog }) {
  return (
    <>
      {catalog.name} (<span className="font-mono">{catalog.file}</span>):{" "}
      {catalog.fromFolder ? `el de la carpeta de datos (${catalog.folderPath ?? catalog.file})` : "el que viene con la app"}
      {catalog.unusedFile && (
        <span className="block text-warn">
          Hay un archivo en la carpeta de datos que no se está usando: no es válido, o se puso después de abrir la app.
        </span>
      )}
    </>
  )
}

/** One text sent to the model, with what changes in each case marked. */
function SentText({ sent, single }: { sent: QuestionSent; single: boolean }) {
  const parts = [...new Set(sent.segments.map((segment) => segment.part).filter((part): part is string => !!part))]
  return (
    <div className="space-y-1">
      {!single && <p className="text-xs font-medium">{sent.title}</p>}
      <pre className="max-h-[32rem] overflow-auto whitespace-pre-wrap break-words rounded-md border bg-background px-3 py-2 font-mono text-xs leading-relaxed">
        {sent.segments.map((segment, index) =>
          segment.part ? (
            <mark key={index} className={MARK} title={`Cambia en cada caso: ${segment.part}`}>
              {segment.text}
            </mark>
          ) : (
            <span key={index}>{segment.text}</span>
          ),
        )}
      </pre>
      <p className="text-xs text-muted-foreground">
        {parts.length > 0 ? `Cambia en cada caso: ${parts.join("; ")}.` : "Sale igual en todos los casos."}
      </p>
    </div>
  )
}

/** How many the base keeps of each version, the current one first. */
function keptText(question: ModelQuestion): string {
  if (question.keptByVersion.length === 0) return "Todavía no se guardó ninguna."
  const counts = question.keptByVersion.map(
    (v) => `${formatCount(v.count)} de ${v.version}${v.version === question.version ? " (la vigente)" : ""}`,
  )
  const current = question.keptByVersion.some((v) => v.version === question.version)
  return `Guardadas: ${counts.join(" · ")}.${current ? "" : " Ninguna de la versión vigente."}`
}

function formatMoment(ms: number): string {
  return new Date(ms).toLocaleString("es-AR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  })
}

/**
 * The last question of the current version the base kept, read when the
 * person opens it: it carries the text of the firm's documents.
 */
function LastAsked({ id }: { id: string }) {
  const [open, setOpen] = useState(false)
  const [found, setFound] = useState<{ asked: AskedQuestion | null } | null>(null)
  const [problem, setProblem] = useState<string | null>(null)

  useEffect(() => {
    if (!open) return
    let current = true
    setFound(null)
    setProblem(null)
    lastModelQuestion(id)
      .then((asked) => {
        if (current) setFound({ asked })
      })
      .catch((cause: unknown) => {
        if (current) setProblem(String(cause))
      })
    return () => {
      current = false
    }
  }, [open, id])
  const asked = found?.asked ?? null

  return (
    <details className="rounded-md bg-muted/30 px-3 py-2" onToggle={(event) => setOpen(event.currentTarget.open)}>
      <summary className="cursor-pointer text-xs">
        Ver la última de la versión vigente (contiene texto de documentos del estudio)
      </summary>
      <div className="mt-2 space-y-2 text-xs">
        <p className="text-warn">
          Es la pregunta completa tal como salió: trae el texto de documentos del estudio. No la copies fuera de la app.
        </p>
        {open && !found && !problem && <p className="text-muted-foreground">Leyendo…</p>}
        {problem && <p className="text-destructive">{problem}</p>}
        {found && !asked && <p className="text-muted-foreground">Todavía no salió ninguna de la versión vigente.</p>}
        {asked && (
          <>
            <p className="text-muted-foreground">
              Salió el {formatMoment(asked.askedAt)}, con {asked.engine}, versión <span className="font-mono">{asked.version}</span>.
            </p>
            <pre className="max-h-[32rem] overflow-auto whitespace-pre-wrap break-words rounded-md border bg-background px-3 py-2 font-mono leading-relaxed">
              {asked.prompt}
            </pre>
          </>
        )}
      </div>
    </details>
  )
}
