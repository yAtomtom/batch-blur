/**
 * 一括書き出しの実行と進捗管理。
 * 進捗は Channel 経由で 1 ファイルごとに届く。失敗は生エラーを保持して表示する。
 * 部分失敗・キャンセルは ExportOutcome（正常応答）で受け、fatalError とは区別する。
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
  /** 失敗ファイルの (パス, エラー)。実行中は Channel 由来、完了後は outcome を正とする。 */
  failures: { path: string; error: string }[];
  /** ユーザー操作による中断。エラーではない。 */
  canceled: boolean;
  /** バッチ全体を開始できなかった場合のエラー（出力衝突・インフラ障害等）。 */
  fatalError: string | null;
  finished: boolean;
}

const initialState: ExportState = {
  running: false,
  done: 0,
  total: 0,
  currentPath: "",
  failures: [],
  canceled: false,
  fatalError: null,
  finished: false,
};

export function useExport() {
  const [state, setState] = useState<ExportState>(initialState);
  const failuresRef = useRef<{ path: string; error: string }[]>([]);
  // 実行世代トークン。完了後に遅延到着した progress や、再実行後に届く
  // 前回実行のイベントが state を汚染しないよう、一致する世代のみ反映する。
  const runToken = useRef(0);

  const run = useCallback(
    async (
      paths: string[],
      settings: FilterSettings,
      config: SaveConfiguration,
    ) => {
      if (paths.length === 0) return;
      const token = ++runToken.current;
      failuresRef.current = [];
      setState({ ...initialState, running: true, total: paths.length });

      const onProgress = (p: ExportProgress) => {
        if (runToken.current !== token) return;
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
        const outcome = await exportBatch(
          paths,
          settings,
          toSaveMode(config),
          config.jpegQuality,
          onProgress,
        );
        if (runToken.current !== token) return;
        // 完了以降は outcome を正とする（progress の到着順に依存しない）。
        // 世代を進め、これ以降に遅延到着する progress を無効化する。
        runToken.current += 1;
        setState((s) => ({
          ...s,
          running: false,
          finished: true,
          done: outcome.completed + outcome.failures.length,
          canceled: outcome.canceled,
          failures: outcome.failures,
        }));
      } catch (e) {
        if (runToken.current !== token) return;
        runToken.current += 1;
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
