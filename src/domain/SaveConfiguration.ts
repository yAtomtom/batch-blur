/**
 * 保存設定の UI 側モデルと SaveMode への射影。
 * prefix/suffix は独立（空文字は「付与しない」）。「両方選べる」要件に対応。
 */

import type { SaveMode } from "../ipc/types";

export type SaveKind = "overwrite" | "saveAs";

export interface SaveConfiguration {
  kind: SaveKind;
  prefix: string;
  suffix: string;
  outDir: string | null;
  jpegQuality: number;
}

export const defaultSaveConfiguration: SaveConfiguration = {
  kind: "saveAs",
  prefix: "",
  suffix: "_blur",
  outDir: null,
  jpegQuality: 90,
};

/** IPC の SaveMode へ変換する。 */
export function toSaveMode(cfg: SaveConfiguration): SaveMode {
  if (cfg.kind === "overwrite") return { mode: "overwrite" };
  return {
    mode: "saveAs",
    prefix: cfg.prefix,
    suffix: cfg.suffix,
    outDir: cfg.outDir,
  };
}

/** 保存後のファイル名プレビュー（別名保存時）。 */
export function previewOutputName(
  fileName: string,
  cfg: SaveConfiguration,
): string {
  if (cfg.kind === "overwrite") return fileName;
  const dot = fileName.lastIndexOf(".");
  const stem = dot >= 0 ? fileName.slice(0, dot) : fileName;
  const ext = dot >= 0 ? fileName.slice(dot) : "";
  return `${cfg.prefix}${stem}${cfg.suffix}${ext}`;
}
