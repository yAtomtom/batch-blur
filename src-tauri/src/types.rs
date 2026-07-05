//! IPC 境界で往復する型（フロントとの契約）。
//!
//! ドメイン型はコアに閉じ込め、ここは UI が扱いやすい素朴な形に射影する。
//! 各コマンドは「本来関心を持つ引数」だけを受け取る（時刻・ユーザー情報は持たない）。

use serde::{Deserialize, Serialize};

use crate::domain::filter::{AxisStrength, FilterKind, FilterSpec, FilterStack};

/// UI が送るフィルタ設定（単一種別＋単一半径, X/Y 同一）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterSettings {
    pub kind: FilterKind,
    pub radius: u32,
}

impl FilterSettings {
    /// ドメインの単一フィルタスタックへ変換する（不変条件はここで検証）。
    pub fn to_stack(&self) -> Result<FilterStack, String> {
        let strength = AxisStrength::uniform(self.radius)?;
        Ok(FilterStack::single(FilterSpec::whole(self.kind, strength)))
    }
}

/// 読み込んだ画像のメタ情報。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMeta {
    pub path: String,
    pub file_name: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

/// 読み込み結果（ファイル単位）。1 件失敗しても全体を潰さず、生エラーを行表示する。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum LoadResult {
    Ok { meta: ImageMeta },
    Error { path: String, error: String },
}

/// プレビュー結果。`req_id` を反響させ、フロント側で古い応答を破棄できるようにする。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    /// `data:image/png;base64,...` 形式。<img src> にそのまま入れられる。
    pub data_url: String,
    pub req_id: u64,
    pub preview_width: u32,
    pub preview_height: u32,
}

/// バッチ進捗（Tauri Channel で 1 ファイルごとに送る）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgress {
    pub done: u32,
    pub total: u32,
    pub current_path: String,
    /// このファイルのエラー。None は成功。
    pub error: Option<String>,
}
