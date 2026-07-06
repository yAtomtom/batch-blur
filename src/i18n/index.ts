/**
 * i18next 初期化（副作用モジュール）。main.tsx から import する。
 *
 * 言語の決定順:
 *   1. localStorage("batch-blur.locale") に保存済みの選択
 *   2. なければブラウザ/OS ロケール（navigator）から判定
 * 以降の切替は detector が localStorage に永続化する（Theme と命名を揃える）。
 */

import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { ja, en } from "./locales";

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      ja: { translation: ja },
      en: { translation: en },
    },
    fallbackLng: "ja",
    supportedLngs: ["ja", "en"],
    // "en-US" 等を "en" に丸める。
    nonExplicitSupportedLngs: true,
    interpolation: {
      // React 側でエスケープ済みのため二重エスケープを避ける。
      escapeValue: false,
    },
    detection: {
      order: ["localStorage", "navigator"],
      caches: ["localStorage"],
      lookupLocalStorage: "batch-blur.locale",
    },
    // リソースは同梱・同期ロードのため Suspense は不要（初期描画のちらつき回避）。
    react: { useSuspense: false },
  });

export default i18n;
