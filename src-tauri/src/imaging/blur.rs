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

/// UI の強度(半径) → モザイクのブロック辺長への固定変換（UI とパイプラインの契約）。
///
/// 事前条件: `radius <= MAX_RADIUS`（上流の `AxisStrength::uniform` が保証）。
/// 事後条件: 戻り値 >= 1。radius==0 → 1（恒等）、radius>=1 → 2 以上（可視のピクセル化）。
/// `scaled_radius` の事後条件（radius>=1 なら >=1）との合成により、書き出しで
/// ピクセル化されるならプレビューでも必ずピクセル化される（WYSIWYG の維持）。
pub fn radius_to_block(radius: u32) -> u32 {
    radius + 1
}

/// 単一種別・単一半径のブラーを RGBA 画像へ適用する。
///
/// 事前条件: 画像は 1x1 以上。事後条件: 出力寸法 == 入力寸法。radius==0 は恒等。
/// 半透明を含む画像は乗算済みアルファ空間で平均するため、透明画素の色は
/// 可視領域へにじまない（a=0 の出力画素は (0,0,0,0)）。
pub fn blur_rgba(img: &RgbaImage, kind: FilterKind, radius: u32) -> RgbaImage {
    if radius == 0 {
        return img.clone();
    }
    // 全不透明（スクショ等の大多数）はアルファ変換を省略する。結果は変換ありと同一。
    if img.pixels().all(|p| p.0[3] == 255) {
        return blur_channels(img, kind, radius);
    }
    unpremultiply(&blur_channels(&premultiply(img), kind, radius))
}

/// 各チャネル独立のブラー本体（アルファの解釈は呼び出し側 `blur_rgba` に集約）。
fn blur_channels(img: &RgbaImage, kind: FilterKind, radius: u32) -> RgbaImage {
    match kind {
        FilterKind::Gaussian => imageproc::filter::gaussian_blur_f32(img, radius_to_sigma(radius)),
        FilterKind::Block => box_blur_rgba(img, radius),
        FilterKind::Mosaic => mosaic_rgba(img, radius_to_block(radius)),
    }
}

/// 直線(straight)アルファ → 乗算済み(premultiplied)アルファ。整数四捨五入。
fn premultiply(img: &RgbaImage) -> RgbaImage {
    let mut out = img.clone();
    for p in out.pixels_mut() {
        let a = p.0[3] as u32;
        for c in 0..3 {
            p.0[c] = ((p.0[c] as u32 * a + 127) / 255) as u8;
        }
    }
    out
}

/// 乗算済みアルファ → 直線アルファ。a=0 は色情報を持たないため (0,0,0,0) に定める。
fn unpremultiply(img: &RgbaImage) -> RgbaImage {
    let mut out = img.clone();
    for p in out.pixels_mut() {
        let a = p.0[3] as u32;
        if a == 0 {
            p.0 = [0, 0, 0, 0];
            continue;
        }
        for c in 0..3 {
            p.0[c] = ((p.0[c] as u32 * 255 + a / 2) / a).min(255) as u8;
        }
    }
    out
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
        blur_rgba(&acc, spec.kind, scaled_radius(spec.strength.x(), scale))
    })
}

