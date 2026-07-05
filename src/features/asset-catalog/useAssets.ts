/**
 * 画像の読み込み（ダイアログ / ドラッグ&ドロップ）と一覧の保持。
 * ドラッグ&ドロップは Tauri ネイティブイベントを使う（HTML5 D&D は実パスを取れない）。
 */

import { useCallback, useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { loadImages } from "../../ipc/commands";
import type { ImageMeta } from "../../ipc/types";

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

export interface UseAssets {
  assets: ImageMeta[];
  errors: LoadError[];
  isDragging: boolean;
  pickFiles: () => Promise<void>;
  clear: () => void;
  remove: (path: string) => void;
}

export function useAssets(): UseAssets {
  const [assets, setAssets] = useState<ImageMeta[]>([]);
  const [errors, setErrors] = useState<LoadError[]>([]);
  const [isDragging, setIsDragging] = useState(false);

  const addPaths = useCallback(async (paths: string[]) => {
    const targets = paths.filter(hasImageExtension);
    if (targets.length === 0) return;

    // ファイル単位の失敗は results 内 Error 行で扱う。ここで reject するのは
    // 読み込み処理自体の失敗（Rust 側の join エラー等）＝raw に表示する。
    let results;
    try {
      results = await loadImages(targets);
    } catch (e) {
      setErrors([{ path: targets.join(", "), error: String(e) }]);
      return;
    }

    setAssets((prev) => {
      const byPath = new Map(prev.map((a) => [a.path, a]));
      for (const r of results) {
        if (r.status === "ok") byPath.set(r.meta.path, r.meta);
      }
      return Array.from(byPath.values());
    });
    setErrors(
      results
        .filter((r): r is Extract<typeof r, { status: "error" }> => r.status === "error")
        .map((r) => ({ path: r.path, error: r.error })),
    );
  }, []);

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
