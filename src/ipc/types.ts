/**
 * IPC 境界の型（Rust 側 types.rs / domain の serde 形と一致させる手書きミラー）。
 * 将来は tauri-specta による自動生成に置き換える候補。
 */

export type FilterKind = "gaussian" | "block";

export interface FilterSettings {
  kind: FilterKind;
  radius: number;
}

export interface ImageMeta {
  path: string;
  fileName: string;
  width: number;
  height: number;
  format: string;
}

export type LoadResult =
  | { status: "ok"; meta: ImageMeta }
  | { status: "error"; path: string; error: string };

export interface PreviewResult {
  dataUrl: string;
  reqId: number;
  previewWidth: number;
  previewHeight: number;
}

export interface ExportProgress {
  done: number;
  total: number;
  currentPath: string;
  error: string | null;
}

/** 保存モード（Rust domain::save::SaveMode と一致）。 */
export type SaveMode =
  | { mode: "overwrite" }
  | { mode: "saveAs"; prefix: string; suffix: string; outDir: string | null };
