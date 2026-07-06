//! 画像の入出力（imaging アダプタ）。
//!
//! デコード（EXIF 向き正規化つき）、フォーマット別エンコード、原子的書き込み、
//! プレビュー用ダウンスケールを提供する。失敗は raw なエラーを surface する。

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use image::{DynamicImage, ImageDecoder, ImageEncoder, ImageFormat, ImageReader, RgbaImage};

/// デコード済み画像とその素性。
pub struct LoadedImage {
    pub image: RgbaImage,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
}

/// パスから画像を RGBA8 でデコードし、EXIF Orientation を正規化する。
///
/// 事後条件: 返る画像は EXIF に従い正しい向き。フォーマットはパス/内容から判定。
pub fn load_rgba(path: &Path) -> Result<LoadedImage> {
    let reader = ImageReader::open(path)
        .with_context(|| format!("cannot open image: {}", path.display()))?
        .with_guessed_format()
        .with_context(|| format!("cannot determine format: {}", path.display()))?;

    let format = reader
        .format()
        .ok_or_else(|| anyhow!("unsupported or unknown format: {}", path.display()))?;

    let mut decoder = reader
        .into_decoder()
        .with_context(|| format!("cannot build decoder: {}", path.display()))?;
    // EXIF 向き。取得できない場合は無変換。
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);

    let mut dynimg = DynamicImage::from_decoder(decoder)
        .with_context(|| format!("cannot decode image: {}", path.display()))?;
    dynimg.apply_orientation(orientation);

    let image = dynimg.to_rgba8();
    let (width, height) = image.dimensions();
    Ok(LoadedImage { image, format, width, height })
}

/// パスの拡張子からエンコード先フォーマットを決める。
pub fn format_from_path(path: &Path) -> Result<ImageFormat> {
    ImageFormat::from_path(path)
        .with_context(|| format!("cannot determine format from extension: {}", path.display()))
}

/// RGBA 画像を指定フォーマットのバイト列にエンコードする。
///
/// - JPEG: アルファを落とし RGB で `jpeg_quality` エンコード。
/// - WebP: `image` の制約により **ロスレス** のみ（明示・隠蔽 fallback しない）。
/// - PNG/BMP: そのまま RGBA。
pub fn encode_to_bytes(img: &RgbaImage, format: ImageFormat, jpeg_quality: u8) -> Result<Vec<u8>> {
    let (w, h) = img.dimensions();
    let mut buf: Vec<u8> = Vec::new();

    // すべて ImageEncoder::write_image で統一（バージョン差に強い）。
    match format {
        ImageFormat::Jpeg => {
            // JPEG はアルファを持てないので RGB に変換する。
            let rgb = DynamicImage::ImageRgba8(img.clone()).to_rgb8();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, jpeg_quality)
                .write_image(rgb.as_raw(), w, h, image::ExtendedColorType::Rgb8)
                .context("JPEG encoding failed")?;
        }
        ImageFormat::Png => {
            image::codecs::png::PngEncoder::new(&mut buf)
                .write_image(img.as_raw(), w, h, image::ExtendedColorType::Rgba8)
                .context("PNG encoding failed")?;
        }
        ImageFormat::WebP => {
            // image クレートの WebP エンコードはロスレスのみ（隠蔽 fallback しない）。
            image::codecs::webp::WebPEncoder::new_lossless(&mut buf)
                .write_image(img.as_raw(), w, h, image::ExtendedColorType::Rgba8)
                .context("WebP (lossless) encoding failed")?;
        }
        ImageFormat::Bmp => {
            image::codecs::bmp::BmpEncoder::new(&mut buf)
                .write_image(img.as_raw(), w, h, image::ExtendedColorType::Rgba8)
                .context("BMP encoding failed")?;
        }
        other => {
            return Err(anyhow!("unsupported output format: {other:?}"));
        }
    }
    Ok(buf)
}

/// 同一ディレクトリの一時ファイルへ書いてから rename する原子的書き込み。
///
/// 途中クラッシュで元ファイルを壊さない。temp は同ボリュームなので rename は原子的。
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
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

/// 長辺が `max_dim` 以下になる縮小率を返す（既に十分小さければ 1.0）。
pub fn preview_scale(width: u32, height: u32, max_dim: u32) -> f32 {
    let longest = width.max(height);
    if longest == 0 || longest <= max_dim {
        1.0
    } else {
        max_dim as f32 / longest as f32
    }
}

/// プレビュー用に長辺 `max_dim` へダウンスケールした画像と縮小率を返す。
pub fn downscale_for_preview(img: &RgbaImage, max_dim: u32) -> (RgbaImage, f32) {
    let (w, h) = img.dimensions();
    let scale = preview_scale(w, h, max_dim);
    if scale >= 1.0 {
        return (img.clone(), 1.0);
    }
    let nw = ((w as f32) * scale).round().max(1.0) as u32;
    let nh = ((h as f32) * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(img, nw, nh, image::imageops::FilterType::Triangle);
    (resized, scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn preview_scale_no_upscale() {
        assert_eq!(preview_scale(100, 50, 200), 1.0);
    }

    #[test]
    fn preview_scale_downscales_by_longest_side() {
        // 長辺 1000 を 500 に → 0.5
        assert!((preview_scale(1000, 250, 500) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn downscale_reduces_longest_side() {
        let img = RgbaImage::from_pixel(1000, 400, Rgba([1, 2, 3, 255]));
        let (small, scale) = downscale_for_preview(&img, 500);
        assert_eq!(small.width().max(small.height()), 500);
        assert!((scale - 0.5).abs() < 1e-6);
    }

    #[test]
    fn png_encode_decodes_back_to_same_pixels() {
        let img = RgbaImage::from_pixel(8, 8, Rgba([10, 20, 30, 255]));
        let bytes = encode_to_bytes(&img, ImageFormat::Png, 90).unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert_eq!(decoded, img);
    }
}
