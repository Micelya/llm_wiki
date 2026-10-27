import { useEffect, useMemo, useState } from "react"
import { Cpu, Globe, ScanText, Sparkles } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import type { AiServiceId, AiServices, CatalogEngine, EngineChoice, EnginesCatalog } from "@/commands/docbase"
import { availableLlmPresets, findLlmPreset, type LlmPreset } from "@/components/settings/llm-presets"
import {
  CAPABILITY_LEVEL_NAMES,
  REASONING_NAMES,
  catalogEngine,
  choiceName,
  engineOfProvider,
  servicesUsing,
} from "@/lib/ai-services"
import { CODEX_REASONING_LEVELS, codexReasoningEffort } from "@/lib/codex-reasoning"
import { resolveSearchConfig } from "@/lib/web-search"
import { updateAiServices, useAiServicesStore } from "@/stores/ai-services-store"
import { loadCoreConnection, useCoreConnectionStore } from "@/stores/core-connection-store"
import { useWikiStore, type ProviderConfigs, type SearchProviderOverride } from "@/stores/wiki-store"

/** Where the data of a connection is edited. */
export type ConnectionsPlace = "llm" | "web-search" | "connections"

/** The shared connections of Ajustes → Conexiones, by the name the catalog gives them. */
const SHARED_CONNECTIONS: Record<string, string> = { "micelya-core": "Micelya Core" }

const ICONS: Record<AiServiceId, typeof Cpu> = { ask: Sparkles, recognize: ScanText, research: Globe }

/**
 * Ajustes → Servicios de IA (P-141, D-443): one row per service with its
 * engine, its model and reasoning, and an optional alternative engine;
 * what each engine says it can do, where it processes what it is sent and
 * what its provider keeps; and the connections in use. A change is saved
 * at once in the base, which uses it from its next request.
 */
