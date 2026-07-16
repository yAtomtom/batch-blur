//! ブラーカーネル（imaging アダプタ）。
//!
//! ドメインの [`FilterStack`] を実ピクセルへ適用する。プレビュー（縮小）と
//! 書き出し（フル）が **同一のブラー関数** を通るため WYSIWYG が保たれる。

use image::RgbaImage;

use crate::domain::filter::{FilterKind, FilterStack};

/// UI の強度(半径) → ガウス sigma への固定変換（UI とパイプラインの契約）。
pub fn radius_to_sigma(radius: u32) -> f32 {
    radius as f32 / 2.0
}

/// 単一種別・単一半径のブラーを RGBA 画像へ適用する。
///
/// 事前条件: 画像は 1x1 以上。事後条件: 出力寸法 == 入力寸法。radius==0 は恒等。
pub fn blur_rgba(img: &RgbaImage, kind: FilterKind, radius: u32) -> RgbaImage {
    if radius == 0 {
        return img.clone();
    }
    match kind {
        FilterKind::Gaussian => imageproc::filter::gaussian_blur_f32(img, radius_to_sigma(radius)),
        FilterKind::Block => box_blur_rgba(img, radius),
    }
}

/// フルサイズ適用: スタックを順序どおり fold する（ブラーは非可換なので順序保持）。
///
/// 事後条件: 空スタックは恒等（入力の複製）。出力寸法 == 入力寸法。
pub fn apply_stack(img: &RgbaImage, stack: &FilterStack) -> RgbaImage {
    stack.specs().iter().fold(img.clone(), |acc, spec| {
        // MVP: region は Whole のみ。将来は spec.region で部分適用に分岐する。
        blur_rgba(&acc, spec.kind, spec.strength.x())
    })
}

/// プレビュー適用: 縮小画像に対し、半径を `scale` 倍して忠実性を保つ。
///
/// フルサイズと同じ半径を縮小画像へ適用すると過剰にぼけるため、
/// `preview_radius = round(radius * scale)` に補正する（`scale` は縮小率）。
pub fn apply_stack_scaled(img: &RgbaImage, stack: &FilterStack, scale: f32) -> RgbaImage {
    stack.specs().iter().fold(img.clone(), |acc, spec| {
        let scaled = ((spec.strength.x() as f32) * scale).round() as u32;
        blur_rgba(&acc, spec.kind, scaled)
    })
}

/// 端をエッジ複製(clamp)で扱う分離可能ボックスブラー。
///
/// 水平→垂直の 2 パス、各チャネル独立の running-sum で O(pixels)。
/// window = 2*radius+1 を常に一定にし、端でも減算されない（暗くならない）。
fn box_blur_rgba(img: &RgbaImage, radius: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let wi = w as i64;
    let hi = h as i64;
    let r = radius as i64;
    let d = 2 * r + 1; // window 幅（除数, 一定）
    let half = d / 2; // 四捨五入用

    let src = img.as_raw();
    let mut tmp = vec![0u8; src.len()];

    // 水平パス: src -> tmp
    for y in 0..hi {
        let row = (y * wi) as usize; // 行頭のピクセル index
        for c in 0..4usize {
            let sample = |x: i64| -> i64 { src[(row + clamp_idx(x, wi)) * 4 + c] as i64 };
            let mut sum: i64 = (-r..=r).map(sample).sum();
            tmp[row * 4 + c] = ((sum + half) / d) as u8;
            for x in 1..wi {
                sum += sample(x + r) - sample(x - 1 - r);
                tmp[(row + x as usize) * 4 + c] = ((sum + half) / d) as u8;
            }
        }
    }

    // 垂直パス: tmp -> out
    let mut out = vec![0u8; src.len()];
    let wus = w as usize;
    for x in 0..wi {
        let xu = x as usize;
        for c in 0..4usize {
            let sample = |y: i64| -> i64 { tmp[(clamp_idx(y, hi) * wus + xu) * 4 + c] as i64 };
            let mut sum: i64 = (-r..=r).map(sample).sum();
            out[xu * 4 + c] = ((sum + half) / d) as u8;
            for y in 1..hi {
                sum += sample(y + r) - sample(y - 1 - r);
                out[(y as usize * wus + xu) * 4 + c] = ((sum + half) / d) as u8;
            }
        }
    }

    RgbaImage::from_raw(w, h, out).expect("output buffer length matches dimensions")
}

/// 座標を `[0, n-1]` にクランプ（エッジ複製）。
fn clamp_idx(i: i64, n: i64) -> usize {
    i.clamp(0, n - 1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::filter::{AxisStrength, FilterSpec};
    use image::Rgba;

    fn solid(w: u32, h: u32, px: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba(px))
    }

    #[test]
    fn radius_zero_is_identity() {
        let img = solid(4, 4, [10, 20, 30, 255]);
        for kind in [FilterKind::Gaussian, FilterKind::Block] {
            let out = blur_rgba(&img, kind, 0);
            assert_eq!(out, img, "radius 0 は恒等であるべき");
        }
    }

    #[test]
    fn box_blur_preserves_dimensions() {
        let img = solid(7, 3, [100, 100, 100, 255]);
        let out = box_blur_rgba(&img, 2);
        assert_eq!(out.dimensions(), (7, 3));
    }

    #[test]
    fn box_blur_of_uniform_image_is_unchanged() {
        // エッジ clamp が正しければ、一様画像は境界でも値が変わらない。
        let img = solid(9, 9, [40, 80, 120, 200]);
        let out = box_blur_rgba(&img, 3);
        assert_eq!(out, img);
    }

    #[test]
    fn gaussian_preserves_dimensions() {
        let img = solid(16, 8, [200, 10, 10, 255]);
        let out = blur_rgba(&img, FilterKind::Gaussian, 4);
        assert_eq!(out.dimensions(), (16, 8));
    }

    #[test]
    fn empty_stack_is_identity() {
        let img = solid(5, 5, [1, 2, 3, 4]);
        let out = apply_stack(&img, &FilterStack::new(vec![]));
        assert_eq!(out, img);
    }

    #[test]
    fn apply_stack_single_matches_blur_rgba() {
        let img = solid(12, 12, [123, 200, 50, 255]);
        let spec = FilterSpec::whole(FilterKind::Block, AxisStrength::uniform(2).unwrap());
        let via_stack = apply_stack(&img, &FilterStack::single(spec));
        let direct = blur_rgba(&img, FilterKind::Block, 2);
        assert_eq!(via_stack, direct);
    }
}
