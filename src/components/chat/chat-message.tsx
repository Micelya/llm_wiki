import { memo, useCallback, useEffect, useRef, useState, useMemo } from "react"
import { useTranslation } from "react-i18next"
import { convertFileSrc } from "@tauri-apps/api/core"
import ReactMarkdown from "react-markdown"
import remarkGfm from "remark-gfm"
import { remarkMathPlugin } from "@/lib/markdown-math"
import rehypeKatex from "rehype-katex"
import "katex/dist/katex.min.css"
import {
  Bot, User, FileText, BookmarkPlus, ChevronDown, ChevronRight, RefreshCw, Copy, Check,
  Users, Lightbulb, BookOpen, HelpCircle, GitMerge, BarChart3, Layout, Globe,
  TrendingUp, Target, Sparkles, Image as ImageIcon, FileSearch, Terminal,
  ListTree,
} from "lucide-react"
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener"
import { getPageText, objectOfPage, saveQuery, type PageObject } from "@/commands/docbase"
import { openChatDocument } from "@/lib/chat-documents"
import { openPageObject, openWikiPath } from "@/lib/open-wiki-path"
import { useChatStore } from "@/stores/chat-store"
import { useWikiStore } from "@/stores/wiki-store"
import { readFile } from "@/commands/fs"
import { lastQueryPages } from "@/components/chat/chat-panel"
import type { DisplayMessage, MessageReference } from "@/stores/chat-store"

import { convertLatexToUnicode } from "@/lib/latex-to-unicode"
import { normalizePath, getFileName, isAbsolutePath } from "@/lib/path-utils"
import { messageImageToDataUrl } from "@/lib/chat-image-utils"
import { resolveMarkdownImageSrc } from "@/lib/markdown-image-resolver"
import { transformImageEmbeds } from "@/lib/wikilink-transform"
import { detectLanguage } from "@/lib/detect-language"
import { getHtmlLang, getTextDirection } from "@/lib/language-metadata"
import { MermaidDiagram, unwrapMermaidPre } from "@/components/mermaid-diagram"
import { inferWikiTypeFromPath } from "@/lib/wiki-page-types"
import { cleanAssistantContentForSaving, titleFromCleanAssistantContent } from "@/lib/saved-query-text"
import type { ChatAgentEvent, ChatAgentEventStage, ChatAgentStep, ChatUserInputField, ChatUserInputRequest } from "@/lib/chat-agent-types"
import { getFileCategory, getFileExtension, isTextReadable } from "@/lib/file-types"
import { AgentFileActivity } from "@/components/chat/agent-file-activity"
import { ReferenceKnowledgeGraph } from "@/components/chat/reference-knowledge-graph"
import { OperationCardView } from "@/components/operations/operation-card"


interface ChatMessageProps {
  message: DisplayMessage
  isLastAssistant?: boolean
  onRegenerate?: () => void
  onOpenReferencePreview?: (preview: ChatReferencePreview, relatedPreviews?: ChatReferencePreview[]) => void
  onOpenContextDetails?: (references: MessageReference[]) => void
  onApproveShellCommand?: (command: string, assistantMessageId: string) => void
  onSubmitUserInput?: (request: ChatUserInputRequest, answers: Record<string, unknown>) => boolean
  /** «Seguir buscando»: the agent goes on from where it reached its cap of turns. */
  onContinue?: (message: DisplayMessage) => void
}

export interface ChatReferencePreview {
  title: string
  path: string
  content: string
  source?: string
  external?: boolean
  snippet?: string
}

