import { createI18n } from "vue-i18n";
import zhCN from "./locales/zh-CN";
import en from "./locales/en";
import ja from "./locales/ja";
import fr from "./locales/fr";
import ko from "./locales/ko";
import de from "./locales/de";
import ru from "./locales/ru";

const STORAGE_KEY = "photomap.locale";

function detectLocale() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved) return saved;
    const nav = (navigator.language || (navigator.languages && navigator.languages[0]) || "en").toLowerCase();
    if (nav.startsWith("zh")) return "zh-CN";
    if (nav.startsWith("ja")) return "ja";
    if (nav.startsWith("fr")) return "fr";
    if (nav.startsWith("ko")) return "ko";
    if (nav.startsWith("de")) return "de";
    if (nav.startsWith("ru")) return "ru";
    return "en";
  } catch (e) {
    return "en";
  }
}

const i18n = createI18n({
  legacy: false,
  locale: detectLocale(),
  fallbackLocale: "en",
  messages: { "zh-CN": zhCN, en, ja, fr, ko, de, ru },
});

export function setLocale(locale) {
  i18n.global.locale.value = locale;
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch (e) {
    /* ignore */
  }
}

export function getLocale() {
  return i18n.global.locale.value;
}

export default i18n;
