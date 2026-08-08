/**
 * 保存設定（上書き / 別名保存 + prefix・suffix + 出力先）。
 * prefix と suffix は独立に指定できる（「両方選べる」要件）。
 */

import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import type { SaveConfiguration } from "../../domain/SaveConfiguration";

interface Props {
  config: SaveConfiguration;
  onChange: (next: SaveConfiguration) => void;
}

export function NamingControls({ config, onChange }: Props) {
  const { t } = useTranslation();
  const pickOutDir = async () => {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") onChange({ ...config, outDir: dir });
  };

  return (
    <div className="controls">
      <div className="control-group">
        {/* biome-ignore lint/a11y/noLabelWithoutControl: 単一コントロールを持たないグループ見出し（各 radio は自身の label 内にネスト済み） */}
        <label>{t("naming.saveMethod")}</label>
        <div className="radio-row">
          <label>
            <input
              type="radio"
              name="save-kind"
              checked={config.kind === "overwrite"}
              onChange={() => onChange({ ...config, kind: "overwrite" })}
            />
            {t("naming.overwrite")}
          </label>
          <label>
            <input
              type="radio"
              name="save-kind"
              checked={config.kind === "saveAs"}
              onChange={() => onChange({ ...config, kind: "saveAs" })}
            />
            {t("naming.saveAs")}
          </label>
        </div>
      </div>

      {config.kind === "saveAs" && (
        <>
          <div className="control-group">
            <label htmlFor="naming-prefix">{t("naming.prefix")}</label>
            <input
              id="naming-prefix"
              type="text"
              value={config.prefix}
              placeholder={t("naming.none")}
              onChange={(e) => onChange({ ...config, prefix: e.target.value })}
            />
          </div>
          <div className="control-group">
            <label htmlFor="naming-suffix">{t("naming.suffix")}</label>
            <input
              id="naming-suffix"
              type="text"
              value={config.suffix}
              placeholder={t("naming.none")}
              onChange={(e) => onChange({ ...config, suffix: e.target.value })}
            />
          </div>
          <div className="control-group">
            {/* biome-ignore lint/a11y/noLabelWithoutControl: 単一コントロールを持たないグループ見出し */}
            <label>{t("naming.outDir")}</label>
            <div className="out-dir-row">
              <span className="out-dir" title={config.outDir ?? ""}>
                {config.outDir ?? t("naming.sameAsSource")}
              </span>
              <button type="button" onClick={pickOutDir}>
                {t("naming.select")}
              </button>
              {config.outDir && (
                <button
                  type="button"
                  onClick={() => onChange({ ...config, outDir: null })}
                >
                  {t("naming.clear")}
                </button>
              )}
            </div>
          </div>
        </>
      )}

      {config.kind === "overwrite" && (
        <p className="warn">{t("naming.overwriteWarning")}</p>
      )}

      {/* 上書き時は EXIF 等が不可逆に失われるため、保存方法によらず常に明示する。 */}
      <p className="hint">{t("naming.metadataNote")}</p>
    </div>
  );
}