function ChatMessageImpl({
  message,
  isLastAssistant,
  onRegenerate,
  onOpenReferencePreview,
  onOpenContextDetails,
  onApproveShellCommand,
  onSubmitUserInput,
  onContinue,
}: ChatMessageProps) {
  const { t } = useTranslation()
  const isUser = message.role === "user"
  const isSystem = message.role === "system"
  const isAssistant = message.role === "assistant"
  const [hovered, setHovered] = useState(false)

  return (
    <div
      className={`flex gap-2 ${isUser ? "flex-row-reverse" : "flex-row"}`}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <div
        className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-full ${
          isSystem
            ? "bg-accent text-accent-foreground"
            : isUser
              ? "bg-brand-soft text-brand-ink"
              : "bg-muted text-muted-foreground"
        }`}
      >
        {isUser ? <User className="h-4 w-4" /> : <Bot className="h-4 w-4" />}
      </div>
      <div className="max-w-[80%] flex flex-col gap-1.5">
        {isUser && message.contextDocuments && message.contextDocuments.length > 0 && (
          <div className="flex flex-wrap justify-end gap-1">
            {message.contextDocuments.map((document) => (
              <button
                key={document.locationId}
                type="button"
                onClick={() => openChatDocument(document.locationId)}
                className="inline-flex h-7 max-w-[18rem] items-center gap-1.5 rounded-md border border-transparent bg-brand-soft px-2 text-xs font-medium text-brand-ink hover:bg-hover"
                title={`Abrir «${document.name}» · ${document.place}`}
              >
                <FileText className="h-3 w-3 shrink-0" />
                <span className="truncate">{document.name}</span>
              </button>
            ))}
          </div>
        )}
        {isUser && message.contextFiles && message.contextFiles.length > 0 && (
          <div className="flex flex-wrap justify-end gap-1">
            {message.contextFiles.map((path) => (
              <span
                key={path}
                className="inline-flex h-7 max-w-[18rem] items-center gap-1.5 rounded-md border border-transparent bg-brand-soft px-2 text-xs font-medium text-brand-ink"
                title={path}
              >
                <FileText className="h-3 w-3 shrink-0" />
                <span className="truncate">@{getFileName(path)}</span>
              </span>
            ))}
          </div>
        )}
        {isUser && message.images && message.images.length > 0 && (
          <div className={`flex flex-wrap gap-1.5 ${isUser ? "justify-end" : ""}`}>
            {message.images.map((img, i) => (
              <img
                key={i}
                src={messageImageToDataUrl(img)}
                alt=""
                className="max-h-40 max-w-[180px] rounded-lg border border-border/40 object-contain"
                loading="lazy"
              />
            ))}
          </div>
        )}
        {isAssistant && (
          (message.agentSteps?.some((step) => step.type !== "final") ?? false)
          || (message.agentFileChanges?.length ?? 0) > 0
        ) && (
          <AgentTurnActivity
            steps={message.agentSteps ?? []}
            changes={message.agentFileChanges ?? []}
            canApproveShellCommand={Boolean(isLastAssistant && onApproveShellCommand)}
            onApproveShellCommand={(command) => onApproveShellCommand?.(command, message.id)}
          />
        )}
        {(!isUser || message.content) && (
          <div
            className={`rounded-xl px-3.5 py-2.5 text-sm ${
              isUser
                ? "bg-brand-soft text-strong"
                : "border bg-card text-foreground shadow-card"
            }`}
          >
            {isUser ? (
              <p dir="auto" className="whitespace-pre-wrap break-words">{message.content}</p>
            ) : (
              <MarkdownContent content={message.content} />
            )}
          </div>
        )}
        {isAssistant && (
          <CitedReferencesPanel
            content={message.content}
            savedReferences={message.references}
            onOpenReferencePreview={onOpenReferencePreview}
            onOpenContextDetails={onOpenContextDetails}
          />
        )}
        {isAssistant && message.userInputRequest && (
          <UserInputRequestPanel
            request={message.userInputRequest}
            onSubmit={onSubmitUserInput}
          />
        )}
        {isAssistant && message.operationCard && (
          <OperationCardView
            card={message.operationCard}
            onChange={(card) => useChatStore.getState().setMessageOperationCard(message.id, card)}
          />
        )}
        {isAssistant && message.budget?.reached && (
          <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground" data-testid="turn-cap">
            <span>Llegué al tope de {message.budget.max} vueltas.</span>
            {onContinue && message.budget.continuation && (
              <button
                type="button"
                onClick={() => onContinue(message)}
                className="rounded border px-2 py-0.5 text-foreground hover:bg-brand-soft hover:text-link"
                title="Suma otras tantas vueltas y sigue desde donde quedó. Cada vuelta es una pregunta más al modelo."
              >
                Seguir buscando
              </button>
            )}
          </div>
        )}
        {isAssistant && hovered && (
          <div className="flex items-center gap-1">
            <CopyButton content={message.content} />
            <SaveQueryButton message={message} />
            {isLastAssistant && onRegenerate && (
              <button
                type="button"
                onClick={onRegenerate}
                className="inline-flex items-center gap-1 rounded px-2 py-0.5 text-[11px] text-muted-foreground hover:text-link hover:bg-brand-soft transition-colors"
                title={t("chat.regenerateResponse")}
              >
                <RefreshCw className="h-3 w-3" /> {t("chat.regenerate")}
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  )
}

function AgentTurnActivity({
  steps,
  changes,
  canApproveShellCommand,
  onApproveShellCommand,
}: {
  steps: ChatAgentStep[]
  changes: NonNullable<DisplayMessage["agentFileChanges"]>
  canApproveShellCommand?: boolean
  onApproveShellCommand?: (command: string) => void
}) {
  const { t } = useTranslation()
  return (
    <section className="rounded-md border border-border/50 bg-background/50" aria-label={t("chat.agentChanges.taskTitle")}>
      <div className="flex items-center justify-between border-b border-border/50 px-2.5 py-1.5">
        <span className="text-xs font-medium">{t("chat.agentChanges.taskTitle")}</span>
        {changes.length > 0 && (
          <span className="text-[10px] text-muted-foreground">
            {t("chat.agentChanges.fileCount", { count: new Set(changes.map((change) => change.path)).size })}
          </span>
        )}
      </div>
      <SavedAgentActivity
        steps={steps}
        canApproveShellCommand={canApproveShellCommand}
        onApproveShellCommand={onApproveShellCommand}
        embedded
      />
      {changes.length > 0 && <AgentFileActivity changes={changes} embedded />}
    </section>
  )
}

function SavedAgentActivity({
  steps,
  canApproveShellCommand,
  onApproveShellCommand,
  embedded = false,
}: {
  steps: ChatAgentStep[]
  canApproveShellCommand?: boolean
  onApproveShellCommand?: (command: string) => void
  embedded?: boolean
}) {
  const { t } = useTranslation()
  const events = useMemo<ChatAgentEvent[]>(() => steps
    .filter((step) => step.type !== "final")
    .map((step) => ({
      stage: step.type === "understanding"
        ? "understanding"
        : step.type === "routing"
          ? "routing"
          : step.type === "tool_call"
            ? "tool_call"
            : "tool_result",
      tool: step.tool,
      query: step.query,
      message: step.message,
      count: step.count,
      status: step.status,
      timestamp: step.timestamp,
    })), [steps])
  const shellCommand = useMemo(() => extractShellApprovalCommand(steps), [steps])
  if (events.length === 0 && !shellCommand) return null
  return (
    <div className={embedded ? "space-y-1 border-b border-border/40 px-2 py-1.5" : "space-y-1 rounded-md border border-border/50 bg-background/50 px-2 py-1"}>
      {events.length > 0 && <AgentActivity events={events} compact />}
      {shellCommand && canApproveShellCommand && (
        <button
          type="button"
          onClick={() => onApproveShellCommand?.(shellCommand)}
          className="flex w-full max-w-full items-start gap-1.5 rounded border border-warn-line bg-warn-bg px-2 py-1 text-left text-[11px] text-warn transition-colors hover:border-warn"
          title={shellCommand}
        >
          <Terminal className="h-3 w-3 shrink-0" />
          <span className="shrink-0">{t("chat.approveCommand")}</span>
          <code className="whitespace-pre-wrap break-all font-mono text-[10px] text-foreground dark:text-foreground">
            {shellCommand}
          </code>
        </button>
      )}
    </div>
  )
}

function extractShellApprovalCommand(steps: ChatAgentStep[]): string | null {
  for (const step of steps) {
    if (step.tool !== "shell_exec" || step.status !== "skipped") continue
    const message = step.message?.trim() ?? ""
    const command = message.startsWith("approval required:")
      ? message.slice("approval required:".length).trim()
      : ""
    if (command) return command
  }
  return null
}

export const ChatMessage = memo(ChatMessageImpl, (prev, next) =>
  prev.message === next.message
  && prev.isLastAssistant === next.isLastAssistant
  && prev.onRegenerate === next.onRegenerate
  && prev.onOpenReferencePreview === next.onOpenReferencePreview
  && prev.onApproveShellCommand === next.onApproveShellCommand
  && prev.onSubmitUserInput === next.onSubmitUserInput
  && prev.onContinue === next.onContinue
)

function UserInputRequestPanel({
  request,
  onSubmit,
}: {
  request: ChatUserInputRequest
  onSubmit?: (request: ChatUserInputRequest, answers: Record<string, unknown>) => boolean
}) {
  const { t } = useTranslation()
  const [answers, setAnswers] = useState<Record<string, unknown>>(() => initialUserInputAnswers(request))
  const [submitted, setSubmitted] = useState(false)
  const canSubmit = Boolean(onSubmit) && !submitted

  const update = useCallback((id: string, value: unknown) => {
    setAnswers((prev) => ({ ...prev, [id]: value }))
  }, [])

  return (
    <div className="rounded-lg border border-primary/20 bg-background px-3 py-3 shadow-sm">
      <div className="mb-3">
        <div className="text-sm font-medium text-foreground">{request.title}</div>
        {request.description && (
          <p className="mt-1 text-xs text-muted-foreground">{request.description}</p>
        )}
      </div>
      <div className="space-y-3">
        {request.fields.map((field) => (
          <UserInputFieldControl
            key={field.id}
            field={field}
            value={answers[field.id]}
            disabled={!canSubmit}
            onChange={(value) => update(field.id, value)}
          />
        ))}
      </div>
      <div className="mt-3 flex justify-end">
        <button
          type="button"
          disabled={!canSubmit}
          onClick={() => {
            if (!canSubmit) return
            if (onSubmit?.(request, answers)) {
              setSubmitted(true)
            }
          }}
          className="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60"
        >
          <Check className="h-3.5 w-3.5" />
          {submitted ? t("chat.userInputSubmitted") : t("chat.userInputSubmit")}
        </button>
      </div>
    </div>
  )
}

function UserInputFieldControl({
  field,
  value,
  disabled,
  onChange,
}: {
  field: ChatUserInputField
  value: unknown
  disabled?: boolean
  onChange: (value: unknown) => void
}) {
  const { t } = useTranslation()
  return (
    <div className="space-y-1.5">
      <div>
        <label className="text-xs font-medium text-foreground">{field.label}</label>
        {field.description && (
          <p className="mt-0.5 text-[11px] text-muted-foreground">{field.description}</p>
        )}
      </div>
      {field.type === "single" && (
        <div className="grid gap-1.5">
          {(field.options ?? []).map((option) => (
            <button
              key={option.value}
              type="button"
              disabled={disabled}
              onClick={() => onChange(option.value)}
              className={`rounded-md border px-2.5 py-2 text-left text-xs transition-colors ${
                value === option.value
                  ? "border-primary bg-primary/10 text-foreground"
                  : "border-border bg-muted/30 text-muted-foreground hover:bg-muted"
              } disabled:cursor-not-allowed disabled:opacity-70`}
            >
              <span className="flex items-center justify-between gap-2">
                <span className="font-medium">{option.label}</span>
                {option.recommended && (
                  <span className="rounded bg-brand-soft px-1.5 py-0.5 text-[10px] text-brand-ink">
                    {t("chat.userInputRecommended")}
                  </span>
                )}
              </span>
              {option.description && (
                <span className="mt-0.5 block text-[11px] opacity-80">{option.description}</span>
              )}
            </button>
          ))}
        </div>
      )}
      {field.type === "multi" && (
        <div className="grid gap-1.5">
          {(field.options ?? []).map((option) => {
            const selected = Array.isArray(value) && value.includes(option.value)
            return (
              <label
                key={option.value}
                className={`flex cursor-pointer items-start gap-2 rounded-md border px-2.5 py-2 text-xs transition-colors ${
                  selected ? "border-primary bg-primary/10" : "border-border bg-muted/30 hover:bg-muted"
                } ${disabled ? "cursor-not-allowed opacity-70" : ""}`}
              >
                <input
                  type="checkbox"
                  disabled={disabled}
                  checked={selected}
                  onChange={(event) => {
                    const current = Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : []
                    onChange(event.target.checked
                      ? [...current, option.value]
                      : current.filter((item) => item !== option.value))
                  }}
                  className="mt-0.5"
                />
                <span>
                  <span className="font-medium text-foreground">{option.label}</span>
                  {option.description && (
                    <span className="mt-0.5 block text-[11px] text-muted-foreground">{option.description}</span>
                  )}
                </span>
              </label>
            )
          })}
        </div>
      )}
      {field.type === "text" && (
        <input
          disabled={disabled}
          value={typeof value === "string" ? value : ""}
          placeholder={field.placeholder}
          onChange={(event) => onChange(event.target.value)}
          className="w-full rounded-md border border-border bg-background px-2.5 py-1.5 text-sm outline-none focus:border-primary disabled:opacity-70"
        />
      )}
      {field.type === "textarea" && (
        <textarea
          disabled={disabled}
          value={typeof value === "string" ? value : ""}
          placeholder={field.placeholder}
          onChange={(event) => onChange(event.target.value)}
          rows={4}
          className="w-full resize-y rounded-md border border-border bg-background px-2.5 py-1.5 text-sm outline-none focus:border-primary disabled:opacity-70"
        />
      )}
      {field.type === "confirm" && (
        <label className={`flex items-center gap-2 text-sm ${disabled ? "opacity-70" : ""}`}>
          <input
            type="checkbox"
            disabled={disabled}
            checked={Boolean(value)}
            onChange={(event) => onChange(event.target.checked)}
          />
          <span>{field.placeholder ?? t("chat.userInputEnabled")}</span>
        </label>
      )}
    </div>
  )
}

function initialUserInputAnswers(request: ChatUserInputRequest): Record<string, unknown> {
  const answers: Record<string, unknown> = {}
  for (const field of request.fields) {
    if (field.defaultValue !== undefined) {
      answers[field.id] = field.defaultValue
      continue
    }
    if (field.type === "single") {
      answers[field.id] = field.options?.find((option) => option.recommended)?.value
        ?? field.options?.[0]?.value
        ?? ""
    } else if (field.type === "multi") {
      answers[field.id] = []
    } else if (field.type === "confirm") {
      answers[field.id] = false
    } else {
      answers[field.id] = ""
    }
  }
  return answers
}

function CopyButton({ content }: { content: string }) {
  const { t } = useTranslation()
  const [copied, setCopied] = useState(false)

  const handleCopy = useCallback(async () => {
    // Strip HTML comments and thinking blocks before copying
    const clean = content
      .replace(/<!--.*?-->/gs, "")
      .replace(/<think(?:ing)?>\s*[\s\S]*?<\/think(?:ing)?>\s*/gi, "")
      .replace(/<think(?:ing)?>\s*[\s\S]*$/gi, "")
      .trim()

    await navigator.clipboard.writeText(clean)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }, [content])

  return (
    <button
      type="button"
      onClick={handleCopy}
      className="inline-flex items-center gap-1 rounded px-2 py-0.5 text-[11px] text-muted-foreground hover:text-link hover:bg-brand-soft transition-colors"
      title={t("chat.copyToClipboard")}
    >
      {copied ? <Check className="h-3 w-3" /> : <Copy className="h-3 w-3" />}
      {copied ? t("chat.copied") : t("chat.copy")}
    </button>
  )
}

/** The question a message of the assistant answers: the user's message before it. */
function questionOf(message: DisplayMessage): string {
  const all = useChatStore.getState().messages.filter((m) => m.conversationId === message.conversationId)
  const at = all.findIndex((m) => m.id === message.id)
  for (let i = at - 1; i >= 0; i--) {
    if (all[i].role === "user") return all[i].content
  }
  return ""
}

/**
 * «Guardar consulta»: the answer, its question and what it cites are kept
 * in the base as a saved query (D-413), listed in the chat. Nothing is
 * written to a file and nothing reads it again with the model.
 */
function SaveQueryButton({ message }: { message: DisplayMessage }) {
  const [saved, setSaved] = useState(false)
  const [saving, setSaving] = useState(false)
  const [problem, setProblem] = useState<string | null>(null)

  const handleSave = useCallback(async () => {
    if (saving) return
    setSaving(true)
    setProblem(null)
    try {
      const answer = cleanAssistantContentForSaving(message.content)
      const title = titleFromCleanAssistantContent(answer)
      const refs = (message.references ?? [])
        .filter((ref) => ref.kind === "wiki")
        .map((ref) => ref.path)
      await saveQuery(title, questionOf(message), answer, [...new Set(refs)])
      setSaved(true)
      setTimeout(() => setSaved(false), 2000)
    } catch (err) {
      setProblem(String((err as Error)?.message ?? err))
    } finally {
      setSaving(false)
    }
  }, [message, saving])

  return (
    <button
      type="button"
      onClick={handleSave}
      disabled={saving}
      className="self-start inline-flex items-center gap-1 rounded px-2 py-0.5 text-[11px] text-muted-foreground hover:text-link hover:bg-brand-soft transition-colors"
      title={problem ?? "Guardar la pregunta y su respuesta en «Consultas guardadas»"}
    >
      <BookmarkPlus className="h-3 w-3" />
      {saved ? "Guardada" : saving ? "Guardando…" : problem ? "No se pudo guardar" : "Guardar consulta"}
    </button>
  )
}

type CitedPage = MessageReference

const REF_TYPE_CONFIG: Record<string, { icon: typeof FileText; color: string }> = {
  entity: { icon: Users, color: "text-brand" },
  concept: { icon: Lightbulb, color: "text-chart-3" },
  source: { icon: BookOpen, color: "text-chart-1" },
  query: { icon: HelpCircle, color: "text-ok" },
  synthesis: { icon: GitMerge, color: "text-chart-2" },
  comparison: { icon: BarChart3, color: "text-info" },
  finding: { icon: TrendingUp, color: "text-chart-3" },
  thesis: { icon: Target, color: "text-chart-2" },
  methodology: { icon: BookOpen, color: "text-info" },
  overview: { icon: Layout, color: "text-chart-5" },
  external: { icon: Globe, color: "text-info" },
  anytxt: { icon: FileSearch, color: "text-ok" },
  workspace: { icon: FileText, color: "text-chart-5" },
}

function getRefType(path: string, page?: CitedPage): string {
  if (page?.kind === "workspace") return "workspace"
  if (page?.kind === "external") {
    return page.source?.toLowerCase() === "anytxt" ? "anytxt" : "external"
  }
  return inferWikiTypeFromPath(path) ?? "source"
}

function displayExternalPath(page: CitedPage): string {
  const raw = page.url || page.path
  if (!raw) return page.path
  if (raw.startsWith("file://")) {
    try {
      const url = new URL(raw)
      const decoded = decodeURIComponent(url.pathname)
      if (/^\/[A-Za-z]:\//.test(decoded)) return decoded.slice(1)
      if (url.hostname) return `//${url.hostname}${decoded}`
      return decoded
    } catch {
      return raw.replace(/^file:\/\//, "")
    }
  }
  return raw
}

