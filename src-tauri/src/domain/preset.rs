//! フィルタプリセット（将来のプリセット入出力シーム）。
//!
//! `FilterStack` が既に serde 値オブジェクトなので、プリセットの入出力は JSON の
//! read/write に還元される。MVP では型のみ用意し、export/import コマンドは将来追加する。

use serde::{Deserialize, Serialize};

use super::filter::FilterStack;

/// 名前付きのフィルタ設定。JSON で保存・読み込みできる。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterPreset {
    pub name: String,
    pub stack: FilterStack,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::filter::{AxisStrength, FilterKind, FilterSpec};

    #[test]
    fn preset_roundtrips_through_json() {
        let preset = FilterPreset {
            name: "soft".into(),
            stack: FilterStack::single(FilterSpec::whole(
                FilterKind::Gaussian,
                AxisStrength::uniform(5).unwrap(),
            )),
        };
        let json = serde_json::to_string(&preset).unwrap();
        let back: FilterPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(preset, back);
    }
}
