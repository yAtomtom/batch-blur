/**
 * 画像の読み込み（ダイアログ / ドラッグ&ドロップ）と一覧の保持。
 * ドラッグ&ドロップは Tauri ネイティブイベントを使う（HTML5 D&D は実パスを取れない）。
 */

import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { loadImages } from "../../ipc/commands";
import type { ImageMeta, LoadResult } from "../../ipc/types";

const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "webp", "bmp"];

export interface LoadError {
  path: string;
  error: string;
}

function hasImageExtension(path: string): boolean {
  const dot = path.lastIndexOf(".");
  if (dot < 0) return false;
  return IMAGE_EXTENSIONS.includes(path.slice(dot + 1).toLowerCase());
}

/** path をキーに既存エラーへ新規分を統合する（他 path の既存エラーは保持する）。 */
function mergeErrors(prev: LoadError[], next: LoadError[]): LoadError[] {
  const byPath = new Map(prev.map((e) => [e.path, e]));
  for (const e of next) byPath.set(e.path, e);
  return Array.from(byPath.values());
}

export interface UseAssets {
  assets: ImageMeta[];
  errors: LoadError[];
  isDragging: boolean;
  pickFiles: () => Promise<void>;
  clear: () => void;
  remove: (path: string) => void;
}

export function useAssets(): UseAssets {
  const { t } = useTranslation();
  const [assets, setAssets] = useState<ImageMeta[]>([]);
  const [errors, setErrors] = useState<LoadError[]>([]);
  const [isDragging, setIsDragging] = useState(false);

  const addPaths = useCallback(
    async (paths: string[]) => {
      // 対応形式外は黙って捨てず、エラー欄で通知する（フォルダのドロップ等）。
      const skipped = paths.filter((p) => !hasImageExtension(p));
      if (skipped.length > 0) {
        const message = t("fileList.unsupportedType");
        setErrors((prev) =>
          mergeErrors(
            prev,
            skipped.map((path) => ({ path, error: message })),
          ),
        );
      }

      const targets = paths.filter(hasImageExtension);
      if (targets.length === 0) return;

      // ファイル単位の失敗は results 内 Error 行で扱う。ここで reject するのは
      // 読み込み処理自体の失敗（Rust 側の join エラー等）＝raw に表示する。
      let results: LoadResult[];
      try {
        results = await loadImages(targets);
      } catch (e) {
        setErrors((prev) =>
          mergeErrors(prev, [{ path: targets.join(", "), error: String(e) }]),
        );
        return;
      }

      setAssets((prev) => {
        const byPath = new Map(prev.map((a) => [a.path, a]));
        for (const r of results) {
          if (r.status === "ok") byPath.set(r.meta.path, r.meta);
        }
        return Array.from(byPath.values());
      });
      setErrors((prev) => {
        const failed = results
          .filter(
            (r): r is Extract<typeof r, { status: "error" }> =>
              r.status === "error",
          )
          .map((r) => ({ path: r.path, error: r.error }));
        const okPaths = new Set(
          results
            .filter(
              (r): r is Extract<typeof r, { status: "ok" }> =>
                r.status === "ok",
            )
            .map((r) => r.meta.path),
        );
        // 今回読み込みに成功した path の残留エラーは解消する。
        return mergeErrors(prev, failed).filter((e) => !okPaths.has(e.path));
      });
    },
    [t],
  );

  const pickFiles = useCallback(async () => {
    const selected = await open({
      multiple: true,
      filters: [{ name: "Images", extensions: IMAGE_EXTENSIONS }],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    await addPaths(paths);
  }, [addPaths]);

  const clear = useCallback(() => {
    setAssets([]);
    setErrors([]);
  }, []);

  const remove = useCallback((path: string) => {
    setAssets((prev) => prev.filter((a) => a.path !== path));
    // 一覧から除外した path のエラー行も残さない。
    setErrors((prev) => prev.filter((e) => e.path !== path));
  }, []);

  // Tauri ネイティブのドラッグ&ドロップ購読。
  useEffect(() => {
    const unlistenPromise = getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "over" || payload.type === "enter") {
        setIsDragging(true);
      } else if (payload.type === "leave") {
        setIsDragging(false);
      } else if (payload.type === "drop") {
        setIsDragging(false);
        void addPaths(payload.paths);
      }
    });
    return () => {
      void unlistenPromise.then((un) => un());
    };
  }, [addPaths]);

  return { assets, errors, isDragging, pickFiles, clear, remove };
}
