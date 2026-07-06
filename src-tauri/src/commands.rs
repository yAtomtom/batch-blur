//! Tauri コマンド（IPC 境界の薄いアダプタ）。
//!
//! CPU/IO を伴う処理は `spawn_blocking` に載せ、async ランタイムと WebView を
//! 止めない。失敗は raw なエラー文字列を surface する（隠蔽 fallback しない）。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::{ImageFormat, RgbaImage};
use tauri::ipc::Channel;
use tauri::State;

use crate::domain::save::{self, SaveMode};
use crate::imaging::{blur, io};
use crate::types::{ExportProgress, FilterSettings, ImageMeta, LoadResult, PreviewResult};

/// プレビューの縮小ベースをキャッシュ（LRU(1)）。スライダー連打で再デコード/再縮小を避ける。
struct PreviewBase {
    path: PathBuf,
    max_dim: u32,
    base: RgbaImage,
    scale: f32,
}

/// アプリ状態。キャンセルフラグとプレビューキャッシュを共有する。
#[derive(Default)]
pub struct AppState {
    cancel: Arc<AtomicBool>,
    preview_cache: Arc<Mutex<Option<PreviewBase>>>,
}

/// 複数画像のメタ情報を読み込む（ヘッダのみ、フル デコードしない）。
///
/// ファイル単位の失敗は `LoadResult::Error` 行として返す（部分成功を許す）。
/// 一方 `spawn_blocking` の join 失敗（パニック等のインフラ障害）は空配列へ
/// 潰さず raw に surface する（隠蔽 fallback しない）。
#[tauri::command]
pub async fn load_images(paths: Vec<String>) -> Result<Vec<LoadResult>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .into_iter()
            .map(|p| {
                let path = PathBuf::from(&p);
                match load_meta(&path) {
                    Ok(meta) => LoadResult::Ok { meta },
                    Err(e) => LoadResult::Error { path: p, error: format!("{e:#}") },
                }
            })
            .collect()
    })
    .await
    .map_err(|e| format!("failed to load image metadata: {e}"))
}

fn load_meta(path: &Path) -> anyhow::Result<ImageMeta> {
    let (width, height) = image::image_dimensions(path)
        .with_context(|| format!("cannot get image dimensions: {}", path.display()))?;
    let format = io::format_from_path(path)?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string();
    Ok(ImageMeta {
        path: path.to_string_lossy().to_string(),
        file_name,
        width,
        height,
        format: format!("{format:?}"),
    })
}

/// 選択画像のブラー適用プレビューを生成する（縮小ベース＋半径スケール補正）。
#[tauri::command]
pub async fn generate_preview(
    path: String,
    settings: FilterSettings,
    max_dim: u32,
    req_id: u64,
    state: State<'_, AppState>,
) -> Result<PreviewResult, String> {
    let stack = settings.to_stack()?;
    let cache = state.preview_cache.clone();
    let path_buf = PathBuf::from(&path);

    tauri::async_runtime::spawn_blocking(move || -> Result<PreviewResult, String> {
        // 縮小ベースを取得（キャッシュヒットしなければデコード＋縮小して保存）。
        let (base, scale) = {
            let mut guard = cache.lock().map_err(|e| format!("failed to lock cache: {e}"))?;
            let hit = guard
                .as_ref()
                .is_some_and(|b| b.path == path_buf && b.max_dim == max_dim);
            if !hit {
                let loaded = io::load_rgba(&path_buf).map_err(|e| format!("{e:#}"))?;
                let (base, scale) = io::downscale_for_preview(&loaded.image, max_dim);
                *guard = Some(PreviewBase { path: path_buf.clone(), max_dim, base, scale });
            }
            let b = guard.as_ref().expect("set just above");
            (b.base.clone(), b.scale)
        };

        // 縮小ベースへ半径を scale 倍して適用（フルサイズと見た目を一致させる）。
        let blurred = blur::apply_stack_scaled(&base, &stack, scale);
        let png = io::encode_to_bytes(&blurred, ImageFormat::Png, 90).map_err(|e| format!("{e:#}"))?;
        let data_url = format!("data:image/png;base64,{}", BASE64.encode(&png));

        Ok(PreviewResult {
            data_url,
            req_id,
            preview_width: blurred.width(),
            preview_height: blurred.height(),
        })
    })
    .await
    .map_err(|e| format!("preview processing failed: {e}"))?
}

/// 一括書き出し（MVP は逐次）。進捗は Channel で 1 ファイルごとに送る。
#[tauri::command]
pub async fn export_batch(
    paths: Vec<String>,
    settings: FilterSettings,
    save: SaveMode,
    jpeg_quality: u8,
    on_progress: Channel<ExportProgress>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let stack = settings.to_stack()?;
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = state.cancel.clone();

    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let sources: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

        // 出力パスを先に解決し、衝突を検出（自動リネームによる隠蔽はしない）。
        let outputs: Vec<PathBuf> = sources
            .iter()
            .map(|s| save::resolve_output_path(s, &save))
            .collect::<Result<_, _>>()?;
        save::check_no_collisions(&outputs)?;

        // 別名保存で出力先が既存なら loud に停止（上書きは Overwrite モードでのみ意図的）。
        if matches!(save, SaveMode::SaveAs { .. }) {
            for out in &outputs {
                if out.exists() {
                    return Err(format!("output already exists: {}", out.display()));
                }
            }
        }

        let total = sources.len() as u32;
        let mut failures: Vec<String> = Vec::new();

        for (i, (src, out)) in sources.iter().zip(outputs.iter()).enumerate() {
            if cancel.load(Ordering::SeqCst) {
                return Err(format!("canceled ({}/{} completed)", i, total));
            }

            let result = (|| -> anyhow::Result<()> {
                let loaded = io::load_rgba(src)?;
                let blurred = blur::apply_stack(&loaded.image, &stack);
                let out_format = io::format_from_path(out)?;
                let bytes = io::encode_to_bytes(&blurred, out_format, jpeg_quality)?;
                io::write_atomic(out, &bytes)?;
                Ok(())
            })();

            let err = result.err().map(|e| format!("{e:#}"));
            if let Some(ref e) = err {
                failures.push(format!("{}: {}", src.display(), e));
            }
            let _ = on_progress.send(ExportProgress {
                done: (i as u32) + 1,
                total,
                current_path: src.to_string_lossy().to_string(),
                error: err,
            });
        }

        if failures.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} file(s) failed to export:\n{}",
                failures.len(),
                failures.join("\n")
            ))
        }
    })
    .await
    .map_err(|e| format!("export processing failed: {e}"))?
}

/// 実行中の一括書き出しをキャンセルする。
#[tauri::command]
pub fn cancel_export(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::SeqCst);
}
