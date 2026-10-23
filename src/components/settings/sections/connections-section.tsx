import { useEffect, useState } from "react"
import { Cloud, Loader2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { testCoreConnection, type CoreConnectionFields, type CoreConnectionTest, type CoreConnectionView } from "@/commands/micelya-core"
import { EMPTY_CORE_FIELDS, coreConnectionProblems, coreFieldsChanged, coreTestText } from "@/lib/micelya-core"
import { useAppDialog } from "@/stores/app-dialog-store"
import { loadCoreConnection, removeCoreConnection, saveCoreConnection, useCoreConnectionStore } from "@/stores/core-connection-store"

function fieldsOf(connection: CoreConnectionView | null): CoreConnectionFields {
  if (!connection) return EMPTY_CORE_FIELDS
  return { api: connection.api, issuer: connection.issuer, tenant: connection.tenant, clientId: connection.clientId }
}

const FIELDS: { key: keyof CoreConnectionFields; label: string; placeholder: string; hint?: string }[] = [
  { key: "api", label: "Dirección de la API", placeholder: "https://api.core.micelya.net/v1" },
  { key: "issuer", label: "Emisor", placeholder: "https://…/realms/…", hint: "El que da los permisos de acceso (Keycloak)." },
  { key: "tenant", label: "Organización", placeholder: "el identificador que dio Core" },
  { key: "clientId", label: "Identificador del cliente", placeholder: "el usuario técnico de la organización" },
]

/**
 * Ajustes → Conexiones: the data to talk to a provider, loaded once and
 * used by every service that chooses it (Ajustes → Servicios de IA). Today
 * Micelya Core (D-463, D-468): its address, issuer, organization and
 * technical user, with the secret kept in the system keychain and never
 * shown; «Probar la conexión» asks for an access permit and the list of
 * readings with what is on screen, without keeping anything.
 */
export function ConnectionsSection() {
  const dialog = useAppDialog()
  const connection = useCoreConnectionStore((s) => s.connection)
  const loaded = useCoreConnectionStore((s) => s.loaded)
  const readProblem = useCoreConnectionStore((s) => s.problem)
  const [fields, setFields] = useState<CoreConnectionFields>(() => fieldsOf(connection))
  const [secret, setSecret] = useState("")
  const [changingSecret, setChangingSecret] = useState(false)
  const [busy, setBusy] = useState<"save" | "test" | "remove" | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const [saved, setSaved] = useState(false)
  const [test, setTest] = useState<{ result: CoreConnectionTest; fields: CoreConnectionFields } | null>(null)

  useEffect(() => {
    void loadCoreConnection()
  }, [])
  // What is kept comes in (or changes): the form shows it.
  useEffect(() => {
    setFields(fieldsOf(connection))
    setSecret("")
    setChangingSecret(false)
  }, [connection])

  const hasSecret = !!connection?.hasSecret
  const problems = coreConnectionProblems(fields, secret, hasSecret)
  const changed = coreFieldsChanged(fields, connection ? fieldsOf(connection) : null, secret)
  const newSecret = secret.trim() ? secret : null

  function edit(key: keyof CoreConnectionFields, value: string) {
    setFields((current) => ({ ...current, [key]: value }))
    setSaved(false)
    setTest(null)
  }

  async function save() {
    setProblem(null)
    setBusy("save")
    try {
      await saveCoreConnection(fields, newSecret)
      setSaved(true)
    } catch (cause) {
      setProblem(`No se pudo guardar la conexión: ${String(cause)}`)
    } finally {
      setBusy(null)
    }
  }

  async function probe() {
    setProblem(null)
    setTest(null)
    setBusy("test")
    try {
      setTest({ result: await testCoreConnection(fields, newSecret), fields })
    } catch (cause) {
      setProblem(`No se pudo probar la conexión: ${String(cause)}`)
    } finally {
      setBusy(null)
    }
  }

  async function remove() {
    const confirmed = await dialog.confirm({
      title: "Quitar la conexión con Micelya Core",
      message:
        "Se borran del llavero del sistema sus datos y su secreto. Los servicios que usan Micelya Core quedan sin motor hasta que se cargue de nuevo; lo ya leído no se pierde.",
      confirmLabel: "Quitar",
      cancelLabel: "Cancelar",
      variant: "destructive",
    })
    if (!confirmed) return
    setProblem(null)
    setBusy("remove")
    try {
      await removeCoreConnection()
      setTest(null)
      setSaved(false)
    } catch (cause) {
      setProblem(`No se pudo quitar la conexión: ${String(cause)}`)
    } finally {
      setBusy(null)
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold">Conexiones</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Los datos para hablar con un proveedor se cargan una vez y los usan todos los servicios que lo eligen en «Servicios
          de IA». Los secretos se guardan en el llavero del sistema, no en un archivo, y no se vuelven a mostrar.
        </p>
      </div>

      <section className="space-y-4 rounded-md border p-4">
        <div className="flex items-start gap-3">
          <Cloud className="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground" />
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-base font-semibold">Micelya Core</h3>
              <span className={`text-xs ${connection ? "text-emerald-600" : "text-muted-foreground"}`}>
                {!loaded ? "Leyendo…" : connection ? "Cargada" : "Sin cargar"}
              </span>
              {saved && <span className="text-xs text-emerald-600">Guardado</span>}
            </div>
            <p className="text-sm text-muted-foreground">
              La plataforma de servicios de IA de Micelya. Hoy lee el texto de escaneos y fotos (Ajustes → Servicios de IA →
              «Reconocer escaneos»).
            </p>
            {connection?.environment && (
              <p className="mt-1 text-xs text-muted-foreground">Ambiente: {connection.environment}</p>
            )}
          </div>
        </div>

        {readProblem && (
          <p className="text-xs text-destructive">No se pudo leer la conexión guardada en el llavero: {readProblem}</p>
        )}

        <div className="grid gap-3 sm:grid-cols-2">
          {FIELDS.map((field) => (
            <div key={field.key} className="space-y-1.5">
              <Label htmlFor={`core-${field.key}`}>{field.label}</Label>
              <Input
                id={`core-${field.key}`}
                value={fields[field.key]}
                onChange={(event) => edit(field.key, event.target.value)}
                placeholder={field.placeholder}
                spellCheck={false}
                autoComplete="off"
              />
              {field.hint && <p className="text-xs text-muted-foreground">{field.hint}</p>}
            </div>
          ))}
          <div className="space-y-1.5 sm:col-span-2">
            <Label htmlFor="core-secret">Secreto del cliente</Label>
            {hasSecret && !changingSecret ? (
              <div className="flex flex-wrap items-center gap-2 text-sm">
                <span className="text-muted-foreground">Guardado en el llavero del sistema.</span>
                <Button variant="outline" size="sm" onClick={() => setChangingSecret(true)}>
                  Cambiar
                </Button>
              </div>
            ) : (
              <div className="flex flex-wrap items-center gap-2">
                <Input
                  id="core-secret"
                  type="password"
                  className="max-w-sm"
                  value={secret}
                  onChange={(event) => {
                    setSecret(event.target.value)
                    setSaved(false)
                    setTest(null)
                  }}
                  placeholder={hasSecret ? "el secreto nuevo" : "el secreto que dio Core"}
                  autoComplete="new-password"
                />
                {hasSecret && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      setSecret("")
                      setChangingSecret(false)
                    }}
                  >
                    Dejar el guardado
                  </Button>
                )}
              </div>
            )}
          </div>
        </div>

        {problems.length > 0 && changed && (
          <ul className="space-y-0.5 text-xs text-amber-700 dark:text-amber-400">
            {problems.map((text) => (
              <li key={text}>{text}</li>
            ))}
          </ul>
        )}

        <div className="flex flex-wrap gap-2">
          <Button size="sm" onClick={() => void save()} disabled={!changed || problems.length > 0 || busy !== null}>
            {busy === "save" && <Loader2 className="animate-spin" />}
            Guardar
          </Button>
          <Button variant="outline" size="sm" onClick={() => void probe()} disabled={problems.length > 0 || busy !== null}>
            {busy === "test" && <Loader2 className="animate-spin" />}
            Probar la conexión
          </Button>
          {connection && (
            <Button variant="ghost" size="sm" onClick={() => void remove()} disabled={busy !== null}>
              Quitar la conexión
            </Button>
          )}
        </div>

        {test && (
          <div
            data-core-test={test.result.ok ? "ok" : "failed"}
            className={`rounded-md border px-3 py-2 text-sm ${test.result.ok ? "border-emerald-600/40" : "border-destructive/40"}`}
          >
            <p className={test.result.ok ? "text-emerald-700 dark:text-emerald-400" : "text-destructive"}>
              {coreTestText(test.result, test.fields)}
            </p>
            {!test.result.ok && test.result.step !== "fields" && test.result.detail && (
              <p className="mt-1 break-words text-xs text-muted-foreground">Lo que respondió: {test.result.detail}</p>
            )}
            {test.result.ok && changed && <p className="mt-1 text-xs text-muted-foreground">Falta guardar estos datos.</p>}
          </div>
        )}
        {problem && <p className="rounded-md border border-destructive/40 px-3 py-2 text-sm text-destructive">{problem}</p>}
      </section>
    </div>
  )
}
