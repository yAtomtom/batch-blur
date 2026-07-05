import { describe, it, expect } from "vitest";
import {
  initHistory,
  commit,
  undo,
  redo,
  canUndo,
  canRedo,
  HISTORY_LIMIT,
} from "./EditHistory";

interface Settings {
  kind: "gaussian" | "block";
  radius: number;
}

const s = (radius: number, kind: Settings["kind"] = "gaussian"): Settings => ({
  kind,
  radius,
});

describe("EditHistory", () => {
  it("commit moves present and clears redo branch", () => {
    let h = initHistory(s(0));
    h = commit(h, s(5));
    expect(h.present).toEqual(s(5));
    expect(h.past).toEqual([s(0)]);
    expect(h.future).toEqual([]);
  });

  it("commit of an equal value is a no-op", () => {
    let h = initHistory(s(5));
    const same = commit(h, s(5));
    expect(same).toBe(h);
  });

  it("undo/redo walk the stack", () => {
    let h = initHistory(s(0));
    h = commit(h, s(3));
    h = commit(h, s(7));
    h = undo(h);
    expect(h.present).toEqual(s(3));
    h = redo(h);
    expect(h.present).toEqual(s(7));
  });

  it("redo(undo(h)) === h (Ctrl+Z is cancelable)", () => {
    let h = initHistory(s(0));
    h = commit(h, s(4));
    const back = redo(undo(h));
    expect(back).toEqual(h);
  });

  it("new commit after undo discards the redo branch", () => {
    let h = initHistory(s(0));
    h = commit(h, s(3));
    h = undo(h); // present = s(0), future = [s(3)]
    h = commit(h, s(9));
    expect(h.present).toEqual(s(9));
    expect(canRedo(h)).toBe(false);
  });

  it("undo at boundary is a no-op", () => {
    const h = initHistory(s(0));
    expect(undo(h)).toBe(h);
    expect(canUndo(h)).toBe(false);
  });

  it("history is capped at HISTORY_LIMIT", () => {
    let h = initHistory(s(0));
    for (let i = 1; i <= HISTORY_LIMIT + 10; i++) {
      h = commit(h, s(i));
    }
    expect(h.past.length).toBe(HISTORY_LIMIT);
  });
});
