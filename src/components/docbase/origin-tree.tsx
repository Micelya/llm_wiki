import { useState, type ReactNode } from "react"
import {
  BookPlus,
  ChevronDown,
  ChevronRight,
  ExternalLink,
  Eye,
  EyeOff,
  FileText,
  Folder,
  RefreshCw,
} from "lucide-react"
import {
  listFolder,
  openFolder,
  openLocation,
  rescanOrigins,
  setExcluded,
  type DocumentRow,
  type FolderNode,
} from "@/commands/docbase"
import { cn } from "@/lib/utils"
import { useDocbaseStore } from "@/stores/docbase-store"
import { useWikiStore } from "@/stores/wiki-store"
import { DocumentStateBadge } from "./document-state"
import { formatCount } from "./format"
import { useDocbaseQuery } from "./use-docbase-query"

export interface TreeActions {
  /** Whether "incorporar al wiki" can be used right now. */
  canIngest: boolean
  /** Queues for the wiki what is pending under a folder, or one file. */
  onIngest: (originId: string, relPath: string, isFolder: boolean) => void
  onProblem: (problem: string | null) => void
  /** Extra buttons for a folder row (label rules). */
  folderExtras?: (originId: string, folder: string) => ReactNode
}

/**
 * One origin as a tree of folders and files. Each folder loads its
 * content when it is opened.
 */
export function OriginTree({ originId, actions }: { originId: string; actions: TreeActions }) {
  return (
    <div className="rounded-md border text-sm">
      <FolderContent originId={originId} folder="" depth={0} actions={actions} />
    </div>
  )
}

function FolderContent({
  originId,
  folder,
  depth,
  actions,
}: {
  originId: string
  folder: string
  depth: number
  actions: TreeActions
}) {
  const listing = useDocbaseQuery(`folder:${originId}:${folder}`, () => listFolder(originId, folder))
  const data = listing.data
  if (!data) {
    return listing.error ? <p className="px-3 py-2 text-xs text-destructive">{listing.error}</p> : null
  }
  if (data.folders.length === 0 && data.files.length === 0) {
    return (
      <p className="px-3 py-2 text-xs text-muted-foreground" style={{ paddingLeft: 12 + depth * 18 }}>
        Sin archivos.
      </p>
    )
  }
  return (
    <>
      {data.folders.map((node) => (
        <FolderRow key={node.relPath} originId={originId} node={node} depth={depth} actions={actions} />
      ))}
      {data.files.map((file) => (
        <FileRow key={file.locationId} file={file} depth={depth} actions={actions} />
      ))}
    </>
  )
}

const ROW = "group flex items-center gap-2 border-b px-3 py-1 last:border-b-0 hover:bg-muted/40"

function FolderRow({
  originId,
  node,
  depth,
  actions,
}: {
  originId: string
  node: FolderNode
  depth: number
  actions: TreeActions
}) {
  const [open, setOpen] = useState(false)

  async function run(action: () => Promise<void>) {
    actions.onProblem(null)
    try {
      await action()
    } catch (cause) {
      actions.onProblem(String(cause))
    }
  }

  return (
    <>
      <div className={ROW} style={{ paddingLeft: 12 + depth * 18 }}>
        <button className="flex min-w-0 flex-1 items-center gap-1.5 text-left" onClick={() => setOpen(!open)}>
          {open ? <ChevronDown className="size-4 shrink-0" /> : <ChevronRight className="size-4 shrink-0" />}
          <Folder className="size-4 shrink-0 text-muted-foreground" />
          <span className={cn("truncate font-medium", node.excluded && "text-muted-foreground line-through")}>
            {node.name}
          </span>
          <span className="shrink-0 text-xs text-muted-foreground">{formatCount(node.files)}</span>
          {node.excluded && <span className="shrink-0 text-xs text-muted-foreground">excluida</span>}
        </button>
        <RowActions>
          {actions.folderExtras?.(originId, node.relPath)}
          <IconAction title="Abrir la carpeta" onClick={() => run(() => openFolder(originId, node.relPath))}>
            <ExternalLink />
          </IconAction>
          <IconAction
            title={actions.canIngest ? "Incorporar al wiki lo pendiente de esta carpeta" : "El wiki no está activo"}
            disabled={!actions.canIngest || node.excluded}
            onClick={() => actions.onIngest(originId, node.relPath, true)}
          >
            <BookPlus />
          </IconAction>
          <IconAction
            title={node.excluded ? "Volver a incluir" : "Excluir: la base deja de procesarla. No borra nada."}
            onClick={() => run(() => setExcluded(originId, node.relPath, true, !node.excluded))}
          >
            {node.excluded ? <Eye /> : <EyeOff />}
          </IconAction>
          <IconAction title="Volver a barrer" onClick={() => run(rescanOrigins)}>
            <RefreshCw />
          </IconAction>
        </RowActions>
      </div>
      {open && <FolderContent originId={originId} folder={node.relPath} depth={depth + 1} actions={actions} />}
    </>
  )
}

function FileRow({ file, depth, actions }: { file: DocumentRow; depth: number; actions: TreeActions }) {
  const openDocument = useDocbaseStore((s) => s.openDocument)
  const setActiveView = useWikiStore((s) => s.setActiveView)
  const excluded = file.state === "excluded"

  async function run(action: () => Promise<void>) {
    actions.onProblem(null)
    try {
      await action()
    } catch (cause) {
      actions.onProblem(String(cause))
    }
  }

  return (
    <div className={ROW} style={{ paddingLeft: 12 + depth * 18 + 22 }}>
      <button
        className="flex min-w-0 flex-1 items-center gap-1.5 text-left"
        title="Ver el detalle del documento"
        onClick={() => {
          openDocument(file.locationId)
          setActiveView("documents")
        }}
      >
        <FileText className="size-4 shrink-0 text-muted-foreground" />
        <span className={cn("truncate", excluded && "text-muted-foreground line-through")}>{file.name}</span>
      </button>
      <DocumentStateBadge state={file.state} />
      <RowActions>
        <IconAction title="Abrir con el programa del sistema" onClick={() => run(() => openLocation(file.locationId))}>
          <ExternalLink />
        </IconAction>
        <IconAction
          title={actions.canIngest ? "Incorporar al wiki" : "El wiki no está activo"}
          disabled={!actions.canIngest || excluded || file.state === "link"}
          onClick={() => actions.onIngest(file.originId, file.relPath, false)}
        >
          <BookPlus />
        </IconAction>
        <IconAction
          title={excluded ? "Volver a incluir" : "Excluir: la base deja de procesarlo. No borra nada."}
          onClick={() => run(() => setExcluded(file.originId, file.relPath, false, !excluded))}
        >
          {excluded ? <Eye /> : <EyeOff />}
        </IconAction>
      </RowActions>
    </div>
  )
}

function RowActions({ children }: { children: ReactNode }) {
  return <div className="flex shrink-0 items-center gap-0.5 opacity-60 group-hover:opacity-100">{children}</div>
}

export function IconAction({
  title,
  disabled,
  onClick,
  children,
}: {
  title: string
  disabled?: boolean
  onClick: () => void
  children: ReactNode
}) {
  return (
    <button
      title={title}
      disabled={disabled}
      onClick={(event) => {
        event.stopPropagation()
        onClick()
      }}
      className="flex size-6 items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground disabled:pointer-events-none disabled:opacity-30 [&_svg]:size-3.5"
    >
      {children}
    </button>
  )
}
