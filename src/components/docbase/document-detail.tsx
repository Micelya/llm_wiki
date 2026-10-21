import { useState, type ReactNode } from "react"
import { Check, Copy, ExternalLink, FolderOpen } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  getDocument,
  getDocumentArticle,
  openLocation,
  setLocationLabel,
  type DocumentDetail,
  type KnowledgeArticle,
  type LocationRef,
} from "@/commands/docbase"
import { useDocbaseStore } from "@/stores/docbase-store"
import { DOCUMENT_STATES, DocumentStateBadge } from "./document-state"
import { LabelChip, LabelPicker } from "./labels"
import { EmptyNote, ErrorNote } from "./page"
import { formatCount, formatDate, formatSize } from "./format"
import { DocumentTypeFacts } from "./document-types-view"
import { SimilarDocuments } from "./similar-documents"
import { useDocbaseQuery, type DocbaseQuery } from "./use-docbase-query"
import { DocumentEntitiesSection } from "@/components/knowledge/document-entities"
import { ArticleText, usePageWords } from "@/components/knowledge/article-text"
import { DetailTabs, NoArticle, useDetailTab, type DetailTab } from "@/components/knowledge/detail-tabs"
import { articleBody, withoutEntityBlock, withoutListSection } from "@/components/knowledge/knowledge"

/** Text longer than this is cut on screen; the whole text stays in the base. */
const SHOWN_TEXT_LIMIT = 200_000

/**
 * One document, with the same two tabs as a person or organization
 * (D-389, D-423): «Artículo», its page as the base keeps it, and «Datos y
 * correcciones», everything else: who and what it names and its matters,
 * its places and copies, its similar documents, its whole text, and a fact
 * box on the right with its type, its data and its labels. The notes of a
 * person at the foot of both.
 */
export function DocumentDetailView({
  locationId,
  onBack,
  initialTab = null,
}: {
  locationId: number
  onBack: () => void
  initialTab?: DetailTab | null
}) {
  const detail = useDocbaseQuery(`document:${locationId}`, () => getDocument(locationId))
  const contentId = detail.data?.contentId ?? null
  // Its article is asked once the document says which it is: a new
  // document (its text changed) starts the screen over.
  return <DocumentScreen key={contentId ?? "none"} detail={detail} contentId={contentId} onBack={onBack} initialTab={initialTab} />
}

