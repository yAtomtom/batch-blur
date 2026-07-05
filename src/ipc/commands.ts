/**
 * Tauri コマンドの型付きラッパ（IPC 境界）。
 * 引数は camelCase で渡し、Tauri が Rust の snake_case へ変換する。
 */

import { invoke, Channel, convertFileSrc } from "@tauri-apps/api/core";
import type {
  ExportProgress,
  FilterSettings,
  LoadResult,
  PreviewResult,
  SaveMode,
} from "./types";

export function loadImages(paths: string[]): Promise<LoadResult[]> {
  return invoke<LoadResult[]>("load_images", { paths });
}

export function generatePreview(
  path: string,
  settings: FilterSettings,
  maxDim: number,
  reqId: number,
): Promise<PreviewResult> {
  return invoke<PreviewResult>("generate_preview", {
    path,
    settings,
    maxDim,
    reqId,
  });
}

export function exportBatch(
  paths: string[],
  settings: FilterSettings,
  save: SaveMode,
  jpegQuality: number,
  onProgress: (p: ExportProgress) => void,
): Promise<void> {
  const channel = new Channel<ExportProgress>();
  channel.onmessage = onProgress;
  return invoke<void>("export_batch", {
    paths,
    settings,
    save,
    jpegQuality,
    onProgress: channel,
  });
}

export function cancelExport(): Promise<void> {
  return invoke<void>("cancel_export");
}

/** 元画像（未加工）を <img> で表示するためのアセット URL。 */
export function assetUrl(path: string): string {
  return convertFileSrc(path);
}
