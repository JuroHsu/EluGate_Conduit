import { createContext, Fragment, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { syncLanguage } from "./language-sync";

/** 語系檔位於專案根目錄的 locales/，Rust 核心與介面共用；此處自動載入所有檔案 */
const BUNDLES: Record<string, Record<string, string>> = Object.fromEntries(
  Object.entries(import.meta.glob<Record<string, string>>("../locales/*.json", { eager: true, import: "default" })).map(
    ([path, bundle]) => [path.replace(/^.*\/(.+)\.json$/, "$1"), bundle],
  ),
);

/** 語言選單顯示各語言的原生名稱 */
export const LANGUAGES: [string, string][] = [
  ["en", "English"],
  ["zh-TW", "繁體中文"],
  ["zh-CN", "简体中文"],
  ["ja", "日本語"],
  ["ko", "한국어"],
  ["es", "Español"],
  ["fr", "Français"],
  ["de", "Deutsch"],
  ["pt-BR", "Português (Brasil)"],
  ["ru", "Русский"],
  ["vi", "Tiếng Việt"],
];

type Params = Record<string, string | number>;

function lookup(lang: string, key: string): string {
  return BUNDLES[lang]?.[key] ?? BUNDLES.en[key] ?? key;
}

function format(template: string, params?: Params): string {
  return params ? template.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m)) : template;
}

interface I18n {
  lang: string;
  t: (key: string, params?: Params) => string;
  /** 參數可以是 React 元素，例如把指令包在 <code> 裡 */
  tx: (key: string, params: Record<string, ReactNode>) => ReactNode;
}

const Ctx = createContext<I18n | null>(null);

function make(lang: string): I18n {
  return {
    lang,
    t: (key, params) => format(lookup(lang, key), params),
    tx: (key, params) =>
      lookup(lang, key)
        .split(/(\{\w+\})/)
        .map((part, i) => {
          const m = /^\{(\w+)\}$/.exec(part);
          return <Fragment key={i}>{m && m[1] in params ? params[m[1]] : part}</Fragment>;
        }),
  };
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [lang, setLang] = useState<string | null>(null);
  useEffect(() => syncLanguage(
    () => invoke<string>("get_language"),
    (onChange) => listen<string>("language-changed", (e) => onChange(e.payload)),
    setLang,
  ), []);
  useEffect(() => {
    if (lang) document.documentElement.lang = lang;
  }, [lang]);
  const value = useMemo(() => (lang ? make(lang) : null), [lang]);
  if (!value) return null;
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useI18n(): I18n {
  const v = useContext(Ctx);
  if (!v) throw new Error("useI18n must be used inside I18nProvider");
  return v;
}
