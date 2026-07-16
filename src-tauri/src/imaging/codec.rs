//! 画像コーデック（imaging アダプタ・純粋 bytes<->pixels）。
//!
//! バイト列からのデコード（EXIF 向き正規化つき）、フォーマット別エンコード、
//! プレビュー用ダウンスケールを提供する。ファイルシステムアクセスは持たない
//! （読み書きは repository モジュールが担う）。失敗は raw なエラーを surface する。

use std::io::Cursor;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use image::{DynamicImage, ImageDecoder, ImageEncoder, ImageFormat, ImageReader, RgbaImage};

/// デコード済み画像とその素性。
pub struct LoadedImage {
    pub image: RgbaImage,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
}

/// バイト列を画像として RGBA8 でデコードし、EXIF Orientation を正規化する。
///
/// 事前条件: `bytes` は 1 枚の画像として解釈可能。
/// 事後条件: 返る画像は EXIF に従い正しい向き。フォーマットは内容から判定。
pub fn decode_rgba(bytes: &[u8]) -> Result<LoadedImage> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .context("cannot determine image format")?;

    let format = reader
        .format()
        .ok_or_else(|| anyhow!("unsupported or unknown image format"))?;

    let mut decoder = reader.into_decoder().context("cannot build decoder")?;
    // EXIF 向き。取得できない場合は無変換。
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);

    let mut dynimg = DynamicImage::from_decoder(decoder).context("cannot decode image")?;
    dynimg.apply_orientation(orientation);

    let image = dynimg.to_rgba8();
    let (width, height) = image.dimensions();
    Ok(LoadedImage {
        image,
        format,
        width,
        height,
    })
}

/// パス（名前）の拡張子からエンコード先フォーマットを決める。
///
/// 拡張子不明時は loud にエラー（内容推定 fallback はしない ＝ Raw Data Now）。
pub fn format_from_name(path: &Path) -> Result<ImageFormat> {
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
        let decoded = decode_rgba(&bytes).unwrap();
        assert_eq!(decoded.image, img);
        assert_eq!(decoded.format, ImageFormat::Png);
    }

    /// EXIF Orientation タグ付き JPEG を合成する（TIFF は little-endian, IFD0 に Orientation 1 件）。
    ///
    /// SOI 直後に APP1(Exif) セグメントを差し込み、後段の実画像データはエンコーダ出力を流用する。
    fn jpeg_with_orientation(img: &RgbaImage, orientation: u8) -> Vec<u8> {
        let base = encode_to_bytes(img, ImageFormat::Jpeg, 90).unwrap();

        let mut payload: Vec<u8> = Vec::new();
        payload.extend_from_slice(b"Exif\0\0");
        // TIFF ヘッダ (LE): "II", 42, IFD0 offset = 8
        payload.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00]);
        payload.extend_from_slice(&[0x01, 0x00]); // IFD0 エントリ数 = 1
        payload.extend_from_slice(&[0x12, 0x01]); // tag 0x0112 (Orientation)
        payload.extend_from_slice(&[0x03, 0x00]); // type = SHORT
        payload.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // count = 1
        payload.extend_from_slice(&[orientation, 0x00, 0x00, 0x00]); // value (左詰め)
        payload.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // 次 IFD offset = なし

        let seg_len = (payload.len() + 2) as u16; // 長さフィールド自身を含む

        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(&base[0..2]); // SOI (FF D8)
        out.extend_from_slice(&[0xFF, 0xE1]); // APP1 marker
        out.extend_from_slice(&seg_len.to_be_bytes());
        out.extend_from_slice(&payload);
        out.extend_from_slice(&base[2..]); // 残り（元の APP0 以降）
        out
    }

    #[test]
    fn decode_honors_exif_orientation() {
        let img = RgbaImage::from_pixel(4, 2, Rgba([200, 50, 50, 255]));

        // Orientation 6 = 90°回転 → 寸法が入れ替わる (4x2 -> 2x4)。
        let rotated = jpeg_with_orientation(&img, 6);
        let loaded = decode_rgba(&rotated).unwrap();
        assert_eq!(
            (loaded.width, loaded.height),
            (2, 4),
            "EXIF orientation 6 は寸法を入れ替えるべき"
        );

        // 制御: Orientation 1 (NoTransforms) は寸法維持。
        let upright = jpeg_with_orientation(&img, 1);
        let loaded2 = decode_rgba(&upright).unwrap();
        assert_eq!((loaded2.width, loaded2.height), (4, 2));
    }
}
