import { Fragment, useMemo, type ReactNode } from "react"
import { FileText, Sparkles } from "lucide-react"
import { TextMark } from "@/components/ui/badge"
import { Callout } from "@/components/ui/callout"
import { MoreButton, Table, Td, Th, useFirstRows } from "@/components/ui/table"
import { WikiReader } from "@/components/editor/wiki-reader"
import { formatAgo, plural } from "@/components/docbase/format"
import { splitNotes } from "@/lib/article-notes"
import { entityTypeName, vocabulary } from "@/vocabulary"
import { usePageWords } from "./article-text"
import {
  certaintyWords,
  documentGroups,
  factGroups,
  initials,
  leadParts,
  parseArticle,
  partyRows,
  proposedRows,
  relatedCards,
  relationRows,
  saysParts,
  type ArticleParts,
  type ArticleSection,
  type CertaintyWords,
  type DocumentGroup,
  type PageWord,
} from "./article-parts"
import { TextLink, type IndexEntry } from "./page-aside"
import { Section } from "./shared"

/** The rows a long table shows before «Ver los otros…». */
const FIRST_ROWS = 10

const DATA_TAB = "Datos y correcciones"

export interface ArticleModel {
  parts: ArticleParts
  notes: Record<string, string>
  word: PageWord
  marks: CertaintyWords
  /** What the first line says after the type: «Se lo nombra como … · Expediente …». */
  facts: string
  /** The rest of what goes before the first section, as written. */
  lead: string[]
}

/**
 * An article read in its parts (`article-parts.ts`), with the words of the
 * pages; while those load, every section is shown as written.
 */
export function useArticle(body: string | null): ArticleModel | null {
  const pageWord = usePageWords()
  return useMemo(() => {
    if (body === null) return null
    const word: PageWord = pageWord ?? ((id) => id)
    const { body: text, notes } = splitNotes(body)
    const parts = parseArticle(text, word)
    const lead = leadParts(parts.lead)
    return { parts, notes, word, marks: certaintyWords(word), facts: lead.facts, lead: lead.rest }
  }, [body, pageWord])
}

/**
 * The heading a section shows. The screen says the matter as the
 * profession calls it («caso»): the text of the page keeps its own words,
 * which the chat and the search read (P-313).
 */
export function sectionTitle(section: ArticleSection): string {
  const matter = vocabulary().knowledge.matter.toLowerCase()
  if (section.kind === "facts") return `Lo que dicen los documentos del ${matter}`
  if (section.kind === "proposed") return `Posiblemente de este ${matter}`
  return section.heading
}

/** «En esta página»: the sections of the article, and what the screen adds after them. */
export function indexOf(model: ArticleModel | null, extra: IndexEntry[]): IndexEntry[] {
  const sections = model ? model.parts.sections.map((section) => ({ id: section.id, label: sectionTitle(section) })) : []
  return [...sections, ...extra]
}

/**
 * The article as a page of an encyclopedia (D-493): what the documents say
 * with its notes, the data in which they differ in a notice with the way
 * to resolve them, how to read the quotes in a notice of information, the
 * documents and the parties in tables, the links in a table and the
 * related ones in cards; any other section as written. `onShowData` opens
 * «Datos y correcciones».
 */
export function ArticleBody({
  model,
  withRoles,
  onShowData,
  writtenAt,
}: {
  model: ArticleModel
  /** Its documents say its roles in each (the page of a person or organization). */
  withRoles: boolean
  onShowData: () => void
  writtenAt?: number
}) {
  return (
    <div className="text-base leading-[1.7]">
      {model.lead.length > 0 && (
        <div className="mb-8">
          <Markdown model={model} body={model.lead.join("\n\n")} />
        </div>
      )}
      {model.parts.sections.map((section) => (
        <ArticleSectionView key={section.id} model={model} section={section} withRoles={withRoles} onShowData={onShowData} />
      ))}
      {writtenAt !== undefined && <WrittenBy at={writtenAt} />}
    </div>
  )
}

/** When the app wrote the article, and that a correction shows when it writes it again. */
export function WrittenBy({ at }: { at: number }) {
  return (
    <p className="mt-7 flex items-center gap-2 border-t border-dashed pt-3.5 text-[0.8125rem] text-muted-foreground">
      <Sparkles className="size-4 shrink-0" aria-hidden />
      Lo escribió la app {formatAgo(at)}, desde los documentos. Una corrección se ve acá cuando lo vuelve a escribir.
    </p>
  )
}

