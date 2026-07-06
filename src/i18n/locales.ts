/**
 * UI 文言の翻訳辞書（日本語 / 英語）。
 *
 * - キーは浅い論理プレフィックス（header / fileList / batch 等）で分類する。
 * - 補間はプレースホルダ変数（例: {{n}}）で表し、言語間で語順差を吸収する。
 *   複数形（plural）は現状要件になく、単数/複数を跨ぐ文言は件数を括弧・接尾で
 *   表現して回避している（KISS）。将来必要になれば i18next の plural に移行する。
 * - ブランド名 "Batch Blur"・技術語 prefix/suffix・ショートカット表記は非翻訳。
 * - 辞書はプレーンな TS オブジェクトとして公開し、キー/補間変数の網羅一致を
 *   純ロジックのテスト（locales.test.ts）で検証できるようにする。
 */

export const ja = {
  meta: {
    title: "Batch Blur — 一括ブラー",
  },
  header: {
    addImages: "画像を追加",
    clear: "クリア",
    undo: "↶ 戻す",
    redo: "↷ 進む",
    theme: "テーマ",
    themeSystem: "システム",
    themeLight: "ライト",
    themeDark: "ダーク",
    language: "言語",
  },
  drop: {
    overlay: "ここにドロップして追加",
  },
  fileList: {
    emptyHint:
      "画像をドラッグ&ドロップ、または「画像を追加」から選択してください。",
    removeFromList: "一覧から除外",
    savedFileName: "保存後のファイル名",
    loadFailed: "読み込み失敗:",
  },
  canvas: {
    selectPrompt: "プレビューする画像を選択してください。",
    updating: "更新中…",
  },
  filter: {
    kind: "フィルタ種類",
    gaussian: "ガウス",
    block: "ブロック",
    radius: "強さ（半径）:",
  },
  naming: {
    saveMethod: "保存方法",
    overwrite: "上書き保存",
    saveAs: "別名で保存",
    prefix: "prefix（先頭に付与）",
    suffix: "suffix（末尾に付与）",
    none: "（なし）",
    outDir: "出力先",
    sameAsSource: "元ファイルと同じ場所",
    select: "選択",
    clear: "クリア",
    overwriteWarning: "元ファイルを直接上書きします。元に戻せません。",
  },
  batch: {
    cancel: "キャンセル",
    run: "{{n}} 件を一括保存",
    failures: "失敗 {{n}} 件:",
    done: "✓ すべて保存しました（{{n}} 件）",
  },
} as const;

export const en = {
  meta: {
    title: "Batch Blur",
  },
  header: {
    addImages: "Add images",
    clear: "Clear",
    undo: "↶ Undo",
    redo: "↷ Redo",
    theme: "Theme",
    themeSystem: "System",
    themeLight: "Light",
    themeDark: "Dark",
    language: "Language",
  },
  drop: {
    overlay: "Drop here to add",
  },
  fileList: {
    emptyHint: 'Drag & drop images, or add them from "Add images".',
    removeFromList: "Remove from list",
    savedFileName: "Output file name",
    loadFailed: "Failed to load:",
  },
  canvas: {
    selectPrompt: "Select an image to preview.",
    updating: "Updating…",
  },
  filter: {
    kind: "Filter type",
    gaussian: "Gaussian",
    block: "Block",
    radius: "Strength (radius):",
  },
  naming: {
    saveMethod: "Save method",
    overwrite: "Overwrite",
    saveAs: "Save as",
    prefix: "prefix (prepend)",
    suffix: "suffix (append)",
    none: "(none)",
    outDir: "Output folder",
    sameAsSource: "Same folder as source",
    select: "Select",
    clear: "Clear",
    overwriteWarning:
      "Overwrites the original files directly. This cannot be undone.",
  },
  batch: {
    cancel: "Cancel",
    run: "Save all ({{n}})",
    failures: "{{n}} failed:",
    done: "✓ Saved all ({{n}})",
  },
} as const;
