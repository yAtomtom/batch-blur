/**
 * 操作履歴のイミュータブルなスナップショットスタック。
 *
 * past / present / future の三つ組で Undo(Ctrl+Z)/Redo(Ctrl+Y) を表す。
 * コマンドパターンではなくスナップショットを採用（対象が小さく既にイミュータブル）。
 * 不変条件: present は常に存在。`redo(undo(h)) === h`（Ctrl+Z を redo でキャンセル可能）。
 */

/** 履歴の上限。メモリ膨張を防ぐ。 */
export const HISTORY_LIMIT = 50;

export interface EditHistory<T> {
  readonly past: readonly T[];
  readonly present: T;
  readonly future: readonly T[];
}

export function initHistory<T>(present: T): EditHistory<T> {
  return { past: [], present, future: [] };
}

/** 構造的等価（小さな設定オブジェクト前提の素朴比較）。 */
function equals<T>(a: T, b: T): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * 新しい状態を確定する。現在値と等しければ何もしない（連続入力での重複を防ぐ）。
 * 事後条件: future は空（新編集で redo 枝を破棄）、past は上限で切り詰め。
 */
export function commit<T>(h: EditHistory<T>, next: T): EditHistory<T> {
  if (equals(next, h.present)) return h;
  const past = [...h.past, h.present].slice(-HISTORY_LIMIT);
  return { past, present: next, future: [] };
}

/** 一つ戻す。past が空なら境界 no-op。 */
export function undo<T>(h: EditHistory<T>): EditHistory<T> {
  if (h.past.length === 0) return h;
  const present = h.past[h.past.length - 1];
  return {
    past: h.past.slice(0, -1),
    present,
    future: [h.present, ...h.future],
  };
}

/** 一つ進む。future が空なら境界 no-op。 */
export function redo<T>(h: EditHistory<T>): EditHistory<T> {
  if (h.future.length === 0) return h;
  const present = h.future[0];
  return {
    past: [...h.past, h.present],
    present,
    future: h.future.slice(1),
  };
}

export function canUndo<T>(h: EditHistory<T>): boolean {
  return h.past.length > 0;
}

export function canRedo<T>(h: EditHistory<T>): boolean {
  return h.future.length > 0;
}
