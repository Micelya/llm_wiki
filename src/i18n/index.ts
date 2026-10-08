import i18n from "i18next"
import { initReactI18next } from "react-i18next"
import en from "./en.json"
import it from "./it.json"
import zh from "./zh.json"
import ru from "./ru.json"
import { productNamePostProcessor } from "./product-name"

i18n.use(initReactI18next).use(productNamePostProcessor).init({
  resources: {
    en: { translation: en },
    it: { translation: it },
    zh: { translation: zh },
    ru: { translation: ru },
  },
  lng: "en",
  fallbackLng: "en",
  postProcess: [productNamePostProcessor.name],
  interpolation: { escapeValue: false },
})

export default i18n
