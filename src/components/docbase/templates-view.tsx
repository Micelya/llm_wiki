import { useState } from "react"
import { ArrowLeft, Pencil } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  chooseTemplateField,
  getSimilarTemplate,
  listTemplates,
  nameTemplate,
  type FieldChoice,
  type Template,
  type TemplateSummary,
} from "@/commands/docbase"
import { goToTarget } from "@/components/attention/navigate"
import { NameForm } from "@/components/knowledge/matter-edit"
import { useAction, useOpenDocument } from "@/components/knowledge/shared"
import { useTemplatesStore } from "@/stores/templates-store"
import { vocabulary, type Vocabulary } from "@/vocabulary"
import { plural } from "./format"
import { DocbasePage, EmptyNote, ErrorNote, StatusBadge } from "./page"
import { automaticTemplateName, fieldRows, fieldsLine, formatName, templateName, templatesSummary } from "./template"
import { TemplatePanel, TemplateStateBadge } from "./template-panel"
import { useDocbaseQuery } from "./use-docbase-query"

/**
 * The section «Plantillas» (D-466, D-488): every template the base found
 * in its documents, and the detail of one. The name of the section and of
 * a template come from the vocabulary of the profession. A template that
 * is a proposal (its family is not confirmed) can be seen and used, with a
 * notice.
 */
export function TemplatesView() {
  const principal = useTemplatesStore((s) => s.principal)
  const openTemplate = useTemplatesStore((s) => s.openTemplate)
  if (principal !== null) {
    return <TemplateDetailView key={principal} principalLocationId={principal} onBack={() => openTemplate(null)} />
  }
  return <TemplateList onOpen={openTemplate} />
}

function TemplateList({ onOpen }: { onOpen: (principalLocationId: number) => void }) {
  const words = vocabulary()
  const templates = useDocbaseQuery("templates", listTemplates)
  const rows = templates.data ?? []
  return (
    <DocbasePage title={words.templates.name} subtitle={words.templates.description}>
      <div className="mx-auto flex max-w-5xl flex-col gap-4">
        {templates.error && <ErrorNote>{templates.error}</ErrorNote>}
        {templates.data && rows.length === 0 && (
          <EmptyNote>
            Todavía no hay {words.templates.many}: aparecen cuando la app encuentra dos o más documentos con el mismo texto y
            otros datos.
          </EmptyNote>
        )}
        {rows.length > 0 && <TemplateTable templates={rows} onOpen={onOpen} words={words} />}
      </div>
    </DocbasePage>
  )
}

/** The list of the templates, one row each, with the line that sums them up. */
export function TemplateTable({
  templates,
  onOpen,
  words = vocabulary(),
}: {
  templates: TemplateSummary[]
  onOpen: (principalLocationId: number) => void
  words?: Vocabulary
}) {
  return (
    <>
      <p className="text-sm text-muted-foreground">{templatesSummary(templates, words)}</p>
      <ul className="divide-y rounded-lg border bg-card">
        {templates.map((template) => (
          <li key={template.principal.contentId}>
            <button
              className="flex w-full flex-col gap-1 px-4 py-2.5 text-left hover:bg-muted/50"
              onClick={() => onOpen(template.principal.locationId)}
            >
              <span className="flex flex-wrap items-center gap-2">
                <span className="min-w-0 font-medium break-words">{templateName(template, template.principal.name, words)}</span>
                <TemplateStateBadge state={template.state} words={words} />
                {template.possiblySame > 0 && <StatusBadge tone="warning">posiblemente el mismo documento</StatusBadge>}
                {template.format === "scan" && <StatusBadge tone="warning">el texto salió de un escaneo</StatusBadge>}
              </span>
              <span className="text-xs text-muted-foreground">
                {[plural(template.documents, "documento", "documentos"), fieldsLine(template), formatName(template.format)].join(" · ")}
              </span>
            </button>
          </li>
        ))}
      </ul>
    </>
  )
}

