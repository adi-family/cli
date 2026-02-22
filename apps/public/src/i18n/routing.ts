import {defineRouting} from "next-intl/routing";

export const locales = ["en", "uk", "zh", "de", "ru"] as const;
export type Locale = (typeof locales)[number];

export const localeLabels: Record<Locale, string> = {
  en: "EN",
  uk: "UA",
  zh: "ZH",
  de: "DE",
  ru: "RU",
};

export const routing = defineRouting({
  locales,
  defaultLocale: "en",
});
