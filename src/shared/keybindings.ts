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

export function useKeybindings(handlers: KeyHandlers): void {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;

      if (mod && e.key.toLowerCase() === "z") {
        if (isEditable(e.target)) return; // 入力欄の undo は横取りしない
        e.preventDefault();
        if (e.shiftKey) handlers.onRedo();
        else handlers.onUndo();
        return;
      }
      if (mod && e.key.toLowerCase() === "y") {
        if (isEditable(e.target)) return;
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
