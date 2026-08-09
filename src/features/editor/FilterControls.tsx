/**
 * フィルタ設定（種類 + 強度）。
 *
 * スライダーのドラッグ中は onChange でライブ更新（プレビュー反映）し、
 * pointerup 時に onCommit で履歴へ確定する（連続入力で履歴が膨張しないように）。
 */

import { useTranslation } from "react-i18next";
import type { FilterKind, FilterSettings } from "../../ipc/types";

/** UI 上の強度上限（ドメイン上限 500 より控えめに）。 */
export const MAX_RADIUS_UI = 100;

interface Props {
  settings: FilterSettings;
  onChange: (next: FilterSettings) => void;
  /** 確定値を引数で受ける（onChange の state 反映を待たずに履歴へ積めるように）。 */
  onCommit: (next: FilterSettings) => void;
}

export function FilterControls({ settings, onChange, onCommit }: Props) {
  const { t } = useTranslation();
  const setKind = (kind: FilterKind) => {
    const next = { ...settings, kind };
    onChange(next);
    onCommit(next); // 離散操作は即確定
  };

  const setRadius = (radius: number) => {
    onChange({ ...settings, radius });
  };

  return (
    <div className="controls">
      <div className="control-group">
        {/* biome-ignore lint/a11y/noLabelWithoutControl: 単一コントロールを持たないグループ見出し */}
        <label>{t("filter.kind")}</label>
        <div className="segmented">
          <button
            type="button"
            className={settings.kind === "gaussian" ? "active" : ""}
            onClick={() => setKind("gaussian")}
          >
            {t("filter.gaussian")}
          </button>
          <button
            type="button"
            className={settings.kind === "block" ? "active" : ""}
            onClick={() => setKind("block")}
          >
            {t("filter.block")}
          </button>
          <button
            type="button"
            className={settings.kind === "mosaic" ? "active" : ""}
            onClick={() => setKind("mosaic")}
          >
            {t("filter.mosaic")}
          </button>
        </div>
      </div>

      <div className="control-group">
        <label htmlFor="filter-radius">
          {t("filter.radius")} <strong>{settings.radius}</strong>
        </label>
        <input
          id="filter-radius"
          type="range"
          min={0}
          max={MAX_RADIUS_UI}
          step={1}
          value={settings.radius}
          onChange={(e) => setRadius(Number(e.target.value))}
          onPointerUp={() => onCommit(settings)}
          onKeyUp={() => onCommit(settings)}
        />
      </div>
    </div>
  );
}
