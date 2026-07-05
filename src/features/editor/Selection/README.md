# Selection（将来拡張シーム）

フィルタ適用領域（選択領域）の配置場所。MVP では未実装で、常に全面（`Region::Whole`）
に射影される。

将来ここに追加するもの:
- `SelectionOverlay.tsx` … 矩形/マスクの描画・編集 UI
- `useSelection.ts` … 選択領域の状態

ドメイン側の対応: `Region` enum（現状 `Whole` のみ）に `Rect`/`Mask` を追加し、
imaging の適用ループは既に `Region::contains` を通すため、ループ構造を変えずに拡張できる。