function Markdown({ model, body, compact = false }: { model: ArticleModel; body: string; compact?: boolean }) {
  return <WikiReader body={body} notes={model.notes} marks={model.marks} compact={compact} />
}

function Inline({ model, body }: { model: ArticleModel; body: string }) {
  return <WikiReader body={body} notes={model.notes} marks={model.marks} inline />
}

function ArticleSectionView({
  model,
  section,
  withRoles,
  onShowData,
}: {
  model: ArticleModel
  section: ArticleSection
  withRoles: boolean
  onShowData: () => void
}) {
  const content = sectionContent(model, section, withRoles, onShowData)
  return (
    <Section id={section.id} title={sectionTitle(section)} count={section.count}>
      {content ?? <Markdown model={model} body={section.body} />}
    </Section>
  )
}

/** The content of a section with a form of its own; null to show it as written. */
function sectionContent(model: ArticleModel, section: ArticleSection, withRoles: boolean, onShowData: () => void): ReactNode | null {
  switch (section.kind) {
    case "says":
      return <Says model={model} body={section.body} onShowData={onShowData} />
    case "documents": {
      const read = documentGroups(section.body, model.word, withRoles)
      return read ? <Documents model={model} groups={read.groups} extra={read.extra} withRoles={withRoles} onShowData={onShowData} /> : null
    }
    case "proposed": {
      const rows = proposedRows(section.body)
      return rows ? <Proposed model={model} rows={rows} onShowData={onShowData} /> : null
    }
    case "relations": {
      const rows = relationRows(section.body)
      return rows ? <Relations model={model} rows={rows} /> : null
    }
    case "related": {
      const cards = relatedCards(section.body)
      return cards ? <Related model={model} cards={cards} /> : null
    }
    case "parties": {
      const rows = partyRows(section.body)
      return rows ? <Parties model={model} rows={rows} /> : null
    }
    case "facts": {
      const groups = factGroups(section.body)
      return groups ? <Facts model={model} groups={groups} /> : null
    }
    default:
      return null
  }
}

/** A text with «Datos y correcciones» as the way to that tab. */
function withDataLink(text: string, onShowData: () => void): ReactNode {
  const pieces = text.split(`«${DATA_TAB}»`)
  return pieces.map((piece, index) => (
    <Fragment key={index}>
      {piece}
      {index < pieces.length - 1 && <TextLink onClick={onShowData}>{DATA_TAB}</TextLink>}
    </Fragment>
  ))
}

function Says({ model, body, onShowData }: { model: ArticleModel; body: string; onShowData: () => void }) {
  const says = saysParts(body, model.word)
  const doubtful = model.marks.doubtful
  return (
    <>
      {says.lede.map((text, index) => (
        <div key={index} className="mb-3.5 text-[1.0625rem] text-strong dark:text-[var(--d-700)]">
          <Markdown model={model} body={text} />
        </div>
      ))}
      {says.conflicts.length > 0 && (
        <Callout tone="warn" title={model.word("conflicts")} className="mb-4">
          <ul className="flex flex-col gap-1 text-[0.9375rem]">
            {says.conflicts.map((line, index) => (
              <li key={index}>
                {line.split(" · ").map((piece, at) => (
                  <Fragment key={at}>
                    {at > 0 && <span className="mx-2 font-bold text-warn">o</span>}
                    <Inline model={model} body={piece} />
                  </Fragment>
                ))}
              </li>
            ))}
          </ul>
          <p className="mt-1.5 text-[0.84375rem] text-muted-foreground">
            La app no elige. Si uno está mal, marcalo en <TextLink onClick={onShowData}>{DATA_TAB}</TextLink>.
          </p>
        </Callout>
      )}
      {says.legend && (
        <Callout tone="info" title={null} className="mb-4">
          <p className="text-[0.84375rem] leading-relaxed">
            {says.legend.split(`«${doubtful}»`).map((piece, index, all) => (
              <Fragment key={index}>
                {piece}
                {index < all.length - 1 && <TextMark tone="doubt">{doubtful}</TextMark>}
              </Fragment>
            ))}{" "}
            Pasá el mouse por un número para ver la cita.
          </p>
        </Callout>
      )}
      {says.more && <p className="text-[0.90625rem] text-muted-foreground">{withDataLink(says.more, onShowData)}</p>}
      {says.rest.map((text, index) => (
        <Markdown key={index} model={model} body={text} />
      ))}
    </>
  )
}