function isAnyTxtReference(page: CitedPage): boolean {
  return page.kind === "external" && page.source?.toLowerCase() === "anytxt"
}

function referenceSourceLabel(page: CitedPage): string {
  if (isAnyTxtReference(page)) return "AnyTXT"
  if (page.kind === "workspace") return "Workspace"
  if (page.kind === "external") return page.source || "Web"
  return "Base"
}

function referenceLocator(page: CitedPage): string {
  if (page.kind === "external") return displayExternalPath(page)
  return page.path
}

function referenceSnippet(page: CitedPage): string {
  return page.kind === "external" ? page.snippet?.trim() ?? "" : ""
}

function projectAbsolutePath(projectPath: string, path: string): string {
  const pp = normalizePath(projectPath)
  const normalized = normalizePath(path)
  if (normalized.startsWith(`${pp}/`)) return normalized
  if (isAbsolutePath(normalized)) return normalized
  return `${pp}/${normalized.replace(/^\/+/, "")}`
}

function isAgentWorkspacePath(filePath: string): boolean {
  return normalizePath(filePath).split("/").includes("agent-workspace")
}

function isGeneratedOutputImage(filePath: string): boolean {
  const category = getFileCategory(filePath)
  return category === "image" || (getFileExtension(filePath) === "svg" && isAgentWorkspacePath(filePath))
}

