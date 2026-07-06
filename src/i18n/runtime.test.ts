import { describe, it, expect } from "vitest";
import { createInstance } from "i18next";
import { ja, en } from "./locales";

async function make(lng: string) {
  const i = createInstance();
  await i.init({
    lng,
    resources: { ja: { translation: ja }, en: { translation: en } },
    fallbackLng: "ja",
    interpolation: { escapeValue: false },
  });
  return i;
}

describe("i18next runtime resolution", () => {
  it("interpolates {{n}} without plural forms (ja/en)", async () => {
    const jaI = await make("ja");
    const enI = await make("en");
    expect(jaI.t("batch.run", { n: 3 })).toBe("3 件を一括保存");
    expect(enI.t("batch.run", { n: 3 })).toBe("Save all (3)");
    expect(jaI.t("batch.done", { n: 1 })).toBe("✓ すべて保存しました（1 件）");
    expect(enI.t("batch.done", { n: 1 })).toBe("✓ Saved all (1)");
  });

  it("resolves plain keys and falls back missing lang to ja", async () => {
    const enI = await make("en");
    expect(enI.t("header.addImages")).toBe("Add images");
    const frI = await make("fr");
    expect(frI.t("header.addImages")).toBe("画像を追加");
  });
});