export function AiServicesSection({ onOpen }: { onOpen: (place: ConnectionsPlace) => void }) {
  const services = useAiServicesStore((s) => s.services)
  const catalog = useAiServicesStore((s) => s.catalog)
  const loaded = useAiServicesStore((s) => s.loaded)
  const customLlmPresets = useWikiStore((s) => s.customLlmPresets)
  const providerConfigs = useWikiStore((s) => s.providerConfigs)
  const searchApiConfig = useWikiStore((s) => s.searchApiConfig)
  const taskModelRouting = useWikiStore((s) => s.taskModelRouting)
  const coreConnected = useCoreConnectionStore((s) => !!s.connection?.hasSecret)
  const [problem, setProblem] = useState<string | null>(null)
  const [saved, setSaved] = useState<AiServiceId | null>(null)
  const presets = useMemo(() => availableLlmPresets(customLlmPresets), [customLlmPresets])
  const searchConfigs: Record<string, SearchProviderOverride | undefined> = resolveSearchConfig(searchApiConfig).providerConfigs ?? {}
  // As in «Búsqueda en internet»: Firecrawl works without a key, SearXNG needs its address.
  const searchConfigured = (id: string) =>
    id === "firecrawl" || (id === "searxng" ? !!searchConfigs[id]?.searXngUrl : !!searchConfigs[id]?.apiKey)
  // Whether the shared connection an engine needs is loaded (today only Micelya Core's).
  const connected = (connection: string | undefined) => !connection || (connection === "micelya-core" && coreConnected)
  useEffect(() => {
    void loadCoreConnection()
  }, [])

  async function change(service: AiServiceId, part: "engine" | "alternative", choice: EngineChoice | null) {
    setProblem(null)
    try {
      await updateAiServices((current): AiServices => ({ ...current, [service]: { ...current[service], [part]: choice } }))
      setSaved(service)
      setTimeout(() => setSaved((current) => (current === service ? null : current)), 1500)
    } catch (cause) {
      setProblem(String(cause))
    }
  }

  async function chatFollowsTheService() {
    const wiki = useWikiStore.getState()
    const next = { ...wiki.taskModelRouting, chatPresetId: null }
    wiki.setTaskModelRouting(next)
    const { saveTaskModelRouting } = await import("@/lib/project-store")
    await saveTaskModelRouting(next)
  }

  if (!loaded || !catalog) {
    return (
      <div className="space-y-2">
        <h2 className="font-ui text-[1.75rem] leading-tight font-bold tracking-[-0.01em] text-strong">Servicios de IA</h2>
        <p className="text-sm text-muted-foreground">
          No se pudo leer qué motor usa cada servicio: la base documental no está disponible.
        </p>
      </div>
    )
  }

  const askConnection = services.ask.engine?.connection ?? null
  const chatApart = taskModelRouting.chatPresetId && taskModelRouting.chatPresetId !== askConnection
    ? findLlmPreset(taskModelRouting.chatPresetId, customLlmPresets)
    : undefined

  return (
    <div className="space-y-6">
      <div>
        <h2 className="font-ui text-[1.75rem] leading-tight font-bold tracking-[-0.01em] text-strong">Servicios de IA</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Qué motor usa cada servicio, con qué modelo, y uno alternativo si hace falta. La base lee esta elección en cada
          pedido: un cambio se usa en el próximo. Los datos para conectarse con cada proveedor están en «Conexiones»
          (Micelya Core), «Conexiones con modelos» y «Búsqueda en internet»; sus claves se guardan en el llavero del
          sistema, no en un archivo.
        </p>
      </div>

      {problem && <p className="rounded-md border border-err-line px-3 py-2 text-sm text-destructive">{problem}</p>}

      {catalog.services.map((service) => {
        const Icon = ICONS[service.id]
        const choice = services[service.id]
        const engine = catalogEngine(catalog, service.id, choice.engine?.engine)
        return (
          <section key={service.id} className="space-y-4 rounded-md border p-4">
            <div className="flex items-start gap-3">
              <Icon className="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground" />
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <h3 className="font-ui text-base font-bold text-strong">{service.name}</h3>
                  {saved === service.id && <span className="text-xs text-ok">Guardado</span>}
                </div>
                <p className="text-sm text-muted-foreground">{service.purpose}</p>
              </div>
            </div>

            <ChoiceEditor
              label={service.id === "research" ? "Buscador" : "Motor"}
              service={service.id}
              catalog={catalog}
              presets={presets}
              providerConfigs={providerConfigs}
              searchConfigured={searchConfigured}
              connected={connected}
              choice={choice.engine}
              onChange={(next) => void change(service.id, "engine", next)}
            />

            {choice.engine && engine && <EngineFacts catalog={catalog} engine={engine} model={choice.engine.model} />}
            {choice.engine && engine?.connection && !connected(engine.connection) && (
              <div className="flex flex-wrap items-center gap-2 text-xs text-warn">
                <span>
                  Falta la conexión con {SHARED_CONNECTIONS[engine.connection] ?? engine.connection}: sin ella no se lee
                  nada con este motor.
                </span>
                <Button variant="outline" size="sm" onClick={() => onOpen("connections")}>
                  Ir a Conexiones
                </Button>
              </div>
            )}
            {engine?.unavailable && (
              <p className="text-xs text-warn">
                No está disponible en esta computadora: faltan sus modelos, que se instalan con la app. Mientras falten,
                los escaneos los lee el motor elegido, con la lectura automática prendida o a pedido.
              </p>
            )}
            {choice.engine && engine && !engine.inBase && service.id !== "research" && (
              <p className="text-xs text-warn">
                {service.id === "ask"
                  ? "La base todavía no lee con este motor: el chat lo usa, pero la lectura de los documentos queda en pausa hasta elegir Codex."
                  : "La base todavía no reconoce escaneos con este motor."}
              </p>
            )}
            {!choice.engine && (
              <p className="text-xs text-warn">
                {service.id === "ask"
                  ? "Sin motor no se lee ningún documento ni contesta el chat."
                  : service.id === "recognize"
                    ? "Sin motor, los escaneos quedan sin texto."
                    : "Sin buscador no se puede investigar en internet."}
              </p>
            )}
            {service.id === "ask" && chatApart && (
              <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                <span>El chat usa otro motor, elegido antes: {chatApart.label}.</span>
                <Button variant="outline" size="sm" onClick={() => void chatFollowsTheService()}>
                  Que use el mismo
                </Button>
              </div>
            )}
            {service.id === "research" && (
              <p className="text-xs text-muted-foreground">
                La síntesis de lo encontrado la escribe el motor de «Preguntar al modelo».
              </p>
            )}

            <details className="rounded-md bg-muted/30 px-3 py-2" open={!!choice.alternative}>
              <summary className="cursor-pointer text-sm">
                Motor alternativo (opcional): {choiceName(catalog, service.id, choice.alternative, presetLabel(choice.alternative, presets))}
              </summary>
              <div className="mt-3 space-y-2">
                <p className="text-xs text-muted-foreground">
                  Para lo que el principal no hace bien (por ejemplo, manuscrito). Queda guardado; la base todavía no
                  lo usa sola: hoy hay un solo motor para comparar.
                </p>
                <ChoiceEditor
                  label="Alternativo"
                  service={service.id}
                  catalog={catalog}
                  presets={presets}
                  providerConfigs={providerConfigs}
                  searchConfigured={searchConfigured}
                  connected={connected}
                  choice={choice.alternative}
                  onChange={(next) => void change(service.id, "alternative", next)}
                />
              </div>
            </details>
          </section>
        )
      })}

      <section className="space-y-3 rounded-md border p-4">
        <h3 className="font-ui text-base font-bold text-strong">Conexiones en uso</h3>
        <p className="text-sm text-muted-foreground">
          Los datos de un proveedor (su clave, su dirección) se cargan una vez y los usan todos los servicios que lo eligen.
        </p>
        <ul className="space-y-2 text-sm">
          {connectionsInUse(services, presets, catalog).map((connection) => (
            <li key={`${connection.place}:${connection.id}`} className="flex flex-wrap items-center justify-between gap-2">
              <span>
                <span className="font-medium">{connection.label}</span>
                <span className="text-muted-foreground"> — {connection.services.join(", ")}</span>
              </span>
              <Button variant="ghost" size="sm" onClick={() => onOpen(connection.place)}>
                Ver sus datos
              </Button>
            </li>
          ))}
          {connectionsInUse(services, presets, catalog).length === 0 && (
            <li className="text-muted-foreground">Ningún servicio tiene un motor elegido.</li>
          )}
        </ul>
      </section>
    </div>
  )
}

