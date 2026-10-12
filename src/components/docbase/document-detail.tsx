import { useState, type ReactNode } from "react"
import { ArrowLeft, ExternalLink, FolderOpen } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  getDocument,
  getGeneralProject,
  openLocation,
  pagesCiting,
  setLocationLabel,
  type DocumentDetail,
  type LocationRef,
} from "@/commands/docbase"
import { isSameProjectPath } from "@/lib/docbase-wiki-bridge"
import { normalizePath } from "@/lib/path-utils"
import { useDocbaseStore } from "@/stores/docbase-store"
import { useWikiStore } from "@/stores/wiki-store"
import { DOCUMENT_STATES, DocumentStateBadge } from "./document-state"
import { LabelChip, LabelPicker } from "./labels"
import { EmptyNote, ErrorNote } from "./page"
import { formatCount, formatDate, formatSize } from "./format"
import { useDocbaseQuery } from "./use-docbase-query"

/** Text longer than this is cut on screen; the whole text stays in the base. */
const SHOWN_TEXT_LIMIT = 200_000

/**
 * One document, laid out like an encyclopedia article: the text in the
 * middle, a fact box on the right, its locations below.
 */
export function DocumentDetailView({ locationId, onBack }: { locationId: number; onBack: () => void }) {
  const detail = useDocbaseQuery(`document:${locationId}`, () => getDocument(locationId))
  const [problem, setProblem] = useState<string | null>(null)

  async function handleOpen(id: number, reveal: boolean) {
    setProblem(null)
    try {
      await openLocation(id, reveal)
    } catch (cause) {
      setProblem(String(cause))
    }
  }

  const document = detail.data

  return (
    <div className="flex h-full flex-col">
      <header className="shrink-0 border-b px-8 pt-4">
        <button
          className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
          onClick={onBack}
        >
          <ArrowLeft className="size-4" /> Documentos
        </button>
        <div className="mt-2 flex items-end justify-between gap-4 pb-3">
          <h1 className="min-w-0 font-serif text-2xl break-words">{document?.name ?? "Documento"}</h1>
          {document && (
            <div className="flex shrink-0 gap-2">
              <Button variant="outline" onClick={() => handleOpen(document.locationId, false)}>
                <ExternalLink /> Abrir el original
              </Button>
              <Button variant="outline" onClick={() => handleOpen(document.locationId, true)}>
                <FolderOpen /> Ver en su carpeta
              </Button>
            </div>
          )}
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-8 py-6">
        {(problem || detail.error) && (
          <div className="mb-4">
            <ErrorNote>{problem ?? detail.error}</ErrorNote>
          </div>
        )}
        {!detail.loading && !document && !detail.error && (
          <EmptyNote>Este archivo ya no está en el inventario.</EmptyNote>
        )}
        {document && (
          <div className="mx-auto grid max-w-6xl gap-8 lg:grid-cols-[minmax(0,1fr)_20rem]">
            <main className="min-w-0">
              <DocumentText document={document} />
              <Section title={`Ubicaciones (${document.locations.length || 1})`}>
                <Locations document={document} onOpen={handleOpen} />
              </Section>
              <WikiPages locationId={document.locationId} />
            </main>
            <aside>
              <FactBox document={document} onChanged={detail.reload} onProblem={setProblem} />
            </aside>
          </div>
        )}
      </div>
    </div>
  )
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mt-8">
      <h2 className="border-b pb-1 font-serif text-xl">{title}</h2>
      <div className="mt-3">{children}</div>
    </section>
  )
}

function DocumentText({ document }: { document: DocumentDetail }) {
  const info = DOCUMENT_STATES[document.state] ?? DOCUMENT_STATES.unread
  if (!document.text) {
    return (
      <div className="rounded-md border border-dashed px-4 py-6 text-sm text-muted-foreground">
        <p className="font-medium text-foreground">{info.label}</p>
        <p className="mt-1">{info.hint}</p>
        {document.error && <p className="mt-2 font-mono text-xs break-words">{document.error}</p>}
      </div>
    )
  }
  const cut = document.text.length > SHOWN_TEXT_LIMIT
  return (
    <div>
      {document.state === "needs-recognition" && (
        <p className="mb-3 rounded-md border border-dashed px-3 py-2 text-sm text-muted-foreground">
          Parte de este documento es imagen: lo que sigue es solo el texto que se pudo extraer sin reconocerlo.
        </p>
      )}
      <div className="font-serif text-[15px] leading-relaxed break-words whitespace-pre-wrap">
        {cut ? document.text.slice(0, SHOWN_TEXT_LIMIT) : document.text}
      </div>
      {cut && (
        <p className="mt-3 text-sm text-muted-foreground">
          El texto sigue. Se muestran los primeros {formatCount(SHOWN_TEXT_LIMIT)} caracteres.
        </p>
      )}
    </div>
  )
}

