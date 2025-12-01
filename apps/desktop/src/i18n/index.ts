import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { ko } from "./locales/ko";
import { en } from "./locales/en";
import type { SupportedLocale } from "./types";

const STORAGE_KEY = "mod-translator-locale";

function getStoredLocale(): SupportedLocale {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === "en" || stored === "ko") {
      return stored;
    }
  } catch {
    // ignore localStorage errors
  }

  // Detect browser language
  const browserLang = navigator.language.toLowerCase();
  if (browserLang.startsWith("ko")) {
    return "ko";
  }
  return "en";
}

export function persistLocale(locale: SupportedLocale): void {
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    // ignore localStorage errors
  }
}

export const supportedLocales: SupportedLocale[] = ["ko", "en"];

export const localeLabels: Record<SupportedLocale, string> = {
  ko: "한국어",
  en: "English",
};

i18n.use(initReactI18next).init({
  resources: {
    ko: { translation: ko },
    en: { translation: en },
  },
  lng: getStoredLocale(),
  fallbackLng: "en",
  interpolation: {
    escapeValue: false, // React already escapes values
  },
  react: {
    useSuspense: false,
  },
});

export default i18n;
