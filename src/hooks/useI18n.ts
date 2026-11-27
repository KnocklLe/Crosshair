import { useCallback } from "react";
import { formatDate, getLanguage, translate, type TranslationKey } from "../i18n";
import { useStore } from "../services/store";

/** Re-renders the component when the language setting changes. */
export function useI18n() {
  const language = useStore((s) => s.settings?.language);
  const lang = getLanguage();
  const t = useCallback(
    (key: TranslationKey, vars?: Record<string, string | number>) => translate(key, vars),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [language, lang],
  );
  const date = useCallback((ms: number) => formatDate(ms, lang), [lang]);
  return { t, lang, date };
}