function Locations({
  document,
  onOpen,
}: {
  document: DocumentDetail
  onOpen: (locationId: number, reveal: boolean) => void
}) {
  const openDocument = useDocbaseStore((s) => s.openDocument)
  // A file that was not read yet has no document to share locations with.
  const locations: LocationRef[] =
    document.locations.length > 0
      ? document.locations
      : [
          {
            locationId: document.locationId,
            originName: document.originName,
            originReachable: document.originReachable,
            relPath: document.relPath,
            path: document.path,
            mtimeMs: document.mtimeMs,
            sameFile: true,
          },
        ]
  return (
    <ul className="flex flex-col divide-y rounded-md border">
      {locations.map((location) => {
        const current = location.locationId === document.locationId
        return (
          <li key={location.locationId} className="flex items-center justify-between gap-3 px-3 py-2">
            <div className="min-w-0 text-sm">
              <div className="break-words">
                {current ? (
                  <span className="font-medium">{location.relPath}</span>
                ) : (
                  <button
                    className="text-left text-primary hover:underline"
                    onClick={() => openDocument(location.locationId)}
                  >
                    {location.relPath}
                  </button>
                )}
              </div>
              <div className="text-xs text-muted-foreground">
                {location.originName} · modificado el {formatDate(location.mtimeMs)}
                {current && " · esta ubicación"}
                {!location.sameFile && " · mismo texto en otro archivo"}
                {location.originReachable === false && " · origen sin conexión"}
              </div>
            </div>
            <Button
              variant="ghost"
              size="icon-sm"
              title="Ver en su carpeta"
              onClick={() => onOpen(location.locationId, true)}
            >
              <FolderOpen />
            </Button>
          </li>
        )
      })}
    </ul>
  )
}

/**
 * The pages of the general project's wiki that came from this document
 * or from any copy of it. They open here while that wiki is the open
 * project.
 */
function WikiPages({ locationId }: { locationId: number }) {
  const project = useWikiStore((s) => s.project)
  const openPathInPreview = useWikiStore((s) => s.openPathInPreview)
  const pages = useDocbaseQuery(`pages-citing:${locationId}`, () => pagesCiting(locationId))
  const general = useDocbaseQuery("general-project", getGeneralProject)
  const canOpen = isSameProjectPath(general.data?.path, project?.path)

  if (!pages.data) return null
  return (
    <Section title={`En el wiki (${pages.data.length})`}>
      {pages.data.length === 0 ? (
        <p className="text-sm text-muted-foreground">Ninguna página del wiki salió de este documento todavía.</p>
      ) : (
        <ul className="flex flex-col gap-1 text-sm">
          {pages.data.map((page) => (
            <li key={page.path} className="flex flex-wrap items-baseline gap-2">
              {canOpen ? (
                <button
                  className="text-left text-primary hover:underline"
                  onClick={() => openPathInPreview(normalizePath(page.file))}
                >
                  {page.title}
                </button>
              ) : (
                <span>{page.title}</span>
              )}
              <span className="text-xs text-muted-foreground">
                {page.kind && `${page.kind} · `}
                {page.path}
              </span>
            </li>
          ))}
        </ul>
      )}
    </Section>
  )
}

function FactBox({
  document,
  onChanged,
  onProblem,
}: {
  document: DocumentDetail
  onChanged: () => void
  onProblem: (problem: string | null) => void
}) {
  return (
    <div className="rounded-md border bg-muted/30 text-sm">
      <div className="border-b bg-muted px-3 py-2 text-center font-semibold break-words">{document.name}</div>
      <FactRows>
        <Fact label="Estado">
          <DocumentStateBadge state={document.state} />
        </Fact>
        <Fact label="Origen">{document.originName}</Fact>
        <Fact label="Carpeta">{document.folder ? document.folder.split("/").join(" › ") : "—"}</Fact>
      </FactRows>
      <FactHeading>Etiquetas</FactHeading>
      <div className="flex flex-col gap-2 px-3 py-2">
        <div className="flex flex-wrap gap-1">
          {document.labels.length === 0 && <span className="text-xs text-muted-foreground">Sin etiquetas</span>}
          {document.labels.map((label) => (
            <LabelChip
              key={label.id}
              label={label}
              onRemove={() =>
                setLocationLabel(document.locationId, label.id, false)
                  .then(onChanged)
                  .catch((cause) => onProblem(String(cause)))
              }
            />
          ))}
        </div>
        <LabelPicker
          exclude={document.labels.map((label) => label.id)}
          onPick={(label) => setLocationLabel(document.locationId, label.id, true).then(onChanged)}
          onProblem={onProblem}
        />
        <p className="text-xs text-muted-foreground">
          Lo que pongas o quites acá vale para este archivo, digan lo que digan las reglas de su carpeta.
        </p>
      </div>
      <FactHeading>Archivo</FactHeading>
      <FactRows>
        <Fact label="Tamaño">{formatSize(document.size)}</Fact>
        <Fact label="Modificado">{formatDate(document.mtimeMs)}</Fact>
        <Fact label="En la base desde">{formatDate(document.firstSeenAt)}</Fact>
        <Fact label="Copias">{document.copies > 1 ? `${document.copies} ubicaciones` : "Única"}</Fact>
        {document.wordCount !== null && <Fact label="Palabras">{formatCount(document.wordCount)}</Fact>}
        {document.textEngine && (
          <Fact label="Texto obtenido por">
            {document.textEngine === "extraction" ? "Extracción directa" : `Reconocimiento (${document.textEngine})`}
          </Fact>
        )}
        {document.fileHash && (
          <Fact label="Huella">
            <span className="font-mono text-xs" title={document.fileHash}>
              {document.fileHash.slice(0, 12)}…
            </span>
          </Fact>
        )}
      </FactRows>
    </div>
  )
}

function FactHeading({ children }: { children: ReactNode }) {
  return (
    <div className="border-y bg-muted/60 px-3 py-1 text-center text-xs font-semibold text-muted-foreground">
      {children}
    </div>
  )
}

function FactRows({ children }: { children: ReactNode }) {
  return <dl className="divide-y">{children}</dl>
}

function Fact({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid grid-cols-[38%_minmax(0,1fr)] gap-2 px-3 py-1.5">
      <dt className="text-xs font-medium text-muted-foreground">{label}</dt>
      <dd className="min-w-0 break-words">{children}</dd>
    </div>
  )
}