/**
 * Markdown image-reference regex used to count `![](url)` occurrences
 * in cited pages AND extract the first URL (so the image-badge
 * jump button knows where to send the user). Same shape as the
 * search/pipeline regex elsewhere (kept duplicated to avoid
 * coupling — this module never wants to pull caption-pipeline
 * imports for a 3-character count).
 *
 * Group 1 captures the URL (everything inside `(...)` of the
 * markdown image syntax, no whitespace).
 */
const CITED_IMAGE_RE = /!\[[^\]]*\]\(([^)\s]+)\)/g

interface CitedImageInfo {
  count: number
  /** First image URL on the page — used as the scroll target when
   *  the badge button opens the raw source. Null when count===0. */
  firstUrl: string | null
}

function CitedReferencesPanel({
  content,
  savedReferences,
  onOpenReferencePreview,
  onOpenContextDetails,
}: {
  content: string
  savedReferences?: CitedPage[]
  onOpenReferencePreview?: (preview: ChatReferencePreview, relatedPreviews?: ChatReferencePreview[]) => void
  onOpenContextDetails?: (references: MessageReference[]) => void
}) {
  const { t } = useTranslation()
  const workspace = useWikiStore((s) => s.agentWorkspace)
  const [expanded, setExpanded] = useState(false)
  const [outputsExpanded, setOutputsExpanded] = useState(false)
  /**
   * Per-cited-page image info: count + first image URL. We can't
   * hang this off `CitedPage` directly because `extractCitedPages`
   * is sync and works on the AI's text response, never seeing the
   * underlying page. So we fetch the page contents lazily here.
   * Same path → same info, so a tiny in-component map keyed by
   * path is plenty.
   */
  const [imageInfos, setImageInfos] = useState<Record<string, CitedImageInfo>>({})

  // Use saved references first (persisted with message), fall back to dynamic extraction
  const generatedOutputs = useMemo(() => (
    (savedReferences ?? []).filter((page) => page.kind === "workspace")
  ), [savedReferences])
  const citedPages = useMemo(() => {
    if (savedReferences && savedReferences.length > 0) {
      return savedReferences.filter((page) => page.kind !== "workspace")
    }
    return extractCitedPages(content)
  }, [content, savedReferences])
  const contextReferences = savedReferences && savedReferences.length > 0
    ? savedReferences
    : citedPages

  // Async-fetch each cited page's content once and extract image
  // info: count + first URL. Done in parallel; failures are
  // silently treated as { count: 0, firstUrl: null } (page may
  // not exist on disk yet, e.g. a citation the LLM hallucinated).
  useEffect(() => {
    if (citedPages.length === 0) return
    let cancelled = false
    Promise.all(
      citedPages.map(async (page) => {
        // Try the path verbatim first, then the same fallback set
        // the click-handler uses below — keeps "is the file on
        // disk" check consistent across the panel.
        if (page.kind === "external" || page.kind === "workspace") {
          return [page.path, { count: 0, firstUrl: null }] as const
        }
        try {
          const text = (await getPageText(page.path))?.text ?? ""
          // Reset stateful regex.lastIndex by `new RegExp(...)` —
          // module-level `g` regexes carry state across calls
          // and would skip matches on the second invocation.
          const re = new RegExp(CITED_IMAGE_RE.source, CITED_IMAGE_RE.flags)
          const matches = [...text.matchAll(re)]
          const info: CitedImageInfo = {
            count: matches.length,
            firstUrl: matches.length > 0 ? matches[0][1] : null,
          }
          return [page.path, info] as const
        } catch {
          // Not in the base.
        }
        return [page.path, { count: 0, firstUrl: null }] as const
      }),
    ).then((entries) => {
      if (cancelled) return
      const next: Record<string, CitedImageInfo> = {}
      for (const [path, info] of entries) next[path] = info
      setImageInfos(next)
    })
    return () => {
      cancelled = true
    }
  }, [citedPages])

  /** An image of a cited page opens the screen of what the page names (D-413). */
  const handleJumpToImageSource = useCallback(
    async (_firstUrl: string, fallbackPath: string) => {
      await openWikiPath(fallbackPath)
    },
    [],
  )

  const openCitedPage = useCallback(async (page: CitedPage) => {
    if (page.kind === "workspace") {
      if (!workspace) return
      const pp = normalizePath(workspace)
      const workspacePath = projectAbsolutePath(pp, page.path)
      const relatedOutputPreviews = generatedOutputs.map((output) => {
        const outputPath = projectAbsolutePath(pp, output.path)
        return {
          title: output.title,
          path: outputPath,
          source: output.source ?? "Workspace",
          content: output.path === page.path ? page.snippet ?? "" : "",
          snippet: output.snippet,
        }
      })
      try {
        const category = getFileCategory(workspacePath)
        const shouldReadContent = isTextReadable(category) || category === "pdf"
        const content = shouldReadContent ? await readFile(workspacePath) : ""
        if (onOpenReferencePreview) {
          onOpenReferencePreview({
            title: page.title,
            path: workspacePath,
            source: page.source ?? "Workspace",
            content,
            snippet: page.snippet,
          }, relatedOutputPreviews)
        } else {
          await revealItemInDir(workspacePath).catch(() => {})
        }
      } catch (err) {
        console.warn("[chat refs] failed to open workspace reference:", err)
        if (onOpenReferencePreview) {
          onOpenReferencePreview({
            title: page.title,
            path: workspacePath,
            source: page.source ?? "Workspace",
            content: `Unable to load generated file: ${page.path}`,
            snippet: page.snippet,
          }, relatedOutputPreviews)
        }
      }
      return
    }
    if (page.kind === "external") {
      const target = page.url || page.path
      const displayPath = displayExternalPath(page)
      const previewContent = [
        `# ${page.title}`,
        "",
        `**Source:** ${referenceSourceLabel(page)}`,
        `**Path:** ${displayPath}`,
        "",
        "## Preview",
        "",
        page.snippet?.trim() || "(No preview fragment returned.)",
      ].join("\n")
      if (onOpenReferencePreview) {
        onOpenReferencePreview({
          title: page.title,
          path: displayPath,
          source: referenceSourceLabel(page),
          external: true,
          content: previewContent,
          snippet: page.snippet ?? "",
        })
        return
      }
      if (target) {
        await openUrl(target).catch((err) => {
          console.warn("[chat refs] failed to open external reference:", err)
        })
      }
      return
    }
    // A page of the base opens the screen of the document, person,
    // organization or matter it names (D-413); never a file.
    if (await openWikiPath(page.path)) return
    if (onOpenReferencePreview) {
      const text = await getPageText(page.path).catch(() => null)
      onOpenReferencePreview({
        title: page.title,
        path: page.path,
        content: text?.text ?? `No está en la base: ${page.title}`,
      })
    }
  }, [workspace, generatedOutputs, onOpenReferencePreview])

  if (citedPages.length === 0 && generatedOutputs.length === 0) return null

  const MAX_COLLAPSED = 3
  const visiblePages = expanded ? citedPages : citedPages.slice(0, MAX_COLLAPSED)
  const visibleOutputs = outputsExpanded ? generatedOutputs : generatedOutputs.slice(0, MAX_COLLAPSED)
  const hasMore = citedPages.length > MAX_COLLAPSED
  const hasMoreOutputs = generatedOutputs.length > MAX_COLLAPSED

  return (
    <div className="space-y-1">
      {generatedOutputs.length > 0 && (
        <div className="rounded-md border border-primary/20 bg-primary/5 text-xs mb-1">
          <button
            type="button"
            onClick={() => hasMoreOutputs && setOutputsExpanded(!outputsExpanded)}
            className="flex w-full items-center gap-1.5 px-2 py-1 text-link transition-colors hover:text-link/80"
          >
            <Sparkles className="h-3 w-3 shrink-0" />
            <span className="font-medium">{t("chat.generatedOutputs")} ({generatedOutputs.length})</span>
            {hasMoreOutputs && (
              outputsExpanded
                ? <ChevronDown className="h-3 w-3 ml-auto" />
                : <ChevronRight className="h-3 w-3 ml-auto" />
            )}
          </button>
          <div className="px-2 pb-1.5">
            {visibleOutputs.map((page, i) => {
              const refType = getRefType(page.path, page)
              const config = REF_TYPE_CONFIG[refType] ?? REF_TYPE_CONFIG.source
              const Icon = config.icon
              const absoluteOutputPath = workspace ? projectAbsolutePath(workspace, page.path) : page.path
              const isImageOutput = isGeneratedOutputImage(absoluteOutputPath)
              const imageSrc = isImageOutput ? convertFileSrc(absoluteOutputPath) : null
              return (
                <div
                  key={page.path}
                  className="flex w-full items-start gap-1.5 rounded text-left"
                  title={page.path}
                >
                  <span className="mt-1 text-[10px] text-link/60 w-4 shrink-0 text-right">[{i + 1}]</span>
                  <button
                    type="button"
                    onClick={() => openCitedPage(page)}
                    className="flex min-w-0 flex-1 items-start gap-2 rounded px-1 py-1 text-left hover:bg-brand-soft transition-colors"
                  >
                    {imageSrc ? (
                      <span className="h-14 w-20 shrink-0 overflow-hidden rounded border border-border bg-card">
                        <img
                          src={imageSrc}
                          alt={page.title}
                          loading="lazy"
                          className="h-full w-full object-cover"
                          onError={(event) => {
                            event.currentTarget.style.opacity = "0"
                          }}
                        />
                      </span>
                    ) : (
                      <Icon className={`mt-0.5 h-3 w-3 shrink-0 ${config.color}`} />
                    )}
                    <span className="min-w-0 flex-1 text-foreground/90">
                      <span className="block truncate">{page.title}</span>
                      <span className="mt-0.5 block truncate text-[10px] text-muted-foreground/75">
                        {referenceLocator(page)}
                      </span>
                    </span>
                    <span className="shrink-0 rounded border border-border bg-card px-1 py-0 text-[10px] text-brand-ink">
                      {t("chat.generatedOutput")}
                    </span>
                  </button>
                </div>
              )
            })}
            {hasMoreOutputs && !outputsExpanded && (
              <button
                type="button"
                onClick={() => setOutputsExpanded(true)}
                className="w-full text-center text-[10px] text-link/70 hover:text-link pt-0.5"
              >
                +{generatedOutputs.length - MAX_COLLAPSED} more...
              </button>
            )}
          </div>
        </div>
      )}
      {citedPages.length > 0 && (
        <div className="rounded-md border border-border/60 bg-muted/30 text-xs mb-1">
          <div className="flex items-center px-2 py-1 text-muted-foreground">
            <button
              type="button"
              onClick={() => hasMore && setExpanded(!expanded)}
              className="flex min-w-0 flex-1 items-center gap-1.5 text-left hover:text-foreground transition-colors"
            >
              <FileText className="h-3 w-3 shrink-0" />
              <span className="font-medium">{t("chat.references")} ({citedPages.length})</span>
              {hasMore && (
                expanded
                  ? <ChevronDown className="h-3 w-3 ml-auto" />
                  : <ChevronRight className="h-3 w-3 ml-auto" />
              )}
            </button>
            {onOpenContextDetails && (
              <button
                type="button"
                onClick={() => onOpenContextDetails(contextReferences)}
                className="ml-2 inline-flex shrink-0 items-center gap-1 rounded px-1.5 py-0.5 text-[10px] hover:bg-accent hover:text-foreground"
                title={t("chat.contextDetails")}
              >
                <ListTree className="h-3 w-3" />
                {t("chat.details")}
              </button>
            )}
          </div>
          <div className="px-2 pb-1.5">
        <ReferenceKnowledgeGraph references={citedPages} onOpenReference={openCitedPage} />
        {visiblePages.map((page, i) => {
          const refType = getRefType(page.path, page)
          const config = REF_TYPE_CONFIG[refType] ?? REF_TYPE_CONFIG.source
          const Icon = config.icon
          const info = imageInfos[page.path]
          const hasImages = (info?.count ?? 0) > 0
          return (
            // Outer is a div, NOT a button — we have two click
            // targets inside (image badge + main row) and nesting
            // a button inside a button is invalid HTML and breaks
            // event delegation. Hover effect shifts to the inner
            // buttons individually so each gives feedback.
            <div
              key={page.path}
              className="flex w-full items-center gap-1.5 rounded text-left"
              title={page.kind === "external" ? `${referenceSourceLabel(page)}: ${referenceLocator(page)}` : page.path}
            >
              <span className="text-[10px] text-muted-foreground/60 w-4 shrink-0 text-right">[{i + 1}]</span>
              {/*
               * Image badge — clickable, separately from the page
               * row. Click → open the screen of what the page names
               * (D-413).
               *
               * Icon: lucide `Image` (picture-frame outline with
               * mountain + sun) — direct visual cue for "image",
               * NOT `Camera` which reads as "take a photo".
               */}
              {hasImages && info?.firstUrl && (
                <button
                  type="button"
                  onClick={() => handleJumpToImageSource(info.firstUrl!, page.path)}
                  className="flex shrink-0 items-center gap-0.5 rounded px-1 py-0.5 text-[10px] text-link hover:bg-hover transition-colors"
                  title={`Abrir el documento en su primera imagen (${info.count} ${info.count === 1 ? "imagen" : "imágenes"} en esta página)`}
                >
                  <ImageIcon className="h-3 w-3" />
                  {info.count}
                </button>
              )}
              <button
                type="button"
                onClick={() => openCitedPage(page)}
                className="flex min-w-0 flex-1 items-center gap-1.5 rounded px-1 py-0.5 text-left hover:bg-accent/50 transition-colors"
              >
                <Icon className={`h-3 w-3 shrink-0 ${config.color}`} />
                <span className="min-w-0 flex-1 text-foreground/80">
                  <span className="block truncate">{page.title}</span>
                  {page.kind === "external" && (
                    <span className="mt-0.5 block truncate text-[10px] text-muted-foreground/75">
                      {referenceLocator(page)}
                    </span>
                  )}
                  {isAnyTxtReference(page) && referenceSnippet(page) && (
                    <span className="mt-0.5 line-clamp-2 whitespace-normal text-[10px] leading-4 text-muted-foreground">
                      {referenceSnippet(page)}
                    </span>
                  )}
                </span>
                {page.kind === "external" && (
                  <span className="shrink-0 rounded border border-border/60 bg-background/80 px-1 py-0 text-[10px] text-muted-foreground">
                    {referenceSourceLabel(page)}
                  </span>
                )}
              </button>
            </div>
          )
        })}
        {hasMore && !expanded && (
          <button
            type="button"
            onClick={() => setExpanded(true)}
            className="w-full text-center text-[10px] text-muted-foreground hover:text-link pt-0.5"
          >
            +{citedPages.length - MAX_COLLAPSED} more...
          </button>
        )}
          </div>
      </div>
      )}
    </div>
  )
}


