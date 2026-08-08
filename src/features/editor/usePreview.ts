/**
 * 選択画像のブラー適用プレビューを取得する。
 *
 * - スライダー連打対策の debounce（~130ms）。
 * - 応答の採否は effect クロージャの stale フラグで判定する。依存
 *   （path / settings / maxDim）が変わるたび cleanup が旧リクエストの
 *   then / catch / finally を無効化するため、state に触れるのは常に
 *   「現在の選択 × 現在の設定」に対する最新リクエストの結果だけになる
 *   （前画像の応答が後着して誤表示される競合を防ぐ）。
 * - Rust が唯一のブラー実装なので、プレビューと書き出しの見た目が一致する。
 */

import { useEffect, useRef, useState } from "react";
import { generatePreview } from "../../ipc/commands";
import type { FilterSettings } from "../../ipc/types";

const DEBOUNCE_MS = 130;

export interface PreviewState {
  dataUrl: string | null;
  loading: boolean;
  error: string | null;
}

export function usePreview(
  path: string | null,
  settings: FilterSettings,
  maxDim: number,
): PreviewState {
  // どの path に対する結果かを持たせ、返却時に現在の path と突き合わせる。
  // effect での消去（paint 後 ＝ 1 フレーム旧画像が見える）に依存しない。
  const [result, setResult] = useState<{
    path: string;
    dataUrl: string;
  } | null>(null);
  const [failure, setFailure] = useState<{
    path: string;
    message: string;
  } | null>(null);
  const [loading, setLoading] = useState(false);

  // Rust 側へエコーバック用に渡す一意 ID（採否判定には使わない）。
  const reqCounter = useRef(0);

  // biome-ignore lint/correctness/useExhaustiveDependencies: settings はレンダー毎に別オブジェクトになり得るため、依存はオブジェクトでなく値（kind/radius）で指定し debounce の再実行を制御する（冒頭コメント参照）
  useEffect(() => {
    if (!path) {
      setResult(null);
      setFailure(null);
      setLoading(false);
      return;
    }

    let stale = false;
    const timer = window.setTimeout(() => {
      setLoading(true);
      generatePreview(path, settings, maxDim, ++reqCounter.current)
        .then((res) => {
          if (stale) return;
          setResult({ path, dataUrl: res.dataUrl });
          setFailure(null);
        })
        .catch((e) => {
          if (!stale) setFailure({ path, message: String(e) });
        })
        .finally(() => {
          if (!stale) setLoading(false);
        });
    }, DEBOUNCE_MS);

    return () => {
      stale = true;
      window.clearTimeout(timer);
    };
  }, [path, settings.kind, settings.radius, maxDim]);

  // レンダー時に path 一致を検査する ＝ 選択切替の同一フレームから旧画像を出さない。
  // 設定変更のみ（path 同一）の間は前回結果を出し続け、ちらつきを避ける。
  return {
    dataUrl: result && result.path === path ? result.dataUrl : null,
    loading,
    error: failure && failure.path === path ? failure.message : null,
  };
}
