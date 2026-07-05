/**
 * 読み込んだファイルの一覧。選択でプレビュー対象を切り替える。
 * 矢印キーでの移動は上位（キーバインド）から selectedIndex を更新して実現。
 */

import type { ImageMeta } from "../../ipc/types";
import type { LoadError } from "./useAssets";
import type { SaveConfiguration } from "../../domain/SaveConfiguration";
import { previewOutputName } from "../../domain/SaveConfiguration";

interface Props {
  assets: ImageMeta[];
  errors: LoadError[];
  selectedIndex: number;
  saveConfig: SaveConfiguration;
  onSelect: (index: number) => void;
  onRemove: (path: string) => void;
}

export function FileList({
  assets,
  errors,
  selectedIndex,
  saveConfig,
  onSelect,
  onRemove,
}: Props) {
  return (
    <div className="file-list">
      {assets.length === 0 && errors.length === 0 && (
        <p className="hint">
          画像をドラッグ&ドロップ、または「画像を追加」から選択してください。
        </p>
      )}

      <ul>
        {assets.map((a, i) => (
          <li
            key={a.path}
            className={i === selectedIndex ? "selected" : ""}
            onClick={() => onSelect(i)}
          >
            <div className="file-row">
              <span className="file-name" title={a.path}>
                {a.fileName}
              </span>
              <button
                className="remove"
                title="一覧から除外"
                onClick={(e) => {
                  e.stopPropagation();
                  onRemove(a.path);
                }}
              >
                ×
              </button>
            </div>
            <div className="file-meta">
              {a.width}×{a.height} · {a.format}
            </div>
            {saveConfig.kind === "saveAs" && (
              <div className="file-out" title="保存後のファイル名">
                → {previewOutputName(a.fileName, saveConfig)}
              </div>
            )}
          </li>
        ))}
      </ul>

      {errors.length > 0 && (
        <div className="load-errors">
          <div className="load-errors-title">読み込み失敗:</div>
          <ul>
            {errors.map((e) => (
              <li key={e.path} className="error-row">
                <span className="file-name" title={e.path}>
                  {e.path}
                </span>
                <span className="error-detail">{e.error}</span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
