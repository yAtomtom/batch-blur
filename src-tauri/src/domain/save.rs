//! 保存先パスの解決（純粋・ファイルシステム非依存）。
//!
//! 「上書き / 別名保存（prefix・suffix 付与）」の出力パス導出と、バッチ内の
//! 出力衝突検出のみを担う。実際の書き込み（存在確認・rename）は imaging/コマンド層。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// ファイル名に使えない文字（Windows 準拠＋パス区切り）。affix 検証に用いる。
const ILLEGAL_AFFIX_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// 保存モード。上書きか、prefix/suffix を付与した別名保存か。
///
/// prefix・suffix は独立（空文字は「付与しない」）。「両方選べる」要件に対応する最小モデル。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum SaveMode {
    /// 元ファイルをそのまま上書き。
    Overwrite,
    /// 別名保存。出力先ディレクトリ未指定なら元ファイルと同じ場所。
    #[serde(rename_all = "camelCase")]
    SaveAs {
        prefix: String,
        suffix: String,
        out_dir: Option<PathBuf>,
    },
}

/// affix（prefix/suffix）が不正文字・パス区切りを含まないか検証する。
fn validate_affix(label: &str, affix: &str) -> Result<(), String> {
    if affix.chars().any(|c| ILLEGAL_AFFIX_CHARS.contains(&c) || c.is_control()) {
        return Err(format!(
            "{label} に使用できない文字が含まれています: {affix:?}"
        ));
    }
    Ok(())
}

/// 単一ファイルの出力パスを導出する。
///
/// 事前条件:
/// - `source` は絶対パスかつファイル名を持つ。
/// - SaveAs の場合、affix にパス区切り/OS 禁止文字を含まない。
///
/// 事後条件:
/// - Overwrite ⇒ 出力 == `source`。
/// - SaveAs ⇒ `dir / (prefix + stem + suffix [+ "." + ext])`。**拡張子は変えない**。
pub fn resolve_output_path(source: &Path, mode: &SaveMode) -> Result<PathBuf, String> {
    if !source.is_absolute() {
        return Err(format!("入力パスが絶対パスではありません: {}", source.display()));
    }
    if source.file_name().is_none() {
        return Err(format!("入力パスにファイル名がありません: {}", source.display()));
    }

    match mode {
        SaveMode::Overwrite => Ok(source.to_path_buf()),
        SaveMode::SaveAs { prefix, suffix, out_dir } => {
            validate_affix("prefix", prefix)?;
            validate_affix("suffix", suffix)?;

            let stem = source
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| format!("ファイル名(stem)を取得できません: {}", source.display()))?;

            let dir: PathBuf = match out_dir {
                Some(d) => d.clone(),
                None => source
                    .parent()
                    .map(|p| p.to_path_buf())
                    .ok_or_else(|| format!("親ディレクトリを取得できません: {}", source.display()))?,
            };

            let filename = match source.extension().and_then(|e| e.to_str()) {
                Some(ext) => format!("{prefix}{stem}{suffix}.{ext}"),
                None => format!("{prefix}{stem}{suffix}"),
            };

            Ok(dir.join(filename))
        }
    }
}

/// バッチ内で出力パスが衝突していないか検証する（大文字小文字は Win/mac を考慮し無視）。
///
/// 事後条件: 重複があれば最初の衝突パスを含むエラー。自動リネームによる隠蔽はしない。
pub fn check_no_collisions(outputs: &[PathBuf]) -> Result<(), String> {
    use std::collections::HashSet;
    let mut seen: HashSet<String> = HashSet::with_capacity(outputs.len());
    for out in outputs {
        let key = out.to_string_lossy().to_lowercase();
        if !seen.insert(key) {
            return Err(format!("出力パスが衝突しています: {}", out.display()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn abs(p: &str) -> PathBuf {
        // テストは絶対パス前提。Windows/Unix 双方で絶対になるよう調整。
        if cfg!(windows) {
            PathBuf::from(format!("C:\\{}", p.trim_start_matches('/')))
        } else {
            PathBuf::from(format!("/{}", p.trim_start_matches('/')))
        }
    }

    #[test]
    fn overwrite_returns_source() {
        let src = abs("dir/photo.png");
        let out = resolve_output_path(&src, &SaveMode::Overwrite).unwrap();
        assert_eq!(out, src);
    }

    #[test]
    fn saveas_suffix_keeps_extension() {
        let src = abs("dir/photo.png");
        let mode = SaveMode::SaveAs {
            prefix: String::new(),
            suffix: "_blur".into(),
            out_dir: None,
        };
        let out = resolve_output_path(&src, &mode).unwrap();
        assert_eq!(out.file_name().unwrap().to_str().unwrap(), "photo_blur.png");
        assert_eq!(out.parent().unwrap(), src.parent().unwrap());
    }

    #[test]
    fn saveas_prefix_and_suffix_both_applied() {
        let src = abs("dir/photo.jpg");
        let mode = SaveMode::SaveAs {
            prefix: "b_".into(),
            suffix: "_x".into(),
            out_dir: None,
        };
        let out = resolve_output_path(&src, &mode).unwrap();
        assert_eq!(out.file_name().unwrap().to_str().unwrap(), "b_photo_x.jpg");
    }

    #[test]
    fn saveas_uses_out_dir_when_given() {
        let src = abs("dir/photo.png");
        let dst = abs("other");
        let mode = SaveMode::SaveAs {
            prefix: String::new(),
            suffix: "_blur".into(),
            out_dir: Some(dst.clone()),
        };
        let out = resolve_output_path(&src, &mode).unwrap();
        assert_eq!(out, dst.join("photo_blur.png"));
    }

    #[test]
    fn saveas_rejects_affix_with_separator() {
        let src = abs("dir/photo.png");
        let mode = SaveMode::SaveAs {
            prefix: String::new(),
            suffix: "../evil".into(),
            out_dir: None,
        };
        assert!(resolve_output_path(&src, &mode).is_err());
    }

    #[test]
    fn relative_source_is_rejected() {
        let src = PathBuf::from("relative/photo.png");
        assert!(resolve_output_path(&src, &SaveMode::Overwrite).is_err());
    }

    #[test]
    fn collision_detected_case_insensitively() {
        let a = abs("dir/Photo.png");
        let b = abs("dir/photo.png");
        assert!(check_no_collisions(&[a, b]).is_err());
    }

    #[test]
    fn no_collision_for_distinct_paths() {
        let a = abs("dir/a.png");
        let b = abs("dir/b.png");
        assert!(check_no_collisions(&[a, b]).is_ok());
    }
}
