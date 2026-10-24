import { useState, type ReactNode } from "react"
import { Check, Copy, ExternalLink, FolderOpen, MessageSquare } from "lucide-react"
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
import { DocumentRecognition } from "./recognition"
import { SimilarDocuments } from "./similar-documents"
import { GeneratedNotice } from "./template-generate-view"
import { useOpenTemplate } from "./template-panel"
import { useDocbaseQuery, type DocbaseQuery } from "./use-docbase-query"
import { DocumentEntitiesSection } from "@/components/knowledge/document-entities"
import { usePageWords } from "@/components/knowledge/article-text"
import { ArticleBody, indexOf, useArticle, type ArticleModel } from "@/components/knowledge/article-page"
import { DetailTabs, DetailTitle, NoArticle, useDetailTab, type DetailTab } from "@/components/knowledge/detail-tabs"
import { articleBody, withoutEntityBlock, withoutListSection } from "@/components/knowledge/knowledge"
import { InfoBox, InfoHeading, InfoRow, InfoRows, PageAside, PageIndex } from "@/components/knowledge/page-aside"
import { Section } from "@/components/knowledge/shared"
import { engineName, withoutRecognitionMarkers } from "@/lib/text-recognition"
import { askChatAbout, chatDocumentOf } from "@/lib/chat-documents"

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
  return <DocumentScreen key={contentId ?? "none"} locationId={locationId} detail={detail} contentId={contentId} onBack={onBack} initialTab={initialTab} />
}

