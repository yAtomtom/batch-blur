/**
 * アプリ全体のオーケストレーション。
 *
 * 状態の要:
 * - `settings`: ライブのフィルタ設定（プレビュー駆動）。ドラッグ中に更新される。
 * - `history` : 確定済み設定の Undo/Redo スタック。commit は pointerup 等の確定時。
 * この 2 つはドラッグ中のみ乖離し、commit / undo / redo で再同期する。
 */

import { useCallback, useEffect, useRef, useState } from "react";
import { useAssets } from "../features/asset-catalog/useAssets";
import { FileList } from "../features/asset-catalog/FileList";
import { Canvas } from "../features/editor/Canvas";
import { FilterControls } from "../features/editor/FilterControls";
import { NamingControls } from "../features/editor/NamingControls";
import { usePreview } from "../features/editor/usePreview";
import { useExport } from "../features/batch/useExport";
import { BatchRunner } from "../features/batch/BatchRunner";
import { useKeybindings } from "../shared/keybindings";
import {
  canRedo,
  canUndo,
  commit,
  initHistory,
  redo,
  undo,
  type EditHistory,
} from "../domain/EditHistory";
import {
  defaultSaveConfiguration,
  type SaveConfiguration,
} from "../domain/SaveConfiguration";
import { useTheme, type ThemeMode } from "./providers/ThemeProvider";
import type { FilterSettings } from "../ipc/types";

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

  // キーバインド用に最新値を参照する（コールバックを安定化させる）。
  const settingsRef = useRef(settings);
  settingsRef.current = settings;
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

  const commitSettings = useCallback(() => {
    setHistory((h) => commit(h, settingsRef.current));
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
    () => setSelectedIndex((i) => Math.min(assetsRef.current.length - 1, i + 1)),
    [],
  );

  useKeybindings({ onUndo: doUndo, onRedo: doRedo, onPrev: doPrev, onNext: doNext });

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
          <button onClick={pickFiles}>画像を追加</button>
          <button onClick={clear} disabled={assets.length === 0}>
            クリア
          </button>
          <span className="spacer" />
          <button onClick={doUndo} disabled={!canUndo(history)} title="Ctrl+Z">
            ↶ 戻す
          </button>
          <button onClick={doRedo} disabled={!canRedo(history)} title="Ctrl+Y">
            ↷ 進む
          </button>
          <label className="theme-select">
            テーマ
            <select
              value={mode}
              onChange={(e) => setMode(e.target.value as ThemeMode)}
            >
              <option value="system">システム</option>
              <option value="light">ライト</option>
              <option value="dark">ダーク</option>
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

      {isDragging && (
        <div className="drop-overlay">ここにドロップして追加</div>
      )}
    </div>
  );
}
