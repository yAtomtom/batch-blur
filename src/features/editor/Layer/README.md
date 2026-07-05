# Layer（将来拡張シーム）

複数レイヤー対応の配置場所。MVP では未実装で、常に「単一レイヤー（＝キャンバス全体）」
に射影される（`Canvas.tsx` が結果面を担当）。

将来ここに追加するもの:
- `LayerList.tsx` … レイヤーの表示/並べ替え/表示切替
- `useLayers.ts` … レイヤー状態（各レイヤーは独立した `FilterStack` を持てる）

ドメイン側の対応: `FilterStack`（重ね掛け）は既に存在。レイヤーごとに `FilterStack`
を割り当てる形へ拡張する。Canvas/Layer/Selection は互いに独立させる（関心の分離）。
