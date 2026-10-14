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
      relations: {
        "same-document": { name: "Mismo documento" },
        "near-identical": { name: "Prácticamente el mismo" },
        "signed-copy": { name: "Ejemplar firmado", of: "es el ejemplar firmado de" },
        preview: { name: "Vista previa", of: "es la vista previa de" },
        version: { name: "Versión" },
        "contained-in": { name: "Contenido en", of: "está casi entero dentro de" },
        "same-template": { name: "Misma plantilla, otros datos" },
        similar: { name: "Parecidos, sin relación clara" },
        "same-meaning": { name: "Dice lo mismo con otras palabras" },
        "translation-of": { name: "Traducción", of: "es una traducción de" },
        "reply-to": { name: "Respuesta", of: "responde a" },
        "signing-certificate-of": { name: "Constancia de firma", of: "es la constancia de firma de" },
        "delivery-receipt-of": { name: "Acuse de recibo", of: "es el acuse de recibo de" },
        "same-matter": { name: "Mismo asunto" },
        "same-person-other-matter": { name: "Misma persona, otro asunto" },
      },
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
      relations: {
        "same-document": { name: "Same document" },
        "near-identical": { name: "Practically the same" },
        "signed-copy": { name: "Signed copy", of: "is the signed copy of" },
        preview: { name: "Preview", of: "is the preview of" },
        version: { name: "Version" },
        "contained-in": { name: "Contained in", of: "is almost entirely inside" },
        "same-template": { name: "Same template, other data" },
        similar: { name: "Similar, no clear relation" },
        "same-meaning": { name: "Says the same in other words" },
        "translation-of": { name: "Translation", of: "is a translation of" },
        "reply-to": { name: "Reply", of: "replies to" },
        "signing-certificate-of": { name: "Signing certificate", of: "is the signing certificate of" },
        "delivery-receipt-of": { name: "Delivery receipt", of: "is the delivery receipt of" },
        "same-matter": { name: "Same matter" },
        "same-person-other-matter": { name: "Same person, other matter" },
      },
    },
  },
}
