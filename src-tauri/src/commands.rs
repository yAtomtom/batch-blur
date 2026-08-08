//! Tauri コマンド（IPC 境界の薄いアダプタ）。
//!
//! CPU/IO を伴う処理は `spawn_blocking` に載せ、async ランタイムと WebView を
//! 止めない。ストレージ入出力は `AppState` に注入した [`ImageRepository`] 経由で行い、
//! FS への直接依存を持たない（将来 Google Drive 等へ差し替え可能）。
//! 失敗は raw なエラー文字列を surface する（隠蔽 fallback しない）。

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::{ImageFormat, RgbaImage};
use tauri::ipc::Channel;
use tauri::State;

use crate::domain::save::{self, SaveMode};
use crate::imaging::{blur, codec};
use crate::repository::local_fs::LocalFileSystemRepository;
use crate::repository::{ImageRepository, ResourceLocation};
use crate::types::{
    ExportFailure, ExportOutcome, ExportProgress, FilterSettings, ImageMeta, LoadResult,
    PreviewResult,
};

/// プレビューの縮小ベースをキャッシュ（LRU(1)）。スライダー連打で再デコード/再縮小を避ける。
struct PreviewBase {
    /// キャッシュキー: ロケータ（生文字列, 正規化しない）・max_dim・fingerprint の三つ組。
    location: ResourceLocation,
    max_dim: u32,
    /// 読み込み時点の内容鮮度トークン。上書き保存等で内容が変わった後の誤ヒットを防ぐ。
    fingerprint: String,
    base: RgbaImage,
    scale: f32,
    /// 原本の ICC プロファイル。プレビュー PNG にも埋めて書き出しと色を揃える。
    icc: Option<Vec<u8>>,
}

/// アプリ状態。キャンセルフラグ・プレビューキャッシュ・ストレージ実装を共有する。
pub struct AppState {
    cancel: Arc<AtomicBool>,
    preview_cache: Arc<Mutex<Option<PreviewBase>>>,
    /// ストレージポート。既定はローカルFS実装（将来クラウド実装へ差し替え可能）。
    repository: Arc<dyn ImageRepository>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            preview_cache: Arc::new(Mutex::new(None)),
            repository: Arc::new(LocalFileSystemRepository::new()),
        }
    }
}

