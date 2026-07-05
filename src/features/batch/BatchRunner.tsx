/**
 * 一括書き出しの起動ボタン・進捗バー・結果（失敗の生エラー）表示。
 */

import type { ExportState } from "./useExport";

interface Props {
  count: number;
  state: ExportState;
  onRun: () => void;
  onCancel: () => void;
}

export function BatchRunner({ count, state, onRun, onCancel }: Props) {
  const pct = state.total > 0 ? Math.round((state.done / state.total) * 100) : 0;

  return (
    <div className="batch-runner">
      <div className="batch-actions">
        {state.running ? (
          <button className="danger" onClick={onCancel}>
            キャンセル
          </button>
        ) : (
          <button className="primary" onClick={onRun} disabled={count === 0}>
            {count} 件を一括保存
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

      {state.failures.length > 0 && (
        <div className="batch-failures">
          <div className="batch-failures-title">
            失敗 {state.failures.length} 件:
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
        state.failures.length === 0 && (
          <div className="batch-ok">✓ すべて保存しました（{state.total} 件）</div>
        )}
    </div>
  );
}
