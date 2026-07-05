/**
 * 選択画像のブラー適用プレビューを取得する。
 *
 * - スライダー連打対策の debounce（~130ms）。
 * - reqId 単調増加で古い応答を破棄（in-flight が入れ替わっても最新だけ採用）。
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
  const [dataUrl, setDataUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reqCounter = useRef(0);
  const latestAccepted = useRef(0);

  useEffect(() => {
    if (!path) {
      setDataUrl(null);
      setError(null);
      return;
    }

    const timer = window.setTimeout(() => {
      const reqId = ++reqCounter.current;
      setLoading(true);
      generatePreview(path, settings, maxDim, reqId)
        .then((res) => {
          // 古い応答は破棄（最新の reqId のみ採用）。
          if (res.reqId >= latestAccepted.current) {
            latestAccepted.current = res.reqId;
            setDataUrl(res.dataUrl);
            setError(null);
          }
        })
        .catch((e) => setError(String(e)))
        .finally(() => setLoading(false));
    }, DEBOUNCE_MS);

    return () => window.clearTimeout(timer);
  }, [path, settings.kind, settings.radius, maxDim]);

  // 選択が変わった瞬間は前画像のプレビューを消す（誤表示防止）。
  useEffect(() => {
    setDataUrl(null);
  }, [path]);

  return { dataUrl, loading, error };
}