/// 複数画像のメタ情報を読み込む（ヘッダのみ、フル デコードしない）。
///
/// ファイル単位の失敗は `LoadResult::Error` 行として返す（部分成功を許す）。
/// 一方 `spawn_blocking` の join 失敗（パニック等のインフラ障害）は空配列へ
/// 潰さず raw に surface する（隠蔽 fallback しない）。
#[tauri::command]
pub async fn load_images(
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<LoadResult>, String> {
    let repo = state.repository.clone();
    tauri::async_runtime::spawn_blocking(move || {
        paths
            .into_iter()
            .map(|p| match load_meta(repo.as_ref(), &p) {
                Ok(meta) => LoadResult::Ok { meta },
                Err(e) => LoadResult::Error {
                    path: p,
                    error: format!("{e:#}"),
                },
            })
            .collect()
    })
    .await
    .map_err(|e| format!("failed to load image metadata: {e}"))
}

fn load_meta(repo: &dyn ImageRepository, path: &str) -> anyhow::Result<ImageMeta> {
    let loc = ResourceLocation::try_from(path.to_string())?;
    let meta = repo.metadata(&loc)?;
    Ok(ImageMeta {
        path: loc.as_str().to_string(),
        file_name: meta.name,
        width: meta.width,
        height: meta.height,
        format: format!("{:?}", meta.format),
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
    let repo = state.repository.clone();
    let location = ResourceLocation::try_from(path).map_err(|e| format!("{e:#}"))?;

    tauri::async_runtime::spawn_blocking(move || -> Result<PreviewResult, String> {
        // 内容の鮮度トークン。上書き保存等で内容が変わったキャッシュを誤って使わない。
        let fingerprint = repo.fingerprint(&location).map_err(|e| format!("{e:#}"))?;

        // 縮小ベースを取得（キャッシュヒットしなければ read＋デコード＋縮小して保存）。
        let (base, scale, icc) = {
            let mut guard = cache
                .lock()
                .map_err(|e| format!("failed to lock cache: {e}"))?;
            let hit = guard.as_ref().is_some_and(|b| {
                b.location == location && b.max_dim == max_dim && b.fingerprint == fingerprint
            });
            if !hit {
                let bytes = repo.read(&location).map_err(|e| format!("{e:#}"))?;
                // デコード失敗にロケータ文脈を付す（どのファイルで失敗したか追跡できるように）。
                let loaded = codec::decode_rgba(&bytes)
                    .map_err(|e| format!("{}: {e:#}", location.as_str()))?;
                let (base, scale) = codec::downscale_for_preview(&loaded.image, max_dim);
                *guard = Some(PreviewBase {
                    location: location.clone(),
                    max_dim,
                    fingerprint,
                    base,
                    scale,
                    icc: loaded.icc,
                });
            }
            let b = guard.as_ref().expect("set just above");
            (b.base.clone(), b.scale, b.icc.clone())
        };

        // 縮小ベースへ半径を scale 倍して適用（フルサイズと見た目を一致させる）。
        let blurred = blur::apply_stack_scaled(&base, &stack, scale);
        let png = codec::encode_to_bytes(&blurred, ImageFormat::Png, 90, icc.as_deref())
            .map_err(|e| format!("{e:#}"))?;
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
///
/// 部分失敗・キャンセルは `Err` ではなく [`ExportOutcome`] で返す。
/// `Err` は前提検証（出力衝突・別名保存先の既存）とインフラ障害のみ。
#[tauri::command]
pub async fn export_batch(
    paths: Vec<String>,
    settings: FilterSettings,
    save: SaveMode,
    jpeg_quality: u8,
    on_progress: Channel<ExportProgress>,
    state: State<'_, AppState>,
) -> Result<ExportOutcome, String> {
    let stack = settings.to_stack()?;
    state.cancel.store(false, Ordering::SeqCst);
    let cancel = state.cancel.clone();
    let repo = state.repository.clone();

    tauri::async_runtime::spawn_blocking(move || -> Result<ExportOutcome, String> {
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
                let out_loc =
                    ResourceLocation::try_from(out.as_path()).map_err(|e| format!("{e:#}"))?;
                if repo.exists(&out_loc).map_err(|e| format!("{e:#}"))? {
                    return Err(format!("output already exists: {}", out.display()));
                }
            }
        }

        let total = sources.len() as u32;
        let mut completed = 0u32;
        let mut canceled = false;
        let mut failures: Vec<ExportFailure> = Vec::new();

        for (i, (src, out)) in sources.iter().zip(outputs.iter()).enumerate() {
            // キャンセルは失敗ではなく正常な中断（ここまでの結果を outcome で返す）。
            if cancel.load(Ordering::SeqCst) {
                canceled = true;
                break;
            }

            let result = (|| -> anyhow::Result<()> {
                let src_loc = ResourceLocation::try_from(src.as_path())?;
                let out_loc = ResourceLocation::try_from(out.as_path())?;
                let bytes = repo.read(&src_loc)?;
                let loaded = codec::decode_rgba(&bytes)?;
                let blurred = blur::apply_stack(&loaded.image, &stack);
                let out_format = codec::format_from_name(out)?;
                let out_bytes = codec::encode_to_bytes(
                    &blurred,
                    out_format,
                    jpeg_quality,
                    loaded.icc.as_deref(),
                )?;
                repo.write(&out_loc, &out_bytes)?;
                Ok(())
            })();

            let err = result.err().map(|e| format!("{e:#}"));
            match &err {
                Some(e) => failures.push(ExportFailure {
                    path: src.to_string_lossy().to_string(),
                    error: e.clone(),
                }),
                None => completed += 1,
            }
            let _ = on_progress.send(ExportProgress {
                done: (i as u32) + 1,
                total,
                current_path: src.to_string_lossy().to_string(),
                error: err,
            });
        }

        Ok(ExportOutcome {
            completed,
            canceled,
            failures,
        })
    })
    .await
    .map_err(|e| format!("export processing failed: {e}"))?
}

/// 実行中の一括書き出しをキャンセルする。
#[tauri::command]
pub fn cancel_export(state: State<'_, AppState>) {
    state.cancel.store(true, Ordering::SeqCst);
}
