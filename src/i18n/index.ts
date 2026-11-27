// Lightweight i18n: two typed dictionaries and `{var}` interpolation.

import { en, type TranslationKey } from "./en";
import { fr } from "./fr";
import type { Language } from "../types/models";

export type Lang = "en" | "fr";
export type { TranslationKey };

const dictionaries: Record<Lang, Record<TranslationKey, string>> = { en, fr };

let current: Lang = "en";

export function resolveLanguage(setting: Language | undefined, systemLanguage = navigator.language): Lang {
  if (setting === "en" || setting === "fr") return setting;
  return systemLanguage.toLowerCase().startsWith("fr") ? "fr" : "en";
}

export function setLanguage(lang: Lang) {
  current = lang;
  if (typeof document !== "undefined") document.documentElement.lang = lang;
}

export function getLanguage(): Lang {
  return current;
}

export function interpolate(template: string, vars?: Record<string, string | number>): string {
  if (!vars) return template;
  return template.replace(/\{(\w+)\}/g, (m, k: string) => (k in vars ? String(vars[k]) : m));
}

/** Unknown keys are returned as-is (useful for dynamic warning codes). */
export function translate(key: TranslationKey | string, vars?: Record<string, string | number>): string {
  const dict = dictionaries[current];
  const template = (dict as Record<string, string>)[key] ?? (en as Record<string, string>)[key] ?? key;
  return interpolate(template, vars);
}

export function formatDate(ms: number, lang: Lang = current): string {
  return new Intl.DateTimeFormat(lang === "fr" ? "fr-FR" : "en-GB", { dateStyle: "medium", timeStyle: "short" }).format(ms);
}
