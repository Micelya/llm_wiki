import { useCallback, useEffect, useState } from "react"
import { Check, Copy, KeyRound, Loader2, RefreshCw, Trash2 } from "lucide-react"
import { Badge, type Tone } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Callout } from "@/components/ui/callout"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Panel } from "@/components/ui/panel"
import { formatAgo } from "@/components/docbase/format"
import {
  addProgram,
  getLocalServerStatus,
  listPrograms,
  programToken,
  removeProgram,
  renewProgram,
  setProgramPermission,
  setProgramsEnabled,
  type LocalProgram,
  type LocalServerState,
  type LocalServerStatus,
  type ProgramPermission,
  type ProgramWithToken,
} from "@/commands/local-server"
import { SERVER_STATE_WORDS, TOKEN_VARIABLE, apiExample, claudeCodeCommand, codexConfig, lastUseWords, permissionWords } from "@/lib/local-server"
import { useAppDialog } from "@/stores/app-dialog-store"
import { vocabulary } from "@/vocabulary"

const STATE_TONE: Record<LocalServerState, Tone> = { starting: "info", running: "ok", "port-in-use": "err", stopped: "err" }

function message(cause: unknown): string {
  return typeof cause === "string" ? cause : String((cause as Error)?.message ?? cause)
}

/**
 * Ajustes → Programas conectados (D-495, the contract's design 6.7): the
 * server inside the machine and the programs a person lets use the base.
 * Whether the server is on, its address, a switch for the programs (the
 * web clipper always works), each program with its permission, its last
 * use and «Copiar el token», «Renovar» and «Quitar», «Agregar un programa»,
 * and how to connect an assistant by MCP, with an example to copy. Applies
 * at once, without the Save bar.
 */
