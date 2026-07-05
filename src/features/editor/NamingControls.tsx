/**
 * 保存設定（上書き / 別名保存 + prefix・suffix + 出力先）。
 * prefix と suffix は独立に指定できる（「両方選べる」要件）。
 */

import { open } from "@tauri-apps/plugin-dialog";
import type { SaveConfiguration } from "../../domain/SaveConfiguration";

interface Props {
  config: SaveConfiguration;
  onChange: (next: SaveConfiguration) => void;
}

export function NamingControls({ config, onChange }: Props) {
  const pickOutDir = async () => {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") onChange({ ...config, outDir: dir });
  };

  return (
    <div className="controls">
      <div className="control-group">
        <label>保存方法</label>
        <div className="radio-row">
          <label>
            <input
              type="radio"
              name="save-kind"
              checked={config.kind === "overwrite"}
              onChange={() => onChange({ ...config, kind: "overwrite" })}
            />
            上書き保存
          </label>
          <label>
            <input
              type="radio"
              name="save-kind"
              checked={config.kind === "saveAs"}
              onChange={() => onChange({ ...config, kind: "saveAs" })}
            />
            別名で保存
          </label>
        </div>
      </div>

      {config.kind === "saveAs" && (
        <>
          <div className="control-group">
            <label>prefix（先頭に付与）</label>
            <input
              type="text"
              value={config.prefix}
              placeholder="（なし）"
              onChange={(e) => onChange({ ...config, prefix: e.target.value })}
            />
          </div>
          <div className="control-group">
            <label>suffix（末尾に付与）</label>
            <input
              type="text"
              value={config.suffix}
              placeholder="（なし）"
              onChange={(e) => onChange({ ...config, suffix: e.target.value })}
            />
          </div>
          <div className="control-group">
            <label>出力先</label>
            <div className="out-dir-row">
              <span className="out-dir" title={config.outDir ?? ""}>
                {config.outDir ?? "元ファイルと同じ場所"}
              </span>
              <button onClick={pickOutDir}>選択</button>
              {config.outDir && (
                <button onClick={() => onChange({ ...config, outDir: null })}>
                  クリア
                </button>
              )}
            </div>
          </div>
        </>
      )}

      {config.kind === "overwrite" && (
        <p className="warn">
          元ファイルを直接上書きします。元に戻せません。
        </p>
      )}
    </div>
  );
}
