import { describe, expect, it } from "vitest";
import { en, ja } from "./locales";

type Dict = { [key: string]: string | Dict };

/** ネストした辞書をドット区切りのフラットなキー→文言に変換する。 */
function flatten(dict: Dict, prefix = ""): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [k, v] of Object.entries(dict)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (typeof v === "string") out[key] = v;
    else Object.assign(out, flatten(v, key));
  }
  return out;
}

/** 文言中の補間プレースホルダ名（{{name}}）を昇順で抽出する。 */
function placeholders(text: string): string[] {
  return [...text.matchAll(/\{\{\s*(\w+)\s*\}\}/g)].map((m) => m[1]).sort();
}

const flatJa = flatten(ja as unknown as Dict);
const flatEn = flatten(en as unknown as Dict);

describe("locales", () => {
  it("ja と en のキー集合が完全一致する（欠落・余剰なし）", () => {
    expect(Object.keys(flatEn).sort()).toEqual(Object.keys(flatJa).sort());
  });

  it("同一キーの補間プレースホルダが ja / en で一致する", () => {
    for (const key of Object.keys(flatJa)) {
      expect(placeholders(flatEn[key] ?? "")).toEqual(
        placeholders(flatJa[key]),
      );
    }
  });
});