export function ProgramsSection() {
  const dialog = useAppDialog()
  const matter = vocabulary().knowledge.matter.toLowerCase()
  const permissions = permissionWords(matter)
  const [status, setStatus] = useState<LocalServerStatus | null>(null)
  const [programs, setPrograms] = useState<LocalProgram[] | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const [busy, setBusy] = useState<string | null>(null)
  const [name, setName] = useState("")
  const [permission, setPermission] = useState<ProgramPermission>("read")
  const [fresh, setFresh] = useState<ProgramWithToken | null>(null)
  const [copied, setCopied] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      const [server, list] = await Promise.all([getLocalServerStatus(), listPrograms()])
      setStatus(server)
      setPrograms(list)
    } catch (cause) {
      setProblem(message(cause))
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const run = async (key: string, action: () => Promise<void>) => {
    setBusy(key)
    setProblem(null)
    try {
      await action()
    } catch (cause) {
      setProblem(message(cause))
    } finally {
      setBusy(null)
    }
  }

  const copy = async (key: string, text: string) => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(key)
      window.setTimeout(() => setCopied((current) => (current === key ? null : current)), 2000)
    } catch (cause) {
      setProblem(`No se pudo copiar: ${message(cause)}`)
    }
  }

  const toggle = () =>
    run("switch", async () => {
      if (!status) return
      setStatus(await setProgramsEnabled(!status.programsEnabled))
    })

  const add = () =>
    run("add", async () => {
      const added = await addProgram(name, permission)
      setFresh(added)
      setName("")
      setPermission("read")
      await load()
    })

  const copyToken = (program: LocalProgram) =>
    run(`token:${program.id}`, async () => {
      await copy(`token:${program.id}`, await programToken(program.id))
    })

  const renew = async (program: LocalProgram) => {
    const sure = await dialog.confirm({
      title: `Renovar el token de «${program.name}»`,
      message: "El token de ahora deja de andar enseguida: hay que copiar el nuevo en ese programa.",
      confirmLabel: "Renovar",
      cancelLabel: "Cancelar",
    })
    if (!sure) return
    await run(`renew:${program.id}`, async () => {
      setFresh(await renewProgram(program.id))
      await load()
    })
  }

  const remove = async (program: LocalProgram) => {
    const sure = await dialog.confirm({
      title: `Quitar «${program.name}»`,
      message:
        "Su token deja de andar enseguida y sale del llavero del sistema. Lo que ya pidió queda en el registro, y lo que espera en Atención se puede seguir decidiendo.",
      confirmLabel: "Quitar",
      cancelLabel: "Cancelar",
      variant: "destructive",
    })
    if (!sure) return
    await run(`remove:${program.id}`, async () => {
      await removeProgram(program.id)
      if (fresh?.program.id === program.id) setFresh(null)
      await load()
    })
  }

  const changePermission = (program: LocalProgram, next: ProgramPermission) =>
    run(`permission:${program.id}`, async () => {
      await setProgramPermission(program.id, next)
      await load()
    })

  const mcpAddress = status?.mcpAddress ?? "http://127.0.0.1:19827/mcp"
  const address = status?.address ?? "http://127.0.0.1:19827"

  return (
    <div className="space-y-6">
      <div>
        <h2 className="font-ui text-[1.75rem] leading-tight font-bold tracking-[-0.01em] text-strong">Programas conectados</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Otros programas de esta computadora pueden usar la base por el servidor de Micelya: un asistente como Claude Code o
          Codex (por MCP) o un programa propio (por la API local). Cada uno tiene su propio token. Por defecto solo lee; si le
          das permiso para pedir cambios, lo que pida espera en Atención a que una persona lo confirme. Nada se puede usar
          desde otra computadora. Los cambios se aplican al momento.
        </p>
      </div>

      {problem && (
        <Callout tone="err" title="No se pudo">
          {problem}
        </Callout>
      )}

      <Panel
        title="El servidor de Micelya"
        action={
          status && (
            <label className="flex items-center gap-2 text-sm">
              <span className="text-muted-foreground">{status.programsEnabled ? "Programas permitidos" : "Programas apagados"}</span>
              <button
                type="button"
                role="switch"
                aria-checked={status.programsEnabled}
                aria-label="Permitir que otros programas usen la base"
                disabled={busy === "switch"}
                onClick={() => void toggle()}
                className={`relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors ${
                  status.programsEnabled ? "border-primary bg-primary" : "border-muted-foreground/30 bg-muted-foreground/20 hover:bg-muted-foreground/30"
                }`}
              >
                <span
                  className={`inline-block h-3.5 w-3.5 rounded-full bg-primary-foreground shadow-card ring-1 ring-border-strong transition-transform ${
                    status.programsEnabled ? "translate-x-4" : "translate-x-0.5"
                  }`}
                />
              </button>
            </label>
          )
        }
      >
        <div className="space-y-3 text-sm">
          <div className="flex flex-wrap items-center gap-2">
            {status ? <Badge tone={STATE_TONE[status.state]}>{SERVER_STATE_WORDS[status.state]}</Badge> : <Badge>Leyendo…</Badge>}
            {status && !status.programsEnabled && <Badge tone="warn">Programas apagados</Badge>}
          </div>
          <CopyLine label="Dirección" text={address} copied={copied === "address"} onCopy={() => void copy("address", address)} />
          <CopyLine label="MCP" text={mcpAddress} copied={copied === "mcp"} onCopy={() => void copy("mcp", mcpAddress)} />
          <p className="text-muted-foreground">
            El recortador web (Micelya Clipper) guarda páginas siempre, sin token, aunque los programas estén apagados.
          </p>
          {status && !status.programsEnabled && (
            <Callout tone="warn" title="Los programas están apagados">
              Ningún programa puede usar la base, ni con su token. El recortador sigue andando.
            </Callout>
          )}
        </div>
      </Panel>

      <Panel title="Programas" count={programs?.length ?? undefined}>
        <div className="space-y-4 text-sm">
          {programs && programs.length === 0 && <p className="text-muted-foreground">Todavía no agregaste ningún programa.</p>}
          {programs && programs.length > 0 && (
            <ul className="divide-y rounded-lg border">
              {programs.map((program) => (
                <li key={program.id} className="space-y-2 px-3 py-2.5" data-program={program.id}>
                  <div className="flex flex-wrap items-center gap-2">
                    <KeyRound className="size-4 shrink-0 text-muted-foreground" aria-hidden />
                    <span className="font-ui font-bold text-strong">{program.name}</span>
                    <select
                      aria-label={`Permiso de ${program.name}`}
                      value={program.permission}
                      disabled={busy !== null}
                      onChange={(event) => void changePermission(program, event.target.value as ProgramPermission)}
                      className="h-8 rounded-md border border-input bg-background px-2 text-sm"
                    >
                      <option value="read">{permissions.read.label}</option>
                      <option value="change">{permissions.change.label}</option>
                    </select>
                    <span className="text-xs text-muted-foreground">{lastUseWords(program.lastUsedAt, program.uses, (ms) => formatAgo(ms))}</span>
                  </div>
                  <div className="flex flex-wrap gap-2 pl-6">
                    <Button size="sm" variant="outline" disabled={busy !== null} onClick={() => void copyToken(program)}>
                      {copied === `token:${program.id}` ? <Check /> : <Copy />}
                      {copied === `token:${program.id}` ? "Copiado" : "Copiar el token"}
                    </Button>
                    <Button size="sm" variant="outline" disabled={busy !== null} onClick={() => void renew(program)}>
                      <RefreshCw /> Renovar
                    </Button>
                    <Button size="sm" variant="ghost" disabled={busy !== null} onClick={() => void remove(program)}>
                      <Trash2 /> Quitar
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          )}

          {fresh && (
            <Callout tone={fresh.kept ? "ok" : "warn"} title={`El token de «${fresh.program.name}»`}>
              <p>
                Copialo en ese programa. Quien tenga este token puede usar la base como «{fresh.program.name}»: no lo compartas.
                {fresh.kept
                  ? " Se puede volver a copiar con «Copiar el token»."
                  : " No se pudo guardar en el llavero del sistema: copialo ahora, después no se puede volver a ver (se puede renovar)."}
              </p>
              <CodeBox text={fresh.token} copied={copied === "fresh"} onCopy={() => void copy("fresh", fresh.token)} />
              <p className="mt-2">Para Claude Code, en una terminal:</p>
              <CodeBox
                text={claudeCodeCommand(mcpAddress, fresh.token)}
                copied={copied === "fresh-claude"}
                onCopy={() => void copy("fresh-claude", claudeCodeCommand(mcpAddress, fresh.token))}
              />
            </Callout>
          )}

          <div className="space-y-2 rounded-lg border border-dashed p-3">
            <div className="font-ui font-bold text-strong">Agregar un programa</div>
            <div className="grid gap-3 sm:grid-cols-[1fr_auto]">
              <div className="space-y-1.5">
                <Label htmlFor="program-name">Nombre</Label>
                <Input
                  id="program-name"
                  value={name}
                  onChange={(event) => setName(event.target.value)}
                  placeholder="por ejemplo, Claude Code de la oficina"
                  autoComplete="off"
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="program-permission">Permiso</Label>
                <select
                  id="program-permission"
                  value={permission}
                  onChange={(event) => setPermission(event.target.value as ProgramPermission)}
                  className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
                >
                  <option value="read">{permissions.read.label}</option>
                  <option value="change">{permissions.change.label}</option>
                </select>
              </div>
            </div>
            <p className="text-xs text-muted-foreground">{permissions[permission].hint}</p>
            <Button size="sm" disabled={!name.trim() || busy !== null} onClick={() => void add()}>
              {busy === "add" && <Loader2 className="animate-spin" />}
              Agregar
            </Button>
          </div>
        </div>
      </Panel>

      <Panel title="Cómo conectar un asistente por MCP">
        <div className="space-y-3 text-sm">
          <p className="text-muted-foreground">
            Agregá el programa arriba y usá su token en lugar de <span className="font-mono">EL-TOKEN</span>. El asistente usa su
            propio modelo con las operaciones de la base; Micelya no le pregunta nada a su modelo por él.
          </p>
          <div>
            <p className="font-semibold text-strong">Claude Code, en una terminal</p>
            <CodeBox text={claudeCodeCommand(mcpAddress)} copied={copied === "claude"} onCopy={() => void copy("claude", claudeCodeCommand(mcpAddress))} />
          </div>
          <div>
            <p className="font-semibold text-strong">Codex, en su archivo de configuración (config.toml)</p>
            <CodeBox text={codexConfig(mcpAddress)} copied={copied === "codex"} onCopy={() => void copy("codex", codexConfig(mcpAddress))} />
            <p className="mt-1 text-xs text-muted-foreground">
              Con la variable de entorno <span className="font-mono">{TOKEN_VARIABLE}</span> con el token del programa. Todavía
              no se probó con Codex.
            </p>
          </div>
          <div>
            <p className="font-semibold text-strong">Un programa propio, por la API local</p>
            <CodeBox text={apiExample(address)} copied={copied === "api"} onCopy={() => void copy("api", apiExample(address))} />
            <p className="mt-1 text-xs text-muted-foreground">
              Devuelve las operaciones que ese programa puede usar, con lo que pide cada una. Se piden con POST a
              /api/v1/operaciones/&lt;nombre&gt;, con los datos en JSON.
            </p>
          </div>
          <p className="text-xs text-muted-foreground">
            Claude Desktop y los programas que solo aceptan servidores «de línea de comandos» necesitan un puente, que todavía
            no está.
          </p>
        </div>
      </Panel>
    </div>
  )
}

function CopyLine({ label, text, copied, onCopy }: { label: string; text: string; copied: boolean; onCopy: () => void }) {
  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="w-20 shrink-0 text-muted-foreground">{label}</span>
      <span className="font-mono text-[0.8125rem] break-all">{text}</span>
      <Button size="sm" variant="ghost" onClick={onCopy} aria-label={`Copiar ${label.toLowerCase()}`}>
        {copied ? <Check /> : <Copy />}
        {copied ? "Copiado" : "Copiar"}
      </Button>
    </div>
  )
}

function CodeBox({ text, copied, onCopy }: { text: string; copied: boolean; onCopy: () => void }) {
  return (
    <div className="mt-1 flex items-start gap-2">
      <pre className="min-w-0 flex-1 overflow-x-auto rounded-lg border bg-panel-sub px-3 py-2 font-mono text-xs whitespace-pre-wrap [overflow-wrap:anywhere] text-strong">
        {text}
      </pre>
      <Button size="sm" variant="outline" onClick={onCopy}>
        {copied ? <Check /> : <Copy />}
        {copied ? "Copiado" : "Copiar"}
      </Button>
    </div>
  )
}