function DocumentScreen({
  detail,
  contentId,
  onBack,
  initialTab,
}: {
  detail: DocbaseQuery<DocumentDetail | null>
  contentId: number | null
  onBack: () => void
  initialTab: DetailTab | null
}) {
  const article = useDocbaseQuery(`document-article:${contentId ?? "none"}`, () =>
    contentId === null ? Promise.resolve(null) : getDocumentArticle(contentId),
  )
  const [problem, setProblem] = useState<string | null>(null)
  const document = detail.data
  // A file not read yet has no page: whatever the query says.
  const page = contentId === null ? null : article.data
  // A file not read yet, or a document whose page is not written: nothing to read.
  const withoutArticle = document !== null && (contentId === null || (!article.loading && !page))
  const [tab, setTab] = useDetailTab(withoutArticle, initialTab)

  async function handleOpen(id: number, reveal: boolean) {
    setProblem(null)
    try {
      await openLocation(id, reveal)
    } catch (cause) {
      setProblem(String(cause))
    }
  }

  return (
    <DetailTabs
      back="Documentos"
      onBack={onBack}
      title={<h1 className="font-serif text-2xl break-words">{document?.name ?? "Documento"}</h1>}
      actions={
        document ? (
          <>
            <Button variant="outline" onClick={() => handleOpen(document.locationId, false)}>
              <ExternalLink /> Abrir el original
            </Button>
            <Button variant="outline" onClick={() => handleOpen(document.locationId, true)}>
              <FolderOpen /> Ver en su carpeta
            </Button>
          </>
        ) : null
      }
      tab={tab}
      onTab={setTab}
      notesOf={page?.objectId ?? null}
      notice={
        <>
          {(problem || detail.error) && (
            <div className="mb-4">
              <ErrorNote>{problem ?? detail.error}</ErrorNote>
            </div>
          )}
          {!detail.loading && !document && !detail.error && <EmptyNote>Este archivo ya no está en el inventario.</EmptyNote>}
        </>
      }
      article={
        document ? (
          <DocumentArticle
            document={document}
            article={page}
            loading={article.loading}
            onShowData={() => setTab("data")}
          />
        ) : null
      }
      data={
        document ? (
          <div className="grid gap-8 lg:grid-cols-[minmax(0,1fr)_20rem]">
            <main className="min-w-0">
              {document.contentId === null ? (
                <>
                  <DocumentText document={document} />
                  <Section title={`Ubicaciones (${document.locations.length || 1})`}>
                    <Locations document={document} onOpen={handleOpen} />
                  </Section>
                </>
              ) : (
                <>
                  <DocumentEntitiesSection locationId={document.locationId} />
                  <Section title={`Ubicaciones (${document.locations.length || 1})`}>
                    <Locations document={document} onOpen={handleOpen} />
                  </Section>
                  <SimilarDocuments locationId={document.locationId} name={document.name} onProblem={setProblem} />
                  {/* The whole text last: it is the longest. */}
                  <Section title="Texto">
                    <DocumentText document={document} />
                  </Section>
                </>
              )}
            </main>
            <aside>
              <FactBox document={document} onChanged={detail.reload} onProblem={setProblem} />
            </aside>
          </div>
        ) : null
      }
    />
  )
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="mt-8 first:mt-0">
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
 * The article of the document (D-407): its page as the projection last
 * wrote it in the base, for every version of it (its topic, the essential
 * with its quotes and how sure each point is, its data, the amounts, its
 * versions and the documents it is related to), without what «Datos y
 * correcciones» shows in its own sections (where it is, whom it names).
 */
function DocumentArticle({
  document,
  article,
  loading,
  onShowData,
}: {
  document: DocumentDetail
  article: KnowledgeArticle | null
  loading: boolean
  onShowData: () => void
}) {
  const pageWord = usePageWords()
  if (document.contentId !== null && loading && !article) return null
  if (!article) {
    return (
      <NoArticle onShowData={onShowData}>
        {document.contentId === null
          ? "Este archivo todavía no se leyó: no tiene artículo."
          : "Su artículo todavía no está escrito: la app lo escribe en su próxima pasada."}
      </NoArticle>
    )
  }
  if (!pageWord) return null
  const body = withoutEntityBlock(articleBody(article.body))
  return (
    <article>
      <ArticleText body={withoutListSection(body, pageWord("docLocations"))} writtenAt={article.writtenAt} />
    </article>
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
      {document.contentId !== null && (
        <DocumentTypeFacts
          locationId={document.locationId}
          heading={(children) => <FactHeading>{children}</FactHeading>}
          row={(label, children) => (
            <Fact key={label} label={label}>
              {children}
            </Fact>
          )}
        />
      )}
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
            <HashValue hash={document.fileHash} />
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

/** The whole fingerprint, selectable, with a button that copies it. */
function HashValue({ hash }: { hash: string }) {
  const [copied, setCopied] = useState(false)
  const copy = () => {
    navigator.clipboard
      .writeText(hash)
      .then(() => {
        setCopied(true)
        setTimeout(() => setCopied(false), 1500)
      })
      .catch(() => setCopied(false))
  }
  return (
    <span className="inline-flex items-start gap-1">
      <span className="font-mono text-xs break-all select-all">{hash}</span>
      <button
        type="button"
        className="shrink-0 rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
        title={copied ? "Copiada" : "Copiar la huella"}
        aria-label="Copiar la huella"
        onClick={copy}
      >
        {copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
      </button>
    </span>
  )
}

function Fact({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grid grid-cols-[38%_minmax(0,1fr)] gap-2 px-3 py-1.5">
      <dt className="text-xs font-medium text-muted-foreground">{label}</dt>
      <dd className="min-w-0 break-words">{children}</dd>
    </div>
  )
}