function DocumentScreen({
  locationId,
  detail,
  contentId,
  onBack,
  initialTab,
}: {
  locationId: number
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
  const [tab, setTab] = useDetailTab(withoutArticle, initialTab, { view: "documents", subject: locationId })
  const openTemplate = useOpenTemplate()
  const pageWord = usePageWords()
  // Its article without what «Datos y correcciones» shows in its own sections (where it is, whom it names).
  const shownBody = page ? withoutEntityBlock(articleBody(page.body)) : null
  const model = useArticle(shownBody === null ? null : pageWord ? withoutListSection(shownBody, pageWord("docLocations")) : shownBody)
  const index = indexOf(model, page ? [{ id: "notas", label: "Notas" }] : [])

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
      current={page?.title || document?.name || "Documento"}
      kicker="Documento"
      // Its article's title, readable (P-182); the name of its file stays in
      // its data, at the head of the fact box.
      title={<DetailTitle>{page?.title || document?.name || "Documento"}</DetailTitle>}
      facts={document?.docTypeName ? <span className="first-letter:uppercase">{document.docTypeName}</span> : null}
      actions={
        document ? (
          <>
            <Button variant="secondary" onClick={() => handleOpen(document.locationId, false)}>
              <ExternalLink /> Abrir el original
            </Button>
            <Button variant="outline" onClick={() => handleOpen(document.locationId, true)}>
              <FolderOpen /> Ver en su carpeta
            </Button>
            {/* The chat reads its text from the base and cites it (P-243). */}
            <Button
              variant="outline"
              disabled={document.contentId === null || document.state !== "processed"}
              title={
                document.contentId === null || document.state !== "processed"
                  ? "Todavía no tiene su texto en la base: el chat no tendría qué leer."
                  : "Abre el chat con este documento adjunto"
              }
              onClick={() => askChatAbout(chatDocumentOf(document))}
            >
              <MessageSquare /> Preguntar al chat sobre este documento
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
          {document?.generated && <GeneratedNotice line={document.generated} onOpenTemplate={openTemplate} />}
        </>
      }
      article={
        document ? (
          <DocumentArticle
            document={document}
            article={page}
            model={model}
            loading={article.loading}
            onShowData={() => setTab("data")}
          />
        ) : null
      }
      articleAside={
        page && index.length > 1 ? (
          <PageAside index={<PageIndex entries={index} jump={{ label: "Datos y correcciones", onClick: () => setTab("data") }} />} />
        ) : null
      }
      data={
        document ? (
          document.contentId === null ? (
            <>
              <DocumentText document={document} />
              <Section title="Ubicaciones" count={document.locations.length || 1}>
                <Locations document={document} onOpen={handleOpen} />
              </Section>
            </>
          ) : (
            <>
              {/* A scan first: what it needs is a person's action. */}
              <DocumentRecognition document={document} />
              <DocumentEntitiesSection locationId={document.locationId} />
              <Section title="Ubicaciones" count={document.locations.length || 1}>
                <Locations document={document} onOpen={handleOpen} />
              </Section>
              <SimilarDocuments locationId={document.locationId} name={document.name} onProblem={setProblem} />
              {/* The whole text last: it is the longest. */}
              <Section title="Texto">
                <DocumentText document={document} />
              </Section>
            </>
          )
        ) : null
      }
      dataAside={document ? <PageAside box={<FactBox document={document} onChanged={detail.reload} onProblem={setProblem} />} /> : null}
    />
  )
}

function DocumentText({ document }: { document: DocumentDetail }) {
  const info = DOCUMENT_STATES[document.state] ?? DOCUMENT_STATES.unread
  if (!document.text) {
    return (
      <div className="rounded-[10px] border border-dashed border-border-strong px-4 py-6 text-sm text-muted-foreground">
        <p className="font-ui font-bold text-strong">{info.label}</p>
        <p className="mt-1">{info.hint}</p>
        {document.error && <p className="mt-2 font-mono text-xs break-words">{document.error}</p>}
      </div>
    )
  }
  // Which engine read each page is said once, above.
  const shown = withoutRecognitionMarkers(document.text)
  const cut = shown.length > SHOWN_TEXT_LIMIT
  return (
    <div>
      {document.state === "needs-recognition" && (
        <p className="mb-3 rounded-[10px] border border-dashed border-border-strong px-3 py-2 text-sm text-muted-foreground">
          Parte de este documento es imagen: lo que sigue es solo el texto que se pudo extraer sin reconocerlo.
        </p>
      )}
      <div className="rounded-[10px] border bg-card px-5 py-4 text-[0.9375rem] leading-relaxed break-words whitespace-pre-wrap">
        {cut ? shown.slice(0, SHOWN_TEXT_LIMIT) : shown}
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
    <ul className="flex flex-col divide-y rounded-[10px] border bg-card">
      {locations.map((location) => {
        const current = location.locationId === document.locationId
        return (
          <li key={location.locationId} className="flex items-center justify-between gap-3 px-3.5 py-2">
            <div className="min-w-0 text-sm">
              <div className="break-words">
                {current ? (
                  <span className="font-medium">{location.relPath}</span>
                ) : (
                  <button
                    className="text-left text-link hover:underline"
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
  model,
  loading,
  onShowData,
}: {
  document: DocumentDetail
  article: KnowledgeArticle | null
  model: ArticleModel | null
  loading: boolean
  onShowData: () => void
}) {
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
  if (!model) return null
  return (
    <article>
      <ArticleBody model={model} withRoles={false} onShowData={onShowData} writtenAt={article.writtenAt} />
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
    <InfoBox kind="Archivo" name={document.name}>
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
      <div className="flex flex-col gap-2 px-4 py-2.5">
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
            {document.textEngine === "extraction"
              ? "Extracción directa"
              : `Reconocimiento: ${engineName(document.textEngine)}${document.textAt ? `, el ${formatDate(document.textAt)}` : ""}`}
          </Fact>
        )}
        {document.fileHash && (
          <Fact label="Huella">
            <HashValue hash={document.fileHash} />
          </Fact>
        )}
      </FactRows>
    </InfoBox>
  )
}

function FactHeading({ children }: { children: ReactNode }) {
  return <InfoHeading>{children}</InfoHeading>
}

function FactRows({ children }: { children: ReactNode }) {
  return <InfoRows>{children}</InfoRows>
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
        className="shrink-0 rounded p-0.5 text-muted-foreground hover:bg-hover hover:text-strong"
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
  return <InfoRow label={label}>{children}</InfoRow>
}
