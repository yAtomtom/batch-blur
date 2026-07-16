//! Batch Blur バックエンド。
//!
//! 層構成:
//! - `domain`     … 純粋コア（フィルタ合成・保存ルール, image 非依存）
//! - `imaging`    … 純粋コーデック（bytes<->pixels）＋ブラーカーネル（image/imageproc 依存）
//! - `repository` … 画像ストレージのポート＋アダプタ（ローカルFS; 将来クラウド拡張）
//! - `commands`/`types` … Tauri IPC 境界（repository を DI して利用）

pub mod domain;
pub mod imaging;
pub mod repository;
pub mod types;

mod commands;

/// Tauri アプリを起動する。
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::load_images,
            commands::generate_preview,
            commands::export_batch,
            commands::cancel_export,
        ])
        .run(tauri::generate_context!())
        .expect("failed to launch the Tauri application");
}