/// 半径のスケール補正。
///
/// 事後条件: radius==0 なら 0（恒等の維持）、radius>=1 なら 1 以上。
/// 強い縮小率で round が 0 に落ちると「プレビューは素通し・書き出しはぼける」という
/// WYSIWYG 破れになるため、正の半径は下限 1 でぼけの存在自体を保つ。
fn scaled_radius(radius: u32, scale: f32) -> u32 {
    if radius == 0 {
        return 0;
    }
    (((radius as f32) * scale).round() as u32).max(1)
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

/// モザイク（ピクセレート）: Triangle 縮小（近似ブロック平均）→ Nearest 拡大。
///
/// 事前条件: `block >= 1`。事後条件: 出力寸法 == 入力寸法。block == 1 は恒等。
/// block >= 2 の出力は ceil(w/b) x ceil(h/b) 個のセルで区分一定
/// （セル境界は Nearest により均等配分され、寸法が block の整数倍なら block 格子と一致）。
/// 不変条件: 一様画像は不変（resize の重みは正規化されている）。
fn mosaic_rgba(img: &RgbaImage, block: u32) -> RgbaImage {
    if block <= 1 {
        return img.clone();
    }
    let (w, h) = img.dimensions();
    let small = image::imageops::resize(
        img,
        w.div_ceil(block),
        h.div_ceil(block),
        image::imageops::FilterType::Triangle,
    );
    image::imageops::resize(&small, w, h, image::imageops::FilterType::Nearest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::filter::{AxisStrength, FilterSpec, MAX_RADIUS};
    use image::Rgba;

    const ALL_KINDS: [FilterKind; 3] =
        [FilterKind::Gaussian, FilterKind::Block, FilterKind::Mosaic];

    fn solid(w: u32, h: u32, px: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba(px))
    }

    /// 全画素の値が異なる勾配画像（ピクセル化の検出用）。
    fn gradient(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            Rgba([(x * 30) as u8, (y * 40) as u8, (x + y) as u8, 255])
        })
    }

    #[test]
    fn radius_zero_is_identity() {
        let img = solid(4, 4, [10, 20, 30, 255]);
        for kind in ALL_KINDS {
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

    #[test]
    fn scaled_radius_keeps_positive_radius_above_zero() {
        // 丸めで 0（恒等）へ落ちると WYSIWYG が破れる（プレビュー素通し・出力ぼけ）。
        assert_eq!(scaled_radius(1, 0.4), 1);
        assert_eq!(scaled_radius(0, 0.4), 0, "radius 0 は恒等のまま");
        assert_eq!(scaled_radius(20, 0.4), 8);
        assert_eq!(scaled_radius(3, 1.0), 3, "scale 1 は無補正");
    }

    #[test]
    fn transparent_pixels_do_not_bleed_color() {
        // 不透明赤 1 画素 + 完全透明緑。透明画素の色（緑）は可視領域へにじんではならない。
        let mut img = solid(3, 1, [0, 255, 0, 0]);
        img.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        for kind in ALL_KINDS {
            let out = blur_rgba(&img, kind, 1);
            for (x, _, p) in out.enumerate_pixels() {
                assert_eq!(p.0[1], 0, "kind={kind:?} x={x}: 透明画素の緑がにじんだ");
            }
            // 乗算済み空間では透明画素の寄与が 0 のため、端の不透明画素は純赤のまま。
            assert_eq!(out.get_pixel(0, 0).0[0], 255, "kind={kind:?}");
        }
    }

    #[test]
    fn semi_transparent_uniform_image_is_unchanged() {
        // premultiply→blur→unpremultiply の往復が一様画像で値を変えないこと。
        let img = solid(9, 9, [40, 80, 120, 200]);
        for kind in ALL_KINDS {
            assert_eq!(blur_rgba(&img, kind, 3), img, "kind={kind:?}");
        }
    }

    #[test]
    fn fully_transparent_pixels_come_out_zeroed() {
        // a=0 の画素は色情報を持たない ＝ 出力は (0,0,0,0)。
        let img = solid(4, 4, [200, 100, 50, 0]);
        for kind in ALL_KINDS {
            let out = blur_rgba(&img, kind, 2);
            assert!(out.pixels().all(|p| p.0 == [0, 0, 0, 0]), "kind={kind:?}");
        }
    }

    #[test]
    fn radius_to_block_contract() {
        assert_eq!(radius_to_block(0), 1, "radius 0 は恒等（block 1）");
        assert_eq!(radius_to_block(1), 2, "radius 1 から可視のピクセル化");
        assert_eq!(radius_to_block(MAX_RADIUS), MAX_RADIUS + 1);
    }

    #[test]
    fn mosaic_preserves_dimensions_on_non_multiples() {
        // block 3 で 7x5（整数倍でない寸法）→ ceil 除算でセル数 3x2、寸法は不変。
        let img = gradient(7, 5);
        let out = blur_rgba(&img, FilterKind::Mosaic, 2);
        assert_eq!(out.dimensions(), (7, 5));
    }

    #[test]
    fn mosaic_bounds_distinct_colors_on_non_multiples() {
        // 非整数倍寸法でも区分一定: Nearest 拡大は縮小画像の画素を複製するだけなので、
        // 色数はセル数 ceil(7/3) x ceil(5/3) = 6 を超えない（境界の丸め実装に依存しない検証）。
        let img = gradient(7, 5);
        let out = blur_rgba(&img, FilterKind::Mosaic, 2); // block 3
        assert_ne!(out, img, "非恒等（ピクセル化が起きるべき）");
        let mut colors: Vec<[u8; 4]> = out.pixels().map(|p| p.0).collect();
        colors.sort_unstable();
        colors.dedup();
        assert!(
            colors.len() <= 6,
            "色数 {} がセル数 6 を超えた",
            colors.len()
        );
    }

    #[test]
    fn mosaic_is_piecewise_constant_on_exact_grid() {
        // 寸法が block の整数倍なら Nearest 拡大の格子が block 格子と一致し、
        // 各 2x2 セル内は一定になる。
        let img = gradient(8, 6);
        let out = blur_rgba(&img, FilterKind::Mosaic, 1); // block 2
        assert_ne!(out, img, "非恒等（ピクセル化が起きるべき）");
        for (x, y, p) in out.enumerate_pixels() {
            let anchor = out.get_pixel(x - x % 2, y - y % 2);
            assert_eq!(p, anchor, "セル({},{}) 内が一定でない", x / 2, y / 2);
        }
    }

    #[test]
    fn mosaic_uniform_image_is_unchanged() {
        // resize の重みは正規化されているため、一様画像はセル分割でも値が変わらない。
        let img = solid(9, 9, [40, 80, 120, 255]);
        let out = blur_rgba(&img, FilterKind::Mosaic, 3);
        assert_eq!(out, img);
    }

    #[test]
    fn mosaic_preview_never_collapses_to_identity() {
        // 強い縮小率でも効果が消えない（scaled_radius >= 1 と block = radius + 1 の合成）。
        // 消えると「プレビュー素通し・書き出しモザイク」の WYSIWYG 破れになる。
        let img = gradient(8, 8);
        let spec = FilterSpec::whole(FilterKind::Mosaic, AxisStrength::uniform(1).unwrap());
        let out = apply_stack_scaled(&img, &FilterStack::single(spec), 0.1);
        assert_ne!(out, img, "プレビューが素通しになっている");
    }
}
