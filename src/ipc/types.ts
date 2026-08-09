/**
 * IPC 境界の型（Rust 側 types.rs / domain の serde 形と一致させる手書きミラー）。
 * 将来は tauri-specta による自動生成に置き換える候補。
 */

export type FilterKind = "gaussian" | "block" | "mosaic";

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

/** 書き出しに失敗したファイル（生エラー付き）。 */
export interface ExportFailure {
  path: string;
  error: string;
}

/**
 * 一括書き出しの結果。部分失敗・キャンセルはエラーではなく結果として表す。
 * invoke が reject するのは前提検証（出力衝突等）とインフラ障害のみ。
 */
export interface ExportOutcome {
  completed: number;
  canceled: boolean;
  failures: ExportFailure[];
}

/** 保存モード（Rust domain::save::SaveMode と一致）。 */
export type SaveMode =
  | { mode: "overwrite" }
  | { mode: "saveAs"; prefix: string; suffix: string; outDir: string | null };