/** The documents of a page, by matter, in tables with their columns. */
function Documents({
  model,
  groups,
  extra,
  withRoles,
  onShowData,
}: {
  model: ArticleModel
  groups: DocumentGroup[]
  extra: string[]
  withRoles: boolean
  onShowData: () => void
}) {
  const words = vocabulary()
  const why = `*${model.word("matterWhy")}*`
  return (
    <>
      {groups.map((group, index) => (
        <div key={index} className="mb-5 last:mb-0">
          {group.heading && (
            <h3 className="mt-5 mb-2 flex flex-wrap items-baseline gap-x-2 font-ui text-base font-bold text-strong first:mt-0">
              {group.heading === model.word("withoutMatter") ? words.knowledge.withoutMatter : <Inline model={model} body={group.heading} />}
              <span className="font-sans text-[0.8125rem] font-normal text-muted-foreground">
                {plural(group.rows.length, "documento", "documentos")}
              </span>
            </h3>
          )}
          <DocumentTable model={model} group={group} withRoles={withRoles} head={index === 0} />
        </div>
      ))}
      {extra.map((text, index) =>
        text.startsWith(why) ? (
          <div key={index} className="mt-4 rounded-[10px] border bg-panel-sub px-3.5 py-3 text-[0.90625rem]">
            <b className="font-ui font-bold text-strong">{model.word("matterWhy")}</b>
            {text.slice(why.length)}. El detalle de cada uno está en <TextLink onClick={onShowData}>{DATA_TAB}</TextLink>.
          </div>
        ) : (
          <Markdown key={index} model={model} body={text} />
        ),
      )}
    </>
  )
}

function DocumentTable({ model, group, withRoles, head }: { model: ArticleModel; group: DocumentGroup; withRoles: boolean; head: boolean }) {
  const { shown, hidden, showAll } = useFirstRows(group.rows, FIRST_ROWS)
  const proposedWord = model.word("proposed")
  return (
    <>
      <Table fixed>
        <colgroup>
          <col />
          {withRoles && <col style={{ width: "24%" }} />}
          <col style={{ width: "22%" }} />
        </colgroup>
        {head && (
          <thead>
            <tr>
              <Th>Documento</Th>
              {withRoles && <Th>Su papel</Th>}
              <Th>Tipo</Th>
            </tr>
          </thead>
        )}
        <tbody>
          {shown.map((row, index) => (
            <tr key={index}>
              <Td className="[&_a]:no-underline [&_a:hover]:underline">
                <Inline model={model} body={row.document} />
                {row.proposed && (
                  <TextMark tone="doubt" className="ml-1.5">
                    {proposedWord}
                  </TextMark>
                )}
              </Td>
              {withRoles && <Td muted>{row.roles}</Td>}
              <Td muted>{row.type ?? "—"}</Td>
            </tr>
          ))}
        </tbody>
      </Table>
      {hidden > 0 && (
        <MoreButton className="mt-2 ml-1" onClick={showAll}>
          Ver los otros {plural(hidden, "documento", "documentos")}
        </MoreButton>
      )}
    </>
  )
}

/** The documents a model proposes for a matter: dashed, «sin confirmar», with what the model said. */
function Proposed({
  model,
  rows,
  onShowData,
}: {
  model: ArticleModel
  rows: { document: string; reason: string | null }[]
  onShowData: () => void
}) {
  return (
    <div className="flex flex-col gap-2.5">
      {rows.map((row, index) => (
        <div key={index} className="rounded-[10px] border-[1.5px] border-dashed border-border-strong px-3.5 py-3">
          <div className="flex flex-wrap items-center gap-2 font-ui text-sm font-bold text-strong">
            <TextMark tone="doubt" className="mx-0">
              {model.marks.doubtful}
            </TextMark>
            <Inline model={model} body={row.document} />
          </div>
          {row.reason && <p className="mt-1.5 text-[0.90625rem]">El modelo: «{row.reason}»</p>}
          <p className="mt-1 text-[0.8125rem] text-muted-foreground">
            Una persona lo confirma o lo descarta en <TextLink onClick={onShowData}>{DATA_TAB}</TextLink>.
          </p>
        </div>
      ))}
    </div>
  )
}