function presetLabel(choice: EngineChoice | null, presets: LlmPreset[]): string | undefined {
  if (!choice) return undefined
  return presets.find((preset) => preset.id === choice.connection)?.label
}

/** The connections the registry uses, with the services that use each one. */
function connectionsInUse(services: AiServices, presets: LlmPreset[], catalog: EnginesCatalog) {
  const found = new Map<string, { id: string; label: string; place: ConnectionsPlace; services: string[] }>()
  for (const id of ["ask", "recognize", "research"] as const) {
    for (const choice of [services[id].engine, services[id].alternative]) {
      if (!choice?.connection) continue
      const shared = SHARED_CONNECTIONS[choice.connection]
      const place: ConnectionsPlace = shared ? "connections" : id === "research" ? "web-search" : "llm"
      const key = `${place}:${choice.connection}`
      if (found.has(key)) continue
      const label = shared
        ? shared
        : place === "llm"
          ? presets.find((preset) => preset.id === choice.connection)?.label ?? choice.connection
          : catalogEngine(catalog, "research", choice.connection)?.name ?? choice.connection
      const names = servicesUsing(services, choice.connection)
        .filter((service) => shared || (place === "web-search") === (service === "research"))
        .map((service) => catalog.services.find((s) => s.id === service)?.name ?? service)
      found.set(key, { id: choice.connection, label, place, services: names })
    }
  }
  return [...found.values()]
}