/**
 * Extract cited wiki pages from the hidden <!-- cited: 1, 3, 5 --> comment.
 * Maps page numbers back to the pages that were sent to the LLM.
 */
function extractCitedPages(text: string): CitedPage[] {
  const citedMatch = text.match(/<!--\s*cited:\s*(.+?)\s*-->/)
  if (citedMatch && lastQueryPages.length > 0) {
    const numbers = citedMatch[1]
      .split(",")
      .map((s) => parseInt(s.trim(), 10))
      .filter((n) => !isNaN(n) && n >= 1 && n <= lastQueryPages.length)

    const pages = numbers.map((n) => lastQueryPages[n - 1])
    if (pages.length > 0) return pages
  }

  // Fallback: if LLM used [1], [2] notation in text, try to match those
  if (lastQueryPages.length > 0) {
    const numberRefs = text.match(/\[(\d+)\]/g)
    if (numberRefs) {
      const numbers = [...new Set(numberRefs.map((r) => parseInt(r.slice(1, -1), 10)))]
        .filter((n) => n >= 1 && n <= lastQueryPages.length)
      if (numbers.length > 0) {
        return numbers.map((n) => lastQueryPages[n - 1])
      }
    }
  }

  // Fallback for persisted messages: extract [[wikilinks]] from the text
  // Try to resolve each wikilink to a real file path by checking common wiki subdirectories
  const wikilinks = text.match(/\[\[([^\]|]+?)(?:\|[^\]]+?)?\]\]/g)
  if (wikilinks) {
    const seen = new Set<string>()
    const pages: CitedPage[] = []
    const WIKI_DIRS = ["entities", "concepts", "sources", "queries", "synthesis", "comparisons"]

    for (const link of wikilinks) {
      const nameMatch = link.match(/\[\[([^\]|]+?)(?:\|([^\]]+?))?\]\]/)
      if (nameMatch) {
        const id = nameMatch[1].trim()
        const display = nameMatch[2]?.trim() || id

        // Skip if id contains path separators (already a path like queries/xxx)
        if (seen.has(id)) continue
        seen.add(id)

        // Try to find the file in known wiki subdirectories
        let resolvedPath = ""
        if (id.includes("/")) {
          // Already has directory like "queries/my-query"
          resolvedPath = `wiki/${id}.md`
        } else {
          // Search in common directories
          for (const dir of WIKI_DIRS) {
            resolvedPath = `wiki/${dir}/${id}.md`
            // We can't do async file checking here, so try all known patterns
            // The click handler will try multiple paths
            break // Use first candidate, click handler resolves the rest
          }
          if (!resolvedPath) resolvedPath = `wiki/${id}.md`
        }

        pages.push({ title: display, path: resolvedPath })
      }
    }
    if (pages.length > 0) return pages
  }

  // No citations found
  return []
}

