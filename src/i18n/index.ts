import i18n from "i18next"
import { initReactI18next } from "react-i18next"
import en from "./en.json"
import es from "./es.json"
import it from "./it.json"
import zh from "./zh.json"
import ru from "./ru.json"

i18n.use(initReactI18next).init({
  resources: {
    es: { translation: es },
    en: { translation: en },
    it: { translation: it },
    zh: { translation: zh },
    ru: { translation: ru },
  },
  // The screens in Spanish (P-058, P-057): the language of the studios
  // the app is for, unless a person chose another one in Ajustes.
  lng: "es",
  fallbackLng: "es",
  interpolation: { escapeValue: false },
})

export default i18n
