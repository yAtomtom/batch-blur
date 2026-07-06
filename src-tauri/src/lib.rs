//! Batch Blur バックエンド。
//!
//! 層構成:
//! - `domain`  … 純粋コア（フィルタ合成・保存ルール, image 非依存）
//! - `imaging` … アダプタ（image/imageproc による実ピクセル・実ファイル）
//! - `commands`/`types` … Tauri IPC 境界

pub mod domain;
pub mod imaging;
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