interface StreamingMessageProps {
  content: string
  agentEvents?: ChatAgentEvent[]
}

export function StreamingMessage({ content, agentEvents = [] }: StreamingMessageProps) {
  const { thinking, answer } = useMemo(() => separateThinking(content), [content])
  const isThinking = thinking !== null && answer.length === 0

  return (
    <div className="flex gap-2 flex-row">
      <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground">
        <Bot className="h-4 w-4" />
      </div>
      <div className="max-w-[80%] rounded-lg px-3 py-2 text-sm bg-muted text-foreground">
        <AgentActivity events={agentEvents} />
        {isThinking ? (
          <StreamingThinkingBlock content={thinking} />
        ) : (
          <>
            {thinking && <ThinkingBlock content={thinking} />}
            <MarkdownContent content={answer} />
            <span className="animate-pulse">▊</span>
          </>
        )}
      </div>
    </div>
  )
}

function AgentActivity({ events, compact = false }: { events: ChatAgentEvent[]; compact?: boolean }) {
  const { t } = useTranslation()
  const visible = events.filter((event, index, arr) => {
    const prev = arr[index - 1]
    return !prev
      || prev.stage !== event.stage
      || prev.query !== event.query
      || prev.tool !== event.tool
      || prev.message !== event.message
  })
  if (visible.length === 0) return null

  return (
    <div className={`${compact ? "" : "mb-2 border-b border-border/40 pb-2"} flex flex-col gap-1.5`}>
      {visible.map((event, index) => {
        const active = index === visible.length - 1
        const Icon = agentStageIcon(event.stage)
        return (
          <div
            key={`${event.stage}-${event.query ?? ""}-${index}`}
            className={`flex min-w-0 items-center gap-2 text-xs ${
              active ? "text-foreground" : "text-muted-foreground"
            }`}
          >
            <span
              className={`flex h-4 w-4 shrink-0 items-center justify-center ${
                active
                  ? "text-link/70"
                  : "text-muted-foreground/60"
              }`}
            >
              <Icon className={`h-3.5 w-3.5 ${active ? "animate-pulse" : ""}`} />
            </span>
            <span className="truncate">
              {event.message || t(`chat.agent.${event.stage}`)}
              {event.query ? <span className="text-muted-foreground"> · {event.query}</span> : null}
              {typeof event.count === "number" ? (
                <span className="text-muted-foreground"> · {t("chat.agent.resultCount", { count: event.count })}</span>
              ) : null}
            </span>
            {event.timestamp && (
              <time className="ml-auto shrink-0 text-[10px] tabular-nums text-muted-foreground/70">
                {new Date(event.timestamp).toLocaleTimeString([], {
                  hour: "2-digit",
                  minute: "2-digit",
                  second: "2-digit",
                })}
              </time>
            )}
          </div>
        )
      })}
    </div>
  )
}

