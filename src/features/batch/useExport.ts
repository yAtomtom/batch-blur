/**
 * 一括書き出しの実行と進捗管理。
 * 進捗は Channel 経由で 1 ファイルごとに届く。失敗は生エラーを保持して表示する。
 */

import { useCallback, useRef, useState } from "react";
import { cancelExport, exportBatch } from "../../ipc/commands";
import type { ExportProgress, FilterSettings } from "../../ipc/types";
import { toSaveMode, type SaveConfiguration } from "../../domain/SaveConfiguration";

export interface ExportState {
  running: boolean;
  done: number;
  total: number;
  currentPath: string;
  /** 失敗ファイルの (パス, エラー)。 */
  failures: { path: string; error: string }[];
  /** バッチ全体のエラー（衝突・キャンセル等）。 */
  fatalError: string | null;
  finished: boolean;
}

const initialState: ExportState = {
  running: false,
  done: 0,
  total: 0,
  currentPath: "",
  failures: [],
  fatalError: null,
  finished: false,
};

export function useExport() {
  const [state, setState] = useState<ExportState>(initialState);
  const failuresRef = useRef<{ path: string; error: string }[]>([]);

  const run = useCallback(
    async (
      paths: string[],
      settings: FilterSettings,
      config: SaveConfiguration,
    ) => {
      if (paths.length === 0) return;
      failuresRef.current = [];
      setState({ ...initialState, running: true, total: paths.length });

      const onProgress = (p: ExportProgress) => {
        if (p.error) {
          failuresRef.current = [
            ...failuresRef.current,
            { path: p.currentPath, error: p.error },
          ];
        }
        setState((s) => ({
          ...s,
          done: p.done,
          total: p.total,
          currentPath: p.currentPath,
          failures: failuresRef.current,
        }));
      };

      try {
        await exportBatch(
          paths,
          settings,
          toSaveMode(config),
          config.jpegQuality,
          onProgress,
        );
        setState((s) => ({ ...s, running: false, finished: true }));
      } catch (e) {
        setState((s) => ({
          ...s,
          running: false,
          finished: true,
          fatalError: String(e),
        }));
      }
    },
    [],
  );

  const cancel = useCallback(() => {
    void cancelExport();
  }, []);

  const reset = useCallback(() => setState(initialState), []);

  return { state, run, cancel, reset };
}