function Relations({ model, rows }: { model: ArticleModel; rows: { label: string; value: string }[] }) {
  return (
    <Table>
      <tbody>
        {rows.map((row, index) => (
          <tr key={index}>
            {row.label ? (
              <>
                <Td muted width="34%">
                  {row.label}
                </Td>
                <Td className="[&_a]:no-underline [&_a:hover]:underline">
                  <Inline model={model} body={row.value} />
                </Td>
              </>
            ) : (
              <Td colSpan={2}>
                <Inline model={model} body={row.value} />
              </Td>
            )}
          </tr>
        ))}
      </tbody>
    </Table>
  )
}

function Related({ model, cards }: { model: ArticleModel; cards: { link: string; name: string; type: string; shared: string | null }[] }) {
  const person = entityTypeName("persona", false).toLowerCase()
  return (
    <div className="grid gap-2 sm:grid-cols-2">
      {cards.map((card) => {
        const isPerson = card.type.toLowerCase() === person
        return (
          <div key={card.link} className="flex items-center gap-2.5 rounded-[10px] border bg-card px-3 py-[9px] text-sm leading-[1.35] hover:border-border-strong">
            <span
              aria-hidden
              className={`inline-flex size-[30px] shrink-0 items-center justify-center font-ui text-xs font-bold ${
                isPerson ? "rounded-full bg-brand-soft text-brand-ink" : "rounded-lg bg-muted text-muted-foreground"
              }`}
            >
              {initials(card.name)}
            </span>
            <span className="min-w-0">
              <span className="block truncate font-semibold [&_a]:no-underline [&_a:hover]:underline">
                <Inline model={model} body={card.link} />
              </span>
              <span className="block text-[0.78125rem] text-muted-foreground">
                {card.type}
                {card.shared && ` · ${card.shared}`}
              </span>
            </span>
          </div>
        )
      })}
    </div>
  )
}

function Parties({ model, rows }: { model: ArticleModel; rows: { who: string; type: string; roles: string; documents: number }[] }) {
  const { shown, hidden, showAll } = useFirstRows(rows, FIRST_ROWS)
  return (
    <>
      <Table>
        <thead>
          <tr>
            <Th>Quién</Th>
            <Th>Tipo</Th>
            <Th>Su papel en los documentos</Th>
            <Th align="right">Documentos</Th>
          </tr>
        </thead>
        <tbody>
          {shown.map((row, index) => (
            <tr key={index}>
              <Td className="[&_a]:no-underline [&_a:hover]:underline">
                <Inline model={model} body={row.who} />
              </Td>
              <Td muted nowrap>
                {row.type}
              </Td>
              <Td muted>{row.roles}</Td>
              <Td align="right">{row.documents}</Td>
            </tr>
          ))}
        </tbody>
      </Table>
      {hidden > 0 && (
        <MoreButton className="mt-2 ml-1" onClick={showAll}>
          Ver los otros {hidden}
        </MoreButton>
      )}
    </>
  )
}

/** What the documents of a matter say, by whom: a row for each person, and its data below. */
function Facts({ model, groups }: { model: ArticleModel; groups: { subject: string; rows: { label: string; value: string }[] }[] }) {
  const differ = ` (${model.word("conflicts").toLowerCase()})`
  return (
    <Table>
      <tbody>
        {groups.map((group, index) => (
          <Fragment key={index}>
            <tr>
              <Td colSpan={2} className="bg-panel-sub py-[7px] font-ui font-bold text-strong [&_a]:no-underline [&_a:hover]:underline">
                <Inline model={model} body={group.subject} />
              </Td>
            </tr>
            {group.rows.map((row, at) => {
              const differs = row.value.includes(differ)
              return (
                <tr key={at}>
                  <Td muted width="34%">
                    {row.label}
                  </Td>
                  <Td className="[&_a]:no-underline [&_a:hover]:underline">
                    <Inline model={model} body={row.value.replace(differ, "")} />
                    {differs && <TextMark tone="differ">difiere</TextMark>}
                  </Td>
                </tr>
              )
            })}
          </Fragment>
        ))}
      </tbody>
    </Table>
  )
}

/** A document's line in a list: its icon and its name. */
export function DocumentIcon() {
  return <FileText className="size-4 shrink-0 self-center text-muted-foreground" aria-hidden />
}