function agentStageIcon(stage: ChatAgentEventStage) {
  switch (stage) {
    case "understanding":
      return Target
    case "tool_call":
      return Sparkles
    case "tool_result":
      return Check
    case "searching_wiki":
      return BookOpen
    case "searching_graph":
      return GitMerge
    case "searching_web":
      return Globe
    case "searching_anytxt":
      return FileSearch
    case "reading_context":
      return Layout
    case "writing":
      return Bot
    case "routing":
    default:
      return Sparkles
  }
}

function MarkdownContent({ content }: { content: string }) {
  // Strip hidden comments
  const cleaned = content.replace(/<!--.*?-->/gs, "").trimEnd()

  // Project path for resolving wiki-relative image src in chat
  // replies (LLM may surface images that came in via retrieved
  // chunks, e.g. when the chat answer cites a diagram from a wiki
  // page). Same convention the file-preview uses.
  const imageRoot = useWikiStore((s) => s.agentWorkspace)

  // Separate thinking blocks from main content
  const { thinking, answer } = useMemo(() => separateThinking(cleaned), [cleaned])
  const processed = useMemo(() => processContent(answer), [answer])
  const renderLanguage = useMemo(() => detectLanguage(answer), [answer])
  const direction = getTextDirection(renderLanguage)
  const htmlLang = getHtmlLang(renderLanguage)

  return (
    <div>
      {thinking && <ThinkingBlock content={thinking} />}
      <div
        className="chat-markdown prose max-w-none dark:prose-invert prose-code:before:content-none prose-code:after:content-none"
        dir={direction}
        lang={htmlLang}
        style={{ textAlign: "start" }}
      >
        <ReactMarkdown
          remarkPlugins={[remarkGfm, remarkMathPlugin]}
          rehypePlugins={[rehypeKatex]}
          components={{
            a: ({ href, children }) => {
              if (href?.startsWith("wikilink:")) {
                const pageName = href.slice("wikilink:".length)
                return <WikiLink pageName={pageName}>{children}</WikiLink>
              }
              return (
                <span className="text-link underline cursor-default" title={href}>
                  {children}
                </span>
              )
            },
            img: ({ src, alt, ...props }) => (
              <img
                src={typeof src === "string" ? resolveMarkdownImageSrc(src, imageRoot) : undefined}
                alt={alt ?? ""}
                className="my-2 max-w-full rounded border border-border/40"
                loading="lazy"
                {...props}
              />
            ),
            table: ({ children, ...props }) => (
              <div className="my-2 overflow-x-auto rounded border border-border">
                <table className="w-full border-collapse text-xs" {...props}>{children}</table>
              </div>
            ),
            thead: ({ children, ...props }) => (
              <thead className="bg-muted" {...props}>{children}</thead>
            ),
            th: ({ children, ...props }) => (
              <th className="border border-border/80 px-3 py-1.5 text-start font-semibold bg-muted" {...props}>{children}</th>
            ),
            td: ({ children, ...props }) => (
              <td className="border border-border/60 px-3 py-1.5" {...props}>{children}</td>
            ),
            pre: ({ children, ...props }) => {
              const mermaid = unwrapMermaidPre(children)
              if (mermaid) return <>{mermaid}</>
              return (
                <pre
                  dir="ltr"
                  className="rounded bg-background/50 p-2 text-xs overflow-x-auto"
                  style={{ textAlign: "left" }}
                  {...props}
                >
                  {children}
                </pre>
              )
            },
            code: ({ className, children, ...props }) => {
              const lang = className?.replace("language-", "")
              const codeText = String(children).replace(/\n$/, "")
              if (lang === "mermaid") {
                return <MermaidDiagram code={codeText} />
              }
              return <code dir="ltr" className={className} {...props}>{children}</code>
            },
          }}
        >
          {processed}
        </ReactMarkdown>
      </div>
    </div>
  )
}

