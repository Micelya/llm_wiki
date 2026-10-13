import type { Vertical } from "./index"

/** Law firms. */
export const legal: Vertical = {
  default: "es",
  languages: {
    es: {
      archive: {
        name: "Archivo",
        mine: "Mi archivo",
        shared: "Archivo del estudio",
        health: "Salud del archivo",
        wiki: "Wiki del archivo",
        wikiDescription:
          "El wiki que se arma con todos los documentos de las carpetas incorporadas.",
      },
      suggestedGroups: ["Cliente", "Caso"],
      wikiLanguage: "Spanish",
    },
    en: {
      archive: {
        name: "Archive",
        mine: "My archive",
        shared: "Firm archive",
        health: "Archive health",
        wiki: "Archive wiki",
        wikiDescription: "The wiki built from every document in the incorporated folders.",
      },
      suggestedGroups: ["Client", "Matter"],
      wikiLanguage: "English",
    },
  },
}
