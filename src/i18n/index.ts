import { useCallback, useEffect, useState } from "react";
import { applyLanguage, getSettings, onSettings } from "../ipc";
import type { Lang } from "../ipc/types";
import { en } from "./en";
import { ja } from "./ja";

export type TKey = keyof typeof ja;

const dictionaries: Record<Lang, Record<TKey, string>> = { ja, en };

export function translate(lang: Lang, key: TKey, params?: Record<string, string | number>): string {
  const template = dictionaries[lang][key] ?? dictionaries.en[key] ?? key;
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (_, name: string) => {
    const value = params[name];
    return value === undefined ? `{${name}}` : String(value);
  });
}

/**
 * The effective UI language, kept in sync with SettingsView.effectiveLanguage, plus a
 * bound `t()`. Calls apply_language on mount and whenever the language changes
 * (docs/impl/ipc.md; docs/recipes/tauri-desktop-app.md gotcha 7), and keeps
 * `<html lang>` in sync for assistive tech.
 *
 * Each browser window only ever mounts one view, so there is no shared provider here --
 * every view calls this hook directly.
 */
export function useLang(): { lang: Lang; t: (key: TKey, params?: Record<string, string | number>) => string } {
  const [lang, setLang] = useState<Lang>("en");

  useEffect(() => {
    let active = true;
    getSettings().then((s) => {
      if (active) setLang(s.effectiveLanguage);
    });
    let unlisten: (() => void) | undefined;
    onSettings((s) => {
      if (active) setLang(s.effectiveLanguage);
    }).then((u) => {
      unlisten = u;
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    document.documentElement.lang = lang;
    void applyLanguage();
  }, [lang]);

  const t = useCallback(
    (key: TKey, params?: Record<string, string | number>) => translate(lang, key, params),
    [lang]
  );

  return { lang, t };
}
