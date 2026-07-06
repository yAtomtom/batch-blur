/**
 * プレビュー描画キャンバス（MVP は全面ブラーの結果表示のみ）。
 *
 * 将来のレイヤー/選択領域は Layer/・Selection/ に分離配置する。ここは
 * 「合成済み結果を表示する面」という単一の関心に閉じる。
 */

import { useTranslation } from "react-i18next";
import type { ImageMeta } from "../../ipc/types";
import type { PreviewState } from "./usePreview";
import { assetUrl } from "../../ipc/commands";

interface Props {
  selected: ImageMeta | null;
  preview: PreviewState;
}

export function Canvas({ selected, preview }: Props) {
  const { t } = useTranslation();
  if (!selected) {
    return (
      <div className="canvas empty">
        <p>{t("canvas.selectPrompt")}</p>
      </div>
    );
  }

  // ブラー済みプレビューがあればそれを、なければ元画像を表示。
  const src = preview.dataUrl ?? assetUrl(selected.path);

  return (
    <div className="canvas">
      <div className="canvas-viewport">
        <img className="preview-image" src={src} alt={selected.fileName} />
        {preview.loading && (
          <div className="preview-badge">{t("canvas.updating")}</div>
        )}
      </div>
      {preview.error && <div className="preview-error">{preview.error}</div>}
      <div className="canvas-caption">
        {selected.fileName} — {selected.width}×{selected.height}
      </div>
    </div>
  );
}