interface ChoiceEditorProps {
  label: string
  service: AiServiceId
  catalog: EnginesCatalog
  presets: LlmPreset[]
  providerConfigs: ProviderConfigs
  searchConfigured: (id: string) => boolean
  /** Whether the shared connection an engine needs is loaded. */
  connected: (connection: string | undefined) => boolean
  choice: EngineChoice | null
  onChange: (choice: EngineChoice | null) => void
}

/** The engine of a service (or its alternative), with its model and reasoning. */
function ChoiceEditor({ label, service, catalog, presets, providerConfigs, searchConfigured, connected, choice, onChange }: ChoiceEditorProps) {
  const options = engineOptions(service, catalog, presets)
  const engine = catalogEngine(catalog, service, choice?.engine)
  const [model, setModel] = useState(choice?.model ?? "")
  useEffect(() => setModel(choice?.model ?? ""), [choice?.model, choice?.connection])
  const preset = choice ? presets.find((p) => p.id === choice.connection) : undefined
  const suggestions = preset?.suggestedModels ?? []
  const id = `${service}-${label}`.toLowerCase().replace(/\s+/g, "-")

  function pick(value: string) {
    if (!value) return onChange(null)
    const option = options.find((o) => o.value === value)
    if (!option) return
    const saved = providerConfigs[option.connection]
    const optionPreset = presets.find((p) => p.id === option.connection)
    const entry = catalogEngine(catalog, service, option.engine)
    onChange({
      engine: option.engine,
      connection: option.connection,
      model: service === "research" ? "" : (saved?.model ?? optionPreset?.defaultModel ?? entry?.defaultModel ?? "").trim(),
      reasoning: entry?.reasoning ? codexReasoningEffort(saved?.codexReasoningEffort) : "",
    })
  }

  function commitModel() {
    if (!choice || model.trim() === choice.model) return
    onChange({ ...choice, model: model.trim() })
  }

  return (
    <div className="grid gap-3 sm:grid-cols-3">
      <div className="space-y-1.5">
        <Label htmlFor={`${id}-engine`}>{label}</Label>
        <select
          id={`${id}-engine`}
          value={choice ? optionValue(service, choice) : ""}
          onChange={(event) => pick(event.target.value)}
          className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
        >
          <option value="">Ninguno</option>
          {options.map((option) => {
            // An engine whose shared connection is not loaded cannot be chosen.
            const entry = catalogEngine(catalog, service, option.engine)
            const missing = !connected(entry?.connection)
            // The engine of this computer without its models.
            const unavailable = !!entry?.unavailable
            return (
              <option
                key={option.value}
                value={option.value}
                disabled={(missing || unavailable) && option.value !== (choice ? optionValue(service, choice) : "")}
              >
                {option.label}
                {service === "research" && !searchConfigured(option.connection) ? " (faltan sus datos)" : ""}
                {missing ? " (falta la conexión)" : ""}
                {unavailable ? " (no disponible)" : ""}
              </option>
            )
          })}
        </select>
      </div>
      {choice && service !== "research" && engine?.modelChoice !== false && (
        <div className="space-y-1.5">
          <Label htmlFor={`${id}-model`}>Modelo</Label>
          <Input
            id={`${id}-model`}
            value={model}
            list={suggestions.length ? `${id}-models` : undefined}
            onChange={(event) => setModel(event.target.value)}
            onBlur={commitModel}
            onKeyDown={(event) => {
              if (event.key === "Enter") commitModel()
            }}
            placeholder={preset?.defaultModel ?? "el del proveedor"}
          />
          {suggestions.length > 0 && (
            <datalist id={`${id}-models`}>
              {suggestions.map((m) => (
                <option key={m} value={m} />
              ))}
            </datalist>
          )}
        </div>
      )}
      {choice && engine?.reasoning && (
        <div className="space-y-1.5">
          <Label htmlFor={`${id}-reasoning`}>Razonamiento</Label>
          <select
            id={`${id}-reasoning`}
            value={choice.reasoning || codexReasoningEffort(undefined)}
            onChange={(event) => onChange({ ...choice, reasoning: event.target.value })}
            className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
          >
            {CODEX_REASONING_LEVELS.map((level) => (
              <option key={level} value={level}>
                {REASONING_NAMES[level]}
              </option>
            ))}
          </select>
        </div>
      )}
    </div>
  )
}

