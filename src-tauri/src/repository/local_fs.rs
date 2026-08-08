//! ローカルファイルシステムのストレージアダプタ。
//!
//! [`ImageRepository`] を FS 上で実装する。`write` は同ボリューム内の temp へ書いてから
//! rename する（途中クラッシュで元ファイルを壊さない）。`metadata` はヘッダのみ読む
//! `image_dimensions` を用いて全読み込みを避ける。

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use super::{ImageRepository, ResourceLocation, ResourceMeta};
use crate::imaging::codec;

/// ローカルFSアダプタ（状態を持たない）。
#[derive(Debug, Default, Clone)]
pub struct LocalFileSystemRepository;

impl LocalFileSystemRepository {
    pub fn new() -> Self {
        Self
    }
}

/// ロケータを FS パスとして解釈する。
fn to_path(loc: &ResourceLocation) -> &Path {
    Path::new(loc.as_str())
}

impl ImageRepository for LocalFileSystemRepository {
    fn read(&self, loc: &ResourceLocation) -> Result<Vec<u8>> {
        let path = to_path(loc);
        std::fs::read(path).with_context(|| format!("cannot read image: {}", path.display()))
    }

    fn write(&self, loc: &ResourceLocation, bytes: &[u8]) -> Result<()> {
        let path = to_path(loc);
        let parent = path
            .parent()
            .ok_or_else(|| anyhow!("no parent directory: {}", path.display()))?;
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create output directory: {}", parent.display()))?;

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("invalid output file name: {}", path.display()))?;
        let tmp: PathBuf = parent.join(format!(".{file_name}.tmp"));

        std::fs::write(&tmp, bytes)
            .with_context(|| format!("cannot write temporary file: {}", tmp.display()))?;
        std::fs::rename(&tmp, path)
            .with_context(|| format!("cannot rename to output: {}", path.display()))?;
        Ok(())
    }

    fn exists(&self, loc: &ResourceLocation) -> Result<bool> {
        // `try_exists` は権限エラー等を Err として surface する（`exists()` の false 潰しを避ける）。
        let path = to_path(loc);
        path.try_exists()
            .with_context(|| format!("cannot check existence: {}", path.display()))
    }

    fn metadata(&self, loc: &ResourceLocation) -> Result<ResourceMeta> {
        let path = to_path(loc);
        let (width, height) = image::image_dimensions(path)
            .with_context(|| format!("cannot get image dimensions: {}", path.display()))?;
        let format = codec::format_from_name(path)?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        Ok(ResourceMeta {
            name,
            width,
            height,
            format,
        })
    }

    fn fingerprint(&self, loc: &ResourceLocation) -> Result<String> {
        let path = to_path(loc);
        let md = std::fs::metadata(path)
            .with_context(|| format!("cannot read file metadata: {}", path.display()))?;
        let modified = md
            .modified()
            .with_context(|| format!("cannot read modified time: {}", path.display()))?;
        // epoch 以前の mtime も一意に表現する（fallback で潰さない）。
        let stamp = match modified.duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => format!("{}", d.as_nanos()),
            Err(e) => format!("-{}", e.duration().as_nanos()),
        };
        Ok(format!("{}:{stamp}", md.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::codec;
    use image::{ImageFormat, Rgba, RgbaImage};

    /// テスト専用の一時ディレクトリ（テストごとに一意名）。既存があれば作り直す。
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("batch_blur_test_{tag}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn png_bytes(w: u32, h: u32, px: [u8; 4]) -> Vec<u8> {
        codec::encode_to_bytes(
            &RgbaImage::from_pixel(w, h, Rgba(px)),
            ImageFormat::Png,
            90,
            None,
        )
        .unwrap()
    }

    #[test]
    fn write_read_exists_metadata_roundtrip() {
        let dir = temp_dir("local_fs_roundtrip");
        let repo = LocalFileSystemRepository::new();
        let bytes = png_bytes(5, 3, [12, 34, 56, 255]);
        let loc = ResourceLocation::try_from(dir.join("out.png").as_path()).unwrap();

        assert!(!repo.exists(&loc).unwrap());
        repo.write(&loc, &bytes).unwrap();
        assert!(repo.exists(&loc).unwrap());
        assert_eq!(repo.read(&loc).unwrap(), bytes);

        let meta = repo.metadata(&loc).unwrap();
        assert_eq!((meta.width, meta.height), (5, 3));
        assert_eq!(meta.name, "out.png");
        assert_eq!(meta.format, ImageFormat::Png);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_replaces_existing_file() {
        let dir = temp_dir("local_fs_replace");
        let repo = LocalFileSystemRepository::new();
        let loc = ResourceLocation::try_from(dir.join("f.png").as_path()).unwrap();

        let first = png_bytes(2, 2, [1, 1, 1, 255]);
        let second = png_bytes(4, 4, [9, 9, 9, 255]);
        repo.write(&loc, &first).unwrap();
        repo.write(&loc, &second).unwrap();
        assert_eq!(repo.read(&loc).unwrap(), second);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_creates_missing_parent_dirs() {
        let dir = temp_dir("local_fs_mkdir");
        let repo = LocalFileSystemRepository::new();
        let loc = ResourceLocation::try_from(dir.join("nested/deep/o.png").as_path()).unwrap();
        repo.write(&loc, &png_bytes(2, 2, [7, 7, 7, 255])).unwrap();
        assert!(repo.exists(&loc).unwrap());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fingerprint_reflects_content_replacement() {
        let dir = temp_dir("local_fs_fingerprint");
        let repo = LocalFileSystemRepository::new();
        let loc = ResourceLocation::try_from(dir.join("f.png").as_path()).unwrap();

        repo.write(&loc, &png_bytes(2, 2, [1, 1, 1, 255])).unwrap();
        let fp1 = repo.fingerprint(&loc).unwrap();
        assert_eq!(repo.fingerprint(&loc).unwrap(), fp1, "同一内容なら安定");

        // サイズの異なる内容へ置換（mtime 粒度に依存せず必ず差が出る）。
        repo.write(&loc, &png_bytes(8, 8, [9, 9, 9, 255])).unwrap();
        assert_ne!(repo.fingerprint(&loc).unwrap(), fp1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_missing_file_surfaces_error() {
        let dir = temp_dir("local_fs_missing");
        let repo = LocalFileSystemRepository::new();
        let loc = ResourceLocation::try_from(dir.join("nope.png").as_path()).unwrap();
        assert!(repo.read(&loc).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
