import { useMemo } from "react"
import ReactMarkdown from "react-markdown"
import remarkGfm from "remark-gfm"
import { remarkMathPlugin } from "@/lib/markdown-math"
import { remarkFoldable } from "@/lib/markdown-fold"
import rehypeKatex from "rehype-katex"
import "katex/dist/katex.min.css"
import { transformImageEmbeds, transformWikilinks, hideHtmlComments } from "@/lib/wikilink-transform"
import { resolveMarkdownImageSrc } from "@/lib/markdown-image-resolver"
import { normalizePath } from "@/lib/path-utils"
import { detectLanguage } from "@/lib/detect-language"
import { getHtmlLang, getTextDirection } from "@/lib/language-metadata"
import { openWikiPath } from "@/lib/open-wiki-path"
import { useWikiStore } from "@/stores/wiki-store"
import { MermaidDiagram, unwrapMermaidPre } from "@/components/mermaid-diagram"
import { noteLabel } from "@/lib/article-notes"
import { TextMark } from "@/components/ui/badge"
import { certaintyMark, type CertaintyWords } from "@/components/knowledge/article-parts"
import { MarksContext, plainText } from "./marks"
import { NoteMark } from "./note-mark"
import { PageLink } from "./page-link"

interface WikiReaderProps {
  body: string
  /** Original, untransformed Markdown body used for DOM-to-source mapping. */
  sourceBody?: string
  /** Character offset where sourceBody begins in the full Markdown file. */
  sourceOffset?: number
  /**
   * Absolute path of the markdown file being rendered. Used to
   * resolve relative image references against the file's own
   * directory (Obsidian-style), so e.g. `../assets/x.png` works.
   * Optional — when omitted, image paths fall back to wiki-root
   * resolution.
   */
  filePath?: string
  /**
   * The notes of an article (`splitNotes`, D-441): each link to one of
   * them is shown as its number, which shows the note in a card when
   * touched, hovered or reached with the keyboard (P-246).
   */
  notes?: Record<string, string>
  /**
   * The words of how sure a page is of a datum: with them, an emphasis
   * like `*(sin confirmar)*` shows as the amber mark «sin confirmar», with
   * why in its title, and a person's word as a mark of the brand.
   */
  marks?: CertaintyWords
  /** Inside a line (a cell of a table, a notice): without paragraphs. */
  inline?: boolean
  /** Smaller letter, for a panel or a card: 14 px instead of the 16 px of reading. */
  compact?: boolean
}

const LINK_CLASS = "cursor-pointer text-link underline decoration-[var(--link-line)] underline-offset-[3px] hover:decoration-link"

/**
 * Read-only render of a wiki page body. Distinct from WikiEditor
 * (Milkdown WYSIWYG) because Milkdown round-trips the markdown
 * through prosemirror — applying our wikilink → markdown-link
 * pre-processing there would mean the user's saves overwrite the
 * original `[[…]]` source with `[label](#slug)`. Here, since we
 * never serialize back to disk, transforming for display is safe.
 *
 * Wikilink anchor clicks are intercepted: `#slug` opens the screen
 * of what the link names (`openWikiPath`), giving the user
 * single-click navigation between pages.
 */
