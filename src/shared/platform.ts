/**
 * 実行プラットフォーム判定と、それに応じたキーボードショートカット表記。
 *
 * ショートカット記号（⌘ / Ctrl）は言語ではなくプラットフォーム依存のため i18n 対象外。
 * 機能自体は keybindings.ts が ctrlKey || metaKey で両対応済みで、ここは表示専用。
 */

/**
 * macOS 上で動作しているか。
 *
 * まず userAgentData.platform（新 API・利用可能なら "macOS"）を優先し、
 * 無い環境では userAgent 文字列にフォールバックする。UA 文字列単独依存は
 * 将来の仕様変更に脆いため多段判定にしている。
 */
export const isMac: boolean = (() => {
  const uaData = (navigator as Navigator & {
    userAgentData?: { platform?: string };
  }).userAgentData;
  if (uaData?.platform) return uaData.platform === "macOS";
  return navigator.userAgent.includes("Macintosh");
})();

/** Undo のショートカット表記（mac: ⌘Z / それ以外: Ctrl+Z）。 */
export const undoHint: string = isMac ? "⌘Z" : "Ctrl+Z";

/** Redo のショートカット表記（mac: ⌘⇧Z / それ以外: Ctrl+Y）。 */
export const redoHint: string = isMac ? "⌘⇧Z" : "Ctrl+Y";
