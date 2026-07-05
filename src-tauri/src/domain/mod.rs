//! 純粋コアドメイン。image クレート・ファイルシステム・Tauri に非依存で、
//! 単体テストのみで完結する。アプリの価値（フィルタ合成と保存ルール）を隔離する。

pub mod filter;
pub mod preset;
pub mod save;
