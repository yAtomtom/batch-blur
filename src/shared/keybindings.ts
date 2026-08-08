/**
 * グローバルキーバインド。
 * - Ctrl/Cmd+Z: Undo, Ctrl/Cmd+Y または Ctrl/Cmd+Shift+Z: Redo
 * - ArrowUp/ArrowDown: ファイル一覧の選択移動
 *
 * テキスト入力にフォーカスがある場合は既定動作を優先し、横取りしない。
 */

import { useEffect } from "react";

export interface KeyHandlers {
  onUndo: () => void;
  onRedo: () => void;
  onPrev: () => void;
  onNext: () => void;
}

function isEditable(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || target.isContentEditable;
}

/** ネイティブのテキスト undo を持つ input タイプ。 */
const TEXT_INPUT_TYPES = new Set([
  "text",
  "search",
  "url",
  "tel",
  "email",
  "password",
  "number",
]);

/**
 * テキスト編集対象か。range / checkbox 等はテキスト undo を持たないため、
 * フォーカス中でもアプリの Undo/Redo を横取りしてよい（スライダー操作直後に
 * Ctrl+Z が効かなくなる問題の回避）。
 */
function isTextEditable(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.tagName === "TEXTAREA" || target.isContentEditable) return true;
  return target instanceof HTMLInputElement && TEXT_INPUT_TYPES.has(target.type);
}

export function useKeybindings(handlers: KeyHandlers): void {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;

      if (mod && e.key.toLowerCase() === "z") {
        if (isTextEditable(e.target)) return; // テキスト入力欄の undo は横取りしない
        e.preventDefault();
        if (e.shiftKey) handlers.onRedo();
        else handlers.onUndo();
        return;
      }
      if (mod && e.key.toLowerCase() === "y") {
        if (isTextEditable(e.target)) return;
        e.preventDefault();
        handlers.onRedo();
        return;
      }
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        if (isEditable(e.target)) return; // スライダー等の操作を邪魔しない
        e.preventDefault();
        if (e.key === "ArrowDown") handlers.onNext();
        else handlers.onPrev();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [handlers.onUndo, handlers.onRedo, handlers.onPrev, handlers.onNext]);
}
