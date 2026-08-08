/**
 * 一括書き出しの起動ボタン・進捗バー・結果（失敗の生エラー）表示。
 */

import { useTranslation } from "react-i18next";
import type { ExportState } from "./useExport";

interface Props {
  count: number;
  state: ExportState;
  onRun: () => void;
  onCancel: () => void;
}

export function BatchRunner({ count, state, onRun, onCancel }: Props) {
  const { t } = useTranslation();
  const pct = state.total > 0 ? Math.round((state.done / state.total) * 100) : 0;

  return (
    <div className="batch-runner">
      <div className="batch-actions">
        {state.running ? (
          <button className="danger" onClick={onCancel}>
            {t("batch.cancel")}
          </button>
        ) : (
          <button className="primary" onClick={onRun} disabled={count === 0}>
            {t("batch.run", { n: count })}
          </button>
        )}
      </div>

      {(state.running || state.finished) && (
        <div className="progress">
          <div className="progress-bar">
            <div className="progress-fill" style={{ width: `${pct}%` }} />
          </div>
          <div className="progress-label">
            {state.done} / {state.total}
            {state.currentPath && ` — ${state.currentPath}`}
          </div>
        </div>
      )}

      {state.fatalError && (
        <div className="batch-fatal">{state.fatalError}</div>
      )}

      {state.canceled && (
        <div className="batch-canceled">
          {t("batch.canceled", { done: state.done, total: state.total })}
        </div>
      )}

      {state.failures.length > 0 && (
        <div className="batch-failures">
          <div className="batch-failures-title">
            {t("batch.failures", { n: state.failures.length })}
          </div>
          <ul>
            {state.failures.map((f) => (
              <li key={f.path} className="error-row">
                <span className="file-name" title={f.path}>
                  {f.path}
                </span>
                <span className="error-detail">{f.error}</span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {state.finished &&
        !state.fatalError &&
        !state.canceled &&
        state.failures.length === 0 && (
          <div className="batch-ok">{t("batch.done", { n: state.total })}</div>
        )}
    </div>
  );
}