/**
 * Separate <think>...</think> blocks from the main answer.
 * Handles multiple think blocks and partial (unclosed) thinking during streaming.
 */
function separateThinking(text: string): { thinking: string | null; answer: string } {
  // Match complete <think>...</think> and <thinking>...</thinking> blocks
  const thinkRegex = /<think(?:ing)?>([\s\S]*?)<\/think(?:ing)?>/gi
  const thinkParts: string[] = []
  let answer = text

  let match: RegExpExecArray | null
  while ((match = thinkRegex.exec(text)) !== null) {
    thinkParts.push(match[1].trim())
  }
  answer = answer.replace(/<think(?:ing)?>[\s\S]*?<\/think(?:ing)?>/gi, "").trim()

  // Handle unclosed <think> or <thinking> tag (streaming in progress)
  const unclosedMatch = answer.match(/<think(?:ing)?>([\s\S]*)$/i)
  if (unclosedMatch) {
    thinkParts.push(unclosedMatch[1].trim())
    answer = answer.replace(/<think(?:ing)?>[\s\S]*$/i, "").trim()
  }

  const thinking = thinkParts.length > 0 ? thinkParts.join("\n\n") : null
  return { thinking, answer }
}

/** Streaming thinking: shows latest ~5 lines rolling upward with animation */
function StreamingThinkingBlock({ content }: { content: string }) {
  const { t } = useTranslation()
  const lines = content.split("\n").filter((l) => l.trim())
  const visibleLines = lines.slice(-5)

  return (
    <div className="rounded-[10px] border border-dashed border-border-strong border-border-strong bg-panel-sub px-2.5 py-2">
      <div className="flex items-center gap-1.5 mb-1.5">
        <span className="text-sm animate-pulse">💭</span>
        <span className="text-xs font-medium text-muted-foreground">{t("chat.thinking")}</span>
        <span className="text-[10px] text-muted-foreground/70">{t("chat.lineCount", { count: lines.length })}</span>
      </div>
      <div className="h-[5lh] overflow-hidden text-xs text-muted-foreground font-mono leading-relaxed">
        {visibleLines.map((line, i) => (
          <div
            key={lines.length - 5 + i}
            className="truncate"
            style={{ opacity: 0.4 + (i / visibleLines.length) * 0.6 }}
          >
            {line}
          </div>
        ))}
        <span className="animate-pulse text-brand">▊</span>
      </div>
    </div>
  )
}

/** Completed thinking: collapsed by default, click to expand */
function ThinkingBlock({ content }: { content: string }) {
  const { t } = useTranslation()
  const [expanded, setExpanded] = useState(false)
  const lines = content.split("\n").filter((l) => l.trim())

  return (
    <div className="mb-2 rounded-[10px] border border-dashed border-border-strong border-border-strong bg-panel-sub">
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="flex w-full items-center gap-1.5 px-2.5 py-1.5 text-xs text-muted-foreground hover:bg-hover transition-colors"
      >
        <span className="text-sm">💭</span>
        <span className="font-medium">{t("chat.thoughtLineCount", { count: lines.length })}</span>
        <span className="text-muted-foreground/70">
          {expanded ? "▼" : "▶"}
        </span>
      </button>
      {expanded && (
        <div className="border-t border-border px-2.5 py-2 text-xs text-muted-foreground whitespace-pre-wrap max-h-64 overflow-y-auto font-mono leading-relaxed">
          {content}
        </div>
      )}
    </div>
  )
}

/**
 * Process content to create clickable links:
 * - [[wikilinks]] → markdown links with wikilink: protocol
 */
function processContent(text: string): string {
  let result = text

  // Rewrite Obsidian image embeds (`![[…]]`) into standard markdown
  // FIRST — before the `[[…]]` → wikilink conversion below, which
  // would otherwise mangle the embed target into a broken
  // `wikilink:` image. Same rule the wiki reader / raw preview use.
  result = transformImageEmbeds(result)

  // Wrap bare \begin{...}...\end{...} blocks with $$ for remark-math
  result = result.replace(
    /(?<!\$\$\s*)(\\begin\{[^}]+\}[\s\S]*?\\end\{[^}]+\})(?!\s*\$\$)/g,
    (_match, block: string) => `$$\n${block}\n$$`,
  )

  // Only apply Unicode conversion to text outside of math delimiters
  // Only $$...$$ is math: a single $ is the currency sign (markdown-math.ts)
  const parts = result.split(/(\$\$[\s\S]*?\$\$)/g)
  result = parts
    .map((part) => {
      if (part.startsWith("$")) return part // preserve math
      return convertLatexToUnicode(part)
    })
    .join("")

  // Fix malformed wikilinks like [[name] (missing closing bracket)
  result = result.replace(/\[\[([^\]]+)\](?!\])/g, "[[$1]]")

  // Convert [[wikilinks]] to markdown links
  result = result.replace(
    /\[\[([^\]|]+?)(?:\|([^\]]+?))?\]\]/g,
    (_match, pageName: string, displayText?: string) => {
      const display = displayText?.trim() || pageName.trim()
      return `[${display}](wikilink:${pageName.trim()})`
    }
  )

  return result
}

/**
 * A `[[link]]` of an answer: the screen of what it names, found in the
 * base (D-413); plain text when the base knows nothing by that name.
 */
function WikiLink({ pageName, children }: { pageName: string; children: React.ReactNode }) {
  const [exists, setExists] = useState<boolean | null>(null)
  const found = useRef<PageObject | null>(null)

  useEffect(() => {
    let cancelled = false
    objectOfPage(pageName)
      .then((object) => {
        if (cancelled) return
        found.current = object
        setExists(Boolean(object))
      })
      .catch(() => {
        if (!cancelled) setExists(false)
      })
    return () => { cancelled = true }
  }, [pageName])

  const handleClick = useCallback(() => {
    if (found.current) openPageObject(found.current)
  }, [])

  if (exists === false) {
    return (
      <span className="inline text-muted-foreground" title={`Page not found: ${pageName}`}>
        {children}
      </span>
    )
  }

  return (
    <button
      type="button"
      onClick={handleClick}
      className="inline-flex items-center gap-0.5 rounded px-1 py-0.5 text-link underline decoration-[var(--link-line)] hover:bg-brand-soft hover:decoration-link"
      title={`Abrir: ${pageName}`}
    >
      <FileText className="inline h-3 w-3" />
      {children}
    </button>
  )
}
