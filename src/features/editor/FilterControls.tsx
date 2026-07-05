/**
 * フィルタ設定（種類 + 強度）。
 *
 * スライダーのドラッグ中は onChange でライブ更新（プレビュー反映）し、
 * pointerup 時に onCommit で履歴へ確定する（連続入力で履歴が膨張しないように）。
 */

import type { FilterKind, FilterSettings } from "../../ipc/types";

/** UI 上の強度上限（ドメイン上限 500 より控えめに）。 */
export const MAX_RADIUS_UI = 100;

interface Props {
  settings: FilterSettings;
  onChange: (next: FilterSettings) => void;
  onCommit: () => void;
}

export function FilterControls({ settings, onChange, onCommit }: Props) {
  const setKind = (kind: FilterKind) => {
    onChange({ ...settings, kind });
    onCommit(); // 離散操作は即確定
  };

  const setRadius = (radius: number) => {
    onChange({ ...settings, radius });
  };

  return (
    <div className="controls">
      <div className="control-group">
        <label>フィルタ種類</label>
        <div className="segmented">
          <button
            className={settings.kind === "gaussian" ? "active" : ""}
            onClick={() => setKind("gaussian")}
          >
            ガウス
          </button>
          <button
            className={settings.kind === "block" ? "active" : ""}
            onClick={() => setKind("block")}
          >
            ブロック
          </button>
        </div>
      </div>

      <div className="control-group">
        <label>
          強さ（半径）: <strong>{settings.radius}</strong>
        </label>
        <input
          type="range"
          min={0}
          max={MAX_RADIUS_UI}
          step={1}
          value={settings.radius}
          onChange={(e) => setRadius(Number(e.target.value))}
          onPointerUp={onCommit}
          onKeyUp={onCommit}
        />
      </div>
    </div>
  );
}