interface EngineOption {
  value: string
  label: string
  engine: string
  connection: string
}

/** The value of a choice in its select: the connection for asking the model (a preset), the engine for the rest. */
function optionValue(service: AiServiceId, choice: EngineChoice): string {
  return service === "ask" ? choice.connection || choice.engine : choice.engine
}

/**
 * What a service can choose: for asking the model, every connection of
 * Conexiones (each one is an engine of the catalog); for the rest, the
 * engines of the catalog for that service.
 */
function engineOptions(service: AiServiceId, catalog: EnginesCatalog, presets: LlmPreset[]): EngineOption[] {
  if (service === "ask") {
    return presets
      .filter((preset) => catalogEngine(catalog, "ask", engineOfProvider(preset.provider)))
      .map((preset) => ({ value: preset.id, label: preset.label, engine: engineOfProvider(preset.provider), connection: preset.id }))
  }
  return catalog.engines
    .filter((engine) => engine.service === service)
    .map((engine) => ({
      value: engine.id,
      label: engine.name,
      engine: engine.id,
      connection:
        engine.connection ??
        (service === "recognize" ? presets.find((p) => engineOfProvider(p.provider) === engine.id)?.id ?? engine.id : engine.id),
    }))
}

/** What an engine says it can do, where it processes and what its provider keeps. */
function EngineFacts({ catalog, engine, model }: { catalog: EnginesCatalog; engine: CatalogEngine; model: string }) {
  const capabilities = engine.capabilities
    ? catalog.capabilities
        .filter((capability) => engine.capabilities?.[capability.id])
        .map((capability) => `${capability.name}: ${CAPABILITY_LEVEL_NAMES[engine.capabilities?.[capability.id] ?? ""] ?? "sin declarar"}`)
    : []
  return (
    <dl className="grid gap-x-4 gap-y-1 rounded-md bg-muted/30 px-3 py-2 text-xs sm:grid-cols-[max-content_1fr]">
      {capabilities.length > 0 && (
        <>
          <dt className="font-medium">Sabe leer</dt>
          <dd className="text-muted-foreground">{capabilities.join(" · ")} (lo que declara; se confirma comparando con escaneos reales)</dd>
        </>
      )}
      {engine.modelChoice === false && model && (
        <>
          <dt className="font-medium">Modelo</dt>
          <dd className="text-muted-foreground">{model} (el que usa el proveedor; queda guardado con cada texto)</dd>
        </>
      )}
      {engine.notes && (
        <>
          <dt className="font-medium">Cómo lee</dt>
          <dd className="text-muted-foreground">{engine.notes}</dd>
        </>
      )}
      {engine.languages && engine.languages.length > 0 && (
        <>
          <dt className="font-medium">Idiomas</dt>
          <dd className="text-muted-foreground">{engine.languages.join(", ")}</dd>
        </>
      )}
      <dt className="font-medium">Dónde se procesa</dt>
      <dd className="text-muted-foreground">{engine.processing}</dd>
      <dt className="font-medium">Qué guarda el proveedor</dt>
      <dd className="text-muted-foreground">{engine.retention}</dd>
    </dl>
  )
}
