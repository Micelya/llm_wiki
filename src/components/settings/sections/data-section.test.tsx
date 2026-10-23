import { renderToStaticMarkup } from "react-dom/server"
import { describe, expect, it } from "vitest"
import type { BackupSummary, DataStatus } from "@/commands/docbase"
import { backupsToGoBackTo, DataView, sizeInWords } from "./data-section"

/** The visible text, without tags, with spaces collapsed. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&quot;/g, '"')
    .replace(/&#x27;/g, "'")
    .replace(/\s+/g, " ")
    .trim()
}

// Every folder and date below is made up.
const backup = (name: string, baseVersion: number, forVersion: number, madeAt: string, complete = true): BackupSummary => ({
  name,
  baseVersion,
  forVersion,
  writtenByApp: "0.1.0",
  madeAt,
  madeAtMs: 0,
  bytes: 49_600_000,
  complete,
})

const status = (changes: Partial<DataStatus> = {}): DataStatus => ({
  dataFolder: "C:\\Usuarios\\prueba\\AppData\\Roaming\\com.micelya.desktop",
  backupsFolder: "C:\\Usuarios\\prueba\\AppData\\Roaming\\com.micelya.desktop\\respaldos",
  baseVersion: 25,
  appBaseVersion: 25,
  appVersion: "0.1.0",
  opening: { foundVersion: 25, backup: null, pruned: [], problem: null, newer: false },
  backups: [],
  keep: 3,
  restored: null,
  ...changes,
})

function shown(value: DataStatus, problem: string | null = null): string {
  return text(
    renderToStaticMarkup(
      <DataView status={value} problem={problem} busy={false} onOpen={() => {}} onRestore={() => {}} onClose={() => {}} />,
    ),
  )
}

describe("Ajustes → Datos y respaldos", () => {
  it("says where the data and the backups are, the version of the base and how many backups are kept", () => {
    const words = shown(status())
    expect(words).toContain("C:\\Usuarios\\prueba\\AppData\\Roaming\\com.micelya.desktop")
    expect(words).toContain("Versión de la base: 25 (la de esta versión de la app, 0.1.0).")
    expect(words).toContain("Se conservan los 3 últimos")
    expect(words).toContain("Si no puede hacer el respaldo, no actualiza nada.")
    expect(words).toContain("Todavía no hay respaldos")
    expect(words).toContain("Abrir la carpeta de datos")
    expect(words).toContain("Abrir la carpeta de respaldos")
    expect(words).toContain("no marques «Eliminar los datos de aplicación»")
    expect(words).not.toContain("Volver a este respaldo")
  })

  it("tells the update the app made on opening, its backup and the older ones deleted", () => {
    const words = shown(
      status({
        opening: { foundVersion: 24, backup: "antes-de-v25-2026-10-24-0930", pruned: ["antes-de-v22-2026-09-01-1000"], problem: null, newer: false },
        backups: [backup("antes-de-v25-2026-10-24-0930", 24, 25, "24/10/2026 09:30")],
      }),
    )
    expect(words).toContain("Al abrir, la app actualizó la base de la versión 24 a la 25. Antes guardó el respaldo «antes-de-v25-2026-10-24-0930». Se borró el respaldo más viejo.")
    expect(words).toContain("24/10/2026 09:30 — la base en la versión 24, antes de pasar a la 25 · 47,3 MB · la había escrito la app 0.1.0")
  })

  it("says why the base could not be opened when the backup failed, without offering to go back", () => {
    const problem = "Antes de actualizar la base de la versión 24 a la 25, Micelya tiene que guardar un respaldo y esta vez no pudo (disco lleno). No se cambió nada."
    const words = shown(status({ baseVersion: 24, opening: { foundVersion: 24, backup: null, pruned: [], problem, newer: false } }))
    expect(words).toContain("La base no se pudo abrir")
    expect(words).toContain(problem)
    expect(words).toContain("Versión de la base: 24; esta versión de la app (0.1.0) usa la 25.")
    expect(words).not.toContain("Volver a la versión de antes")
  })

  it("offers to go back to a backup this version can open when a newer version wrote the base", () => {
    const value = status({
      baseVersion: 26,
      opening: { foundVersion: 26, backup: null, pruned: [], problem: "Esta base la escribió una versión más nueva de Micelya.", newer: true },
      backups: [
        backup("antes-de-v27-2026-11-02-1100", 26, 27, "02/11/2026 11:00"),
        backup("antes-de-v26-2026-11-01-0900", 25, 26, "01/11/2026 09:00"),
        backup("antes-de-v26-2026-10-30-0800", 25, 26, "30/10/2026 08:00", false),
      ],
    })
    expect(backupsToGoBackTo(value).map((b) => b.name)).toEqual(["antes-de-v26-2026-11-01-0900"])
    const words = shown(value)
    expect(words).toContain("Volver a la versión de antes")
    expect(words).toContain("se pierde lo que se hizo en la app después de su fecha")
    expect(words).toContain("01/11/2026 09:00 — la base en la versión 25 (el más reciente) Volver a este respaldo")
    expect(words).toContain("incompleto: le faltan archivos")
  })

  it("asks to close and open the app again once it went back to a backup", () => {
    const words = shown(
      status({
        opening: { foundVersion: 26, backup: null, pruned: [], problem: "Esta base la escribió una versión más nueva.", newer: true },
        restored: "antes-de-v26-2026-11-01-0900",
        backups: [backup("antes-de-v26-2026-11-01-0900", 25, 26, "01/11/2026 09:00")],
      }),
    )
    expect(words).toContain("Se volvió al respaldo «antes-de-v26-2026-11-01-0900». Para usarlo, cerrá Micelya y volvé a abrirla.")
    expect(words).toContain("Cerrar Micelya")
    expect(words).not.toContain("Volver a este respaldo")
    expect(words).not.toContain("La base no se pudo abrir")
  })

  it("says that no backup can be used when none is of a version this app opens", () => {
    const words = shown(
      status({
        opening: { foundVersion: 27, backup: null, pruned: [], problem: "Más nueva.", newer: true },
        backups: [backup("antes-de-v27-2026-11-02-1100", 26, 27, "02/11/2026 11:00")],
      }),
    )
    expect(words).toContain("No hay un respaldo que esta versión pueda abrir")
  })

  it("writes sizes as a person reads them", () => {
    expect(sizeInWords(49_600_000)).toBe("47,3 MB")
    expect(sizeInWords(12_000)).toBe("12 KB")
    expect(sizeInWords(10)).toBe("1 KB")
  })
})