/** One template: its name, its state, its notices and the panel of today with the text and the fields. */
function TemplateDetailView({ principalLocationId, onBack }: { principalLocationId: number; onBack: () => void }) {
  const words = vocabulary()
  const template = useDocbaseQuery(`similar-template:${principalLocationId}`, () => getSimilarTemplate(principalLocationId))
  const openDocument = useOpenDocument()
  const action = useAction()
  // What a person decides is saved and the screen reads it again.
  const decide = async (run: () => Promise<void>): Promise<boolean> => {
    let saved = false
    await action.run(async () => {
      await run()
      saved = true
      template.reload()
    })
    return saved
  }
  return (
    <DocbasePage
      title={words.templates.name}
      actions={
        <Button size="sm" variant="ghost" onClick={onBack}>
          <ArrowLeft /> Todas
        </Button>
      }
    >
      <div className="mx-auto flex max-w-6xl flex-col gap-4">
        {(template.error || action.problem) && <ErrorNote>{template.error ?? action.problem}</ErrorNote>}
        {!template.loading && !template.data && !template.error && (
          <EmptyNote>Esta {words.templates.one} ya no está: su familia de documentos cambió.</EmptyNote>
        )}
        {template.data && (
          <TemplateDetail
            template={template.data}
            onOpen={(id) => openDocument(id)}
            busy={action.busy}
            onRename={(name) => {
              const principal = template.data?.principal
              return principal === undefined ? Promise.resolve(false) : decide(() => nameTemplate(principal, name))
            }}
            onChooseField={(position, choice) => {
              const principal = template.data?.principal
              return principal === undefined ? Promise.resolve(false) : decide(() => chooseTemplateField(principal, position, choice))
            }}
            words={words}
          />
        )}
      </div>
    </DocbasePage>
  )
}

/** The detail of a template, apart from the base so it can be tested. */
export function TemplateDetail({
  template,
  onOpen,
  onRename,
  onChooseField,
  busy = false,
  words = vocabulary(),
}: {
  template: Template
  onOpen: (locationId: number) => void
  /** A person names it, or gives it back its automatic name (`null`); `false`: it was not saved. */
  onRename?: (name: string | null) => Promise<boolean>
  onChooseField?: (position: number, choice: FieldChoice) => Promise<boolean>
  busy?: boolean
  words?: Vocabulary
}) {
  const [renaming, setRenaming] = useState(false)
  const principal = template.documents[0]?.doc
  const fields = fieldRows(template).length
  const automatic = automaticTemplateName(template, principal?.name ?? "", words)
  return (
    <>
      <div className="flex flex-col gap-1">
        <div className="flex flex-wrap items-center gap-2">
          {renaming && onRename ? (
            <NameForm
              initial={template.name ?? ""}
              placeholder={automatic}
              saveLabel="Guardar"
              busy={busy}
              onCancel={() => setRenaming(false)}
              onSave={(name) => void onRename(name).then((saved) => saved && setRenaming(false))}
            />
          ) : (
            <>
              <h2 className="min-w-0 font-serif text-2xl break-words">{templateName(template, principal?.name ?? "", words)}</h2>
              {onRename && (
                <Button size="icon-xs" variant="ghost" title="Cambiar el nombre" onClick={() => setRenaming(true)}>
                  <Pencil />
                </Button>
              )}
            </>
          )}
          <TemplateStateBadge state={template.state} words={words} />
          {template.name && onRename && !renaming && (
            <Button size="xs" variant="ghost" disabled={busy} title={`El automático es «${automatic}»`} onClick={() => void onRename(null)}>
              Volver al nombre automático
            </Button>
          )}
        </div>
        <p className="text-sm text-muted-foreground">
          {[
            plural(template.documents.length, "documento", "documentos"),
            plural(fields, "campo", "campos"),
            `formato del principal: ${formatName(template.format)}`,
          ].join(" · ")}
        </p>
      </div>
      {template.state === "proposed" && (
        <div className="flex flex-wrap items-center justify-between gap-2 rounded-md border border-dashed px-3 py-2 text-sm">
          <span>
            Es una propuesta: nadie confirmó todavía que estos documentos son de la misma familia. Se puede ver y usar igual.
          </span>
          <Button size="sm" variant="outline" onClick={() => goToTarget({ view: "similar", principal: template.principal })}>
            Confirmar la familia
          </Button>
        </div>
      )}
      {template.format === "scan" && (
        <div className="rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm text-amber-800 dark:text-amber-300">
          El texto salió de un escaneo: lo leyó el reconocimiento y puede tener errores.
        </div>
      )}
      <TemplatePanel
        template={template}
        initialColumn={null}
        onOpen={onOpen}
        heading={false}
        onChooseField={onChooseField}
        busy={busy}
        words={words}
      />
    </>
  )
}
