//! フィルタのコアドメイン（純粋・image クレート非依存）。
//!
//! ここはアプリの価値が集約する「合成可能なフィルタパイプライン」の型定義。
//! ピクセル演算そのものは imaging アダプタが担い、本モジュールは「何を適用するか」
//! を表すイミュータブルな値オブジェクトと不変条件のみを持つ。

use serde::{Deserialize, Serialize};

/// 強度（半径）の上限。事前条件の検証に用いる。
pub const MAX_RADIUS: u32 = 500;

/// フィルタ種別。強度（半径）の解釈だけが異なる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterKind {
    /// ガウスブラー（sigma = radius / 2）
    Gaussian,
    /// ブロック（box）ブラー（window = 2*radius + 1）
    Block,
    /// モザイク（ピクセレート, block = radius + 1）
    Mosaic,
}

/// X/Y 軸ごとの強度（半径）。
///
/// 将来の「X/Y 別強度」拡張シームを今のうちに型で確保している。
/// MVP 期間の不変条件: `x == y`（[`AxisStrength::uniform`] でのみ生成する）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AxisStrength {
    x: u32,
    y: u32,
}

impl AxisStrength {
    /// X/Y 同一強度で生成する（MVP の唯一の生成経路）。
    ///
    /// 事前条件: `radius <= MAX_RADIUS`。範囲外はエラー（クランプによる隠蔽はしない）。
    /// 事後条件: `is_uniform()` が真。
    pub fn uniform(radius: u32) -> Result<Self, String> {
        if radius > MAX_RADIUS {
            return Err(format!(
                "strength (radius) {radius} exceeds the maximum {MAX_RADIUS}"
            ));
        }
        Ok(Self {
            x: radius,
            y: radius,
        })
    }

    pub fn x(&self) -> u32 {
        self.x
    }

    pub fn y(&self) -> u32 {
        self.y
    }

    /// MVP 不変条件のチェック用。
    pub fn is_uniform(&self) -> bool {
        self.x == self.y
    }
}

/// フィルタを適用する領域。
///
/// MVP は全面（[`Region::Whole`]）のみ。将来 `Rect`/`Mask` を追加する選択領域シーム。
/// 未実装シームは必ず `Whole` に射影される（分岐の早期分散を防ぐ）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Region {
    Whole,
}

impl Region {
    /// 座標 `(x, y)`（画像サイズ `dims`）が領域に含まれるか。
    ///
    /// 事後条件: `Whole` は常に真。imaging 側の適用ループが最初からこの関数を通すことで、
    /// 将来領域を追加してもループ構造を変えずに済む。
    pub fn contains(&self, _x: u32, _y: u32, _dims: (u32, u32)) -> bool {
        match self {
            Region::Whole => true,
        }
    }
}

/// 1 つのフィルタ適用の指定（イミュータブルな値オブジェクト）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterSpec {
    pub kind: FilterKind,
    pub strength: AxisStrength,
    pub region: Region,
}

impl FilterSpec {
    /// 全面適用の 1 フィルタを生成する（MVP の主経路）。
    pub fn whole(kind: FilterKind, strength: AxisStrength) -> Self {
        Self {
            kind,
            strength,
            region: Region::Whole,
        }
    }
}

/// フィルタの重ね掛けスタック。
///
/// 「複数フィルタが独立に適用される」将来要件のシーム。適用は順序を保持した fold。
/// MVP の UI は長さ 1 に固定するが、ドメインは可変長を最初から表現できる。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterStack(Vec<FilterSpec>);

impl FilterStack {
    pub fn new(specs: Vec<FilterSpec>) -> Self {
        Self(specs)
    }

    /// 単一フィルタのスタック（MVP 主経路）。
    pub fn single(spec: FilterSpec) -> Self {
        Self(vec![spec])
    }

    /// 適用順に並んだフィルタ列。順序は load-bearing（ブラーは非可換）。
    pub fn specs(&self) -> &[FilterSpec] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_sets_both_axes_equal() {
        let s = AxisStrength::uniform(8).unwrap();
        assert_eq!(s.x(), 8);
        assert_eq!(s.y(), 8);
        assert!(s.is_uniform());
    }

    #[test]
    fn uniform_rejects_over_max() {
        let err = AxisStrength::uniform(MAX_RADIUS + 1).unwrap_err();
        assert!(err.contains("exceeds"));
    }

    #[test]
    fn uniform_allows_zero_and_max_boundaries() {
        assert!(AxisStrength::uniform(0).is_ok());
        assert!(AxisStrength::uniform(MAX_RADIUS).is_ok());
    }

    #[test]
    fn whole_region_contains_any_point() {
        let r = Region::Whole;
        assert!(r.contains(0, 0, (100, 100)));
        assert!(r.contains(999, 999, (100, 100)));
    }

    #[test]
    fn stack_preserves_order() {
        let a = FilterSpec::whole(FilterKind::Gaussian, AxisStrength::uniform(1).unwrap());
        let b = FilterSpec::whole(FilterKind::Block, AxisStrength::uniform(2).unwrap());
        let stack = FilterStack::new(vec![a.clone(), b.clone()]);
        assert_eq!(stack.specs(), &[a, b]);
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn kind_serde_is_lowercase() {
        let json = serde_json::to_string(&FilterKind::Gaussian).unwrap();
        assert_eq!(json, "\"gaussian\"");
        let k: FilterKind = serde_json::from_str("\"block\"").unwrap();
        assert_eq!(k, FilterKind::Block);
        let json = serde_json::to_string(&FilterKind::Mosaic).unwrap();
        assert_eq!(json, "\"mosaic\"");
        let k: FilterKind = serde_json::from_str("\"mosaic\"").unwrap();
        assert_eq!(k, FilterKind::Mosaic);
    }
}