export function WikiReader({ body, sourceBody, sourceOffset = 0, filePath, notes, marks, inline = false, compact = false }: WikiReaderProps) {
  // Local images load only from the agent's folder, where the chat's outputs are.
  const imageRoot = useWikiStore((s) => s.agentWorkspace)
  // Image embeds (`![[…]]`) must be rewritten BEFORE the generic
  // wikilink pass, otherwise the embed target gets mangled into a
  // `#fragment` link.
  const transformed = useMemo(
    () => transformWikilinks(transformImageEmbeds(hideHtmlComments(body))),
    [body],
  )
  const sourceLineStarts = useMemo(() => {
    if (sourceBody === undefined) return null
    const starts = [0]
    for (let index = 0; index < sourceBody.length; index += 1) {
      if (sourceBody.charCodeAt(index) === 10) starts.push(index + 1)
    }
    return starts
  }, [sourceBody])

  const sourceAttrs = (node: unknown): Record<string, number> => {
    if (!sourceLineStarts) return {}
    const position = (node as { position?: { start?: { line?: number }; end?: { line?: number } } } | undefined)?.position
    const startLine = position?.start?.line
    const endLine = position?.end?.line
    if (!startLine || !endLine) return {}
    const start = sourceLineStarts[startLine - 1]
    const end = sourceLineStarts[endLine] ?? sourceBody?.length
    if (start === undefined || end === undefined) return {}
    return { "data-source-start": sourceOffset + start, "data-source-end": sourceOffset + end }
  }
  const renderLanguage = detectLanguage(body)
  const direction = getTextDirection(renderLanguage)
  const htmlLang = getHtmlLang(renderLanguage)
  // Directory of the file being rendered (project-absolute), so
  // relative image srcs resolve against it like Obsidian does.
  const currentFileDir = useMemo(() => {
    if (!filePath) return null
    const norm = normalizePath(filePath)
    const dir = norm.slice(0, norm.lastIndexOf("/"))
    return dir || null
  }, [filePath])

  // A link between pages (`[[name|text]]`) opens the screen of what it
  // names, read from the base; never a file (D-407).
  function handleAnchorClick(e: React.MouseEvent<HTMLAnchorElement>, href: string) {
    if (!href.startsWith("#")) return
    e.preventDefault()
    const slug = (() => {
      try {
        return decodeURIComponent(href.slice(1))
      } catch {
        return href.slice(1)
      }
    })()
    void openWikiPath(slug)
  }

  const markdown = (
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkMathPlugin, remarkFoldable]}
        rehypePlugins={[rehypeKatex]}
        components={{
          p: ({ node, children, ...props }) =>
            inline ? <>{children}</> : <p {...sourceAttrs(node)} {...props}>{children}</p>,
          em: ({ children }) => {
            const said = marks ? plainText(children) : null
            const mark = said !== null && marks ? certaintyMark(said, marks) : null
            if (!mark) return <em>{children}</em>
            return (
              <TextMark tone={mark.tone} title={mark.why ?? undefined}>
                {mark.label}
              </TextMark>
            )
          },
          li: ({ node, children, ...props }) => <li {...sourceAttrs(node)} {...props}>{children}</li>,
          blockquote: ({ node, children, ...props }) => <blockquote {...sourceAttrs(node)} {...props}>{children}</blockquote>,
          a: ({ href, children, ...props }) => {
            const h = typeof href === "string" ? href : ""
            const label = notes ? noteLabel(h) : null
            if (notes && label !== null && label in notes) return <NoteMark label={label} note={notes[label]} />
            // A link between pages shows a card of what it names (P-246);
            // not the marks of markdown's own footnotes.
            if (h.length > 1 && h.startsWith("#") && !h.startsWith("#user-content-")) return <PageLink href={h}>{children}</PageLink>
            const isWikilink = h.startsWith("#")
            return (
              <a
                href={h || undefined}
                onClick={(e) => isWikilink && handleAnchorClick(e, h)}
                className={isWikilink ? LINK_CLASS : "text-link underline underline-offset-[3px]"}
                {...props}
              >
                {children}
              </a>
            )
          },
          h1: ({ node, children, ...props }) => (
            <h1
              {...sourceAttrs(node)}
              className="mt-0 mb-4 border-b pb-3 font-ui text-[1.75rem] leading-tight font-bold tracking-[-0.01em] text-strong"
              {...props}
            >
              {children}
            </h1>
          ),
          h2: ({ node, children, ...props }) => (
            <h2
              {...sourceAttrs(node)}
              className={`mt-10 mb-3.5 border-b pb-2 font-ui leading-tight font-bold text-strong first:mt-0 ${compact ? "text-base" : "text-[1.3125rem]"}`}
              {...props}
            >
              {children}
            </h2>
          ),
          h3: ({ node, children, ...props }) => (
            <h3
              {...sourceAttrs(node)}
              className={`mt-6 mb-2 font-ui leading-snug font-bold text-strong ${compact ? "text-sm" : "text-base"}`}
              {...props}
            >
              {children}
            </h3>
          ),
          img: ({ src, alt, ...props }) => (
            <img
              src={
                typeof src === "string"
                  ? resolveMarkdownImageSrc(src, imageRoot, currentFileDir)
                  : undefined
              }
              data-mdsrc={typeof src === "string" ? src : undefined}
              alt={alt ?? ""}
              className="max-w-full rounded-lg border"
              loading="lazy"
              {...props}
            />
          ),
          table: ({ children, ...props }) => (
            <div className="my-3 overflow-x-auto rounded-[10px] border">
              <table className="w-full border-separate border-spacing-0 text-sm leading-[1.45] [&_tr:last-child>td]:border-b-0" {...props}>
                {children}
              </table>
            </div>
          ),
          thead: ({ children, ...props }) => (
            <thead className="bg-panel-sub" {...props}>
              {children}
            </thead>
          ),
          th: ({ node, children, ...props }) => (
            <th
              {...sourceAttrs(node)}
              className="border-b px-3 py-2 text-start font-ui text-xs font-bold tracking-[0.04em] whitespace-nowrap text-muted-foreground uppercase"
              {...props}
            >
              {children}
            </th>
          ),
          td: ({ node, children, ...props }) => (
            <td {...sourceAttrs(node)} className="border-b px-3 py-2 align-top" {...props}>
              {children}
            </td>
          ),
          pre: ({ children, ...props }) => {
            const mermaid = unwrapMermaidPre(children)
            if (mermaid) return <>{mermaid}</>
            return <pre dir="ltr" style={{ textAlign: "left" }} {...props}>{children}</pre>
          },
          code: ({ className, children, ...props }) => {
            const lang = className?.replace("language-", "")
            const codeText = String(children).replace(/\n$/, "")
            if (lang === "mermaid") return <MermaidDiagram code={codeText} />
            return <code dir="ltr" className={className} {...props}>{children}</code>
          },
        }}
      >
        {transformed}
      </ReactMarkdown>
  )
  if (inline) {
    return (
      <MarksContext.Provider value={marks}>
        <span className="min-w-0" dir={direction} lang={htmlLang}>
          {markdown}
        </span>
      </MarksContext.Provider>
    )
  }
  return (
    <MarksContext.Provider value={marks}>
      <div className={`reading min-w-0 ${compact ? "reading-compact" : ""}`} dir={direction} lang={htmlLang} style={{ textAlign: "start" }}>
        {markdown}
      </div>
    </MarksContext.Provider>
  )
}
