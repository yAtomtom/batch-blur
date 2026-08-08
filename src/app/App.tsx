/**
 * アプリ全体のオーケストレーション。
 *
 * 状態の要:
 * - `settings`: ライブのフィルタ設定（プレビュー駆動）。ドラッグ中に更新される。
 * - `history` : 確定済み設定の Undo/Redo スタック。commit は pointerup 等の確定時。
 * この 2 つはドラッグ中のみ乖離し、commit / undo / redo で再同期する。
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  canRedo,
  canUndo,
  commit,
  type EditHistory,
  initHistory,
  redo,
  undo,
} from "../domain/EditHistory";
import {
  defaultSaveConfiguration,
  type SaveConfiguration,
} from "../domain/SaveConfiguration";
import { FileList } from "../features/asset-catalog/FileList";
import { useAssets } from "../features/asset-catalog/useAssets";
import { BatchRunner } from "../features/batch/BatchRunner";
import { useExport } from "../features/batch/useExport";
import { Canvas } from "../features/editor/Canvas";
import { FilterControls } from "../features/editor/FilterControls";
import { NamingControls } from "../features/editor/NamingControls";
import { usePreview } from "../features/editor/usePreview";
import type { FilterSettings } from "../ipc/types";
import { useKeybindings } from "../shared/keybindings";
import { redoHint, undoHint } from "../shared/platform";
import { type ThemeMode, useTheme } from "./providers/ThemeProvider";

const PREVIEW_MAX_DIM = 1600;
const INITIAL_SETTINGS: FilterSettings = { kind: "gaussian", radius: 8 };

export function App() {
  const { assets, errors, isDragging, pickFiles, clear, remove } = useAssets();
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [settings, setSettings] = useState<FilterSettings>(INITIAL_SETTINGS);
  const [history, setHistory] = useState<EditHistory<FilterSettings>>(() =>
    initHistory(INITIAL_SETTINGS),
  );
  const [saveConfig, setSaveConfig] = useState<SaveConfiguration>(
    defaultSaveConfiguration,
  );
  const exporter = useExport();
  const { mode, setMode } = useTheme();
  const { t, i18n } = useTranslation();

  // 実効言語を <html lang> とタイトルへ反映する（初期化直後・切替時とも）。
  useEffect(() => {
    document.documentElement.lang = i18n.resolvedLanguage ?? "ja";
    document.title = t("meta.title");
  }, [i18n.resolvedLanguage, t]);

  // キーバインド用に最新値を参照する（コールバックを安定化させる）。
  const historyRef = useRef(history);
  historyRef.current = history;
  const assetsRef = useRef(assets);
  assetsRef.current = assets;

  // 選択が範囲外になったら丸める。
  useEffect(() => {
    if (selectedIndex >= assets.length) {
      setSelectedIndex(Math.max(0, assets.length - 1));
    }
  }, [assets.length, selectedIndex]);

  const selected = assets[selectedIndex] ?? null;
  const preview = usePreview(selected?.path ?? null, settings, PREVIEW_MAX_DIM);

  // 確定値は引数で受ける。ref 経由の読みだと同一イベント内の onChange → onCommit で
  // 旧値を積んでしまう（種別変更が履歴に入らないバグの原因）。
  const commitSettings = useCallback((next: FilterSettings) => {
    setHistory((h) => commit(h, next));
  }, []);

  const applyHistory = useCallback((next: EditHistory<FilterSettings>) => {
    setHistory(next);
    setSettings(next.present);
  }, []);

  const doUndo = useCallback(
    () => applyHistory(undo(historyRef.current)),
    [applyHistory],
  );
  const doRedo = useCallback(
    () => applyHistory(redo(historyRef.current)),
    [applyHistory],
  );
  const doPrev = useCallback(
    () => setSelectedIndex((i) => Math.max(0, i - 1)),
    [],
  );
  const doNext = useCallback(
    () =>
      setSelectedIndex((i) => Math.min(assetsRef.current.length - 1, i + 1)),
    [],
  );

  useKeybindings({
    onUndo: doUndo,
    onRedo: doRedo,
    onPrev: doPrev,
    onNext: doNext,
  });

  const runExport = useCallback(() => {
    void exporter.run(
      assets.map((a) => a.path),
      settings,
      saveConfig,
    );
  }, [assets, settings, saveConfig, exporter]);

  return (
    <div className={`app ${isDragging ? "dragging" : ""}`}>
      <header className="app-header">
        <h1>Batch Blur</h1>
        <div className="header-actions">
          <button type="button" onClick={pickFiles}>
            {t("header.addImages")}
          </button>
          <button type="button" onClick={clear} disabled={assets.length === 0}>
            {t("header.clear")}
          </button>
          <span className="spacer" />
          <button
            type="button"
            onClick={doUndo}
            disabled={!canUndo(history)}
            title={undoHint}
          >
            {t("header.undo")}
          </button>
          <button
            type="button"
            onClick={doRedo}
            disabled={!canRedo(history)}
            title={redoHint}
          >
            {t("header.redo")}
          </button>
          <label className="theme-select">
            {t("header.theme")}
            <select
              value={mode}
              onChange={(e) => setMode(e.target.value as ThemeMode)}
            >
              <option value="system">{t("header.themeSystem")}</option>
              <option value="light">{t("header.themeLight")}</option>
              <option value="dark">{t("header.themeDark")}</option>
            </select>
          </label>
          <label className="theme-select">
            {t("header.language")}
            <select
              value={i18n.resolvedLanguage ?? "ja"}
              onChange={(e) => void i18n.changeLanguage(e.target.value)}
            >
              <option value="ja">日本語</option>
              <option value="en">English</option>
            </select>
          </label>
        </div>
      </header>

      <div className="app-body">
        <aside className="panel panel-list">
          <FileList
            assets={assets}
            errors={errors}
            selectedIndex={selectedIndex}
            saveConfig={saveConfig}
            onSelect={setSelectedIndex}
            onRemove={remove}
          />
        </aside>

        <main className="panel panel-canvas">
          <Canvas selected={selected} preview={preview} />
        </main>

        <aside className="panel panel-controls">
          <FilterControls
            settings={settings}
            onChange={setSettings}
            onCommit={commitSettings}
          />
          <NamingControls config={saveConfig} onChange={setSaveConfig} />
          <BatchRunner
            count={assets.length}
            state={exporter.state}
            onRun={runExport}
            onCancel={exporter.cancel}
          />
        </aside>
      </div>

      {isDragging && <div className="drop-overlay">{t("drop.overlay")}</div>}
    </div>
  );
}
