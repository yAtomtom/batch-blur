//! 画像ストレージのポート（境界付けられたコンテキスト: Image Storage）。
//!
//! 読み書き・存在確認・メタ取得を trait の背後に隔離し、ローカルFS／将来のクラウド
//! （Google Drive 等）を同一インターフェースで差し替え可能にする。コアドメインには
//! 依存させない（本モジュールは infra ポート）。失敗は raw に surface する。

use std::path::Path;

use anyhow::{anyhow, Result};
use image::ImageFormat;

pub mod local_fs;

/// ストレージ上の画像リソースの所在。
///
/// FS ではパス文字列、将来の Drive では file-id 等を表す opaque な値。
/// 不変条件: 非空（生成は `TryFrom` で強制し、空文字は Err）。
/// ※ scheme 解析は今は持たない（単一FSのため過剰。将来ルーティング導入時に追加）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceLocation(String);

impl ResourceLocation {
    /// ロケータの生文字列。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ResourceLocation {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self> {
        if value.is_empty() {
            return Err(anyhow!("resource location must not be empty"));
        }
        Ok(Self(value))
    }
}

impl TryFrom<&Path> for ResourceLocation {
    type Error = anyhow::Error;

    /// FS パスからロケータへ。非 UTF-8 パスは lossy 変換せず loud に拒否する。
    fn try_from(path: &Path) -> Result<Self> {
        let s = path
            .to_str()
            .ok_or_else(|| anyhow!("resource location is not valid UTF-8: {}", path.display()))?;
        Self::try_from(s.to_string())
    }
}

/// リソースのメタ情報（フルデコードを伴わずに取得できる素性）。
#[derive(Debug, Clone)]
pub struct ResourceMeta {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
}

/// 画像リソースの読み書きポート。
///
/// スレッド間共有・`spawn_blocking` への move のため `Send + Sync`。
/// 失敗は隠蔽 fallback せず raw に surface する（`anyhow::Result`）。
pub trait ImageRepository: Send + Sync {
    /// 生バイト列を取得する。事後: 返るバイト列はデコード可能な原本（整形しない）。
    /// リソース全体をメモリに載せる（ストレージ非依存にするための意図的な選択。
    /// Drive 等クラウドでもダウンロードは全取得となるため、ストリーミングは持たない）。
    fn read(&self, loc: &ResourceLocation) -> Result<Vec<u8>>;

    /// バイト列を永続化する。事後: 成功時 `loc` は `bytes` を保持する。
    /// FS 実装は同ボリューム内 temp へ書いてから rename する（置換の完了性）。
    fn write(&self, loc: &ResourceLocation, bytes: &[u8]) -> Result<()>;

    /// 出力先が既存かを返す（別名保存の衝突検出に用いる）。
    fn exists(&self, loc: &ResourceLocation) -> Result<bool>;

    /// メタ情報を効率的に取得する（FS はヘッダのみ読む）。
    fn metadata(&self, loc: &ResourceLocation) -> Result<ResourceMeta>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    use crate::imaging::codec;

    /// 非FSバックエンドでもポートが実装可能なことを示す最小の in-memory 実装。
    /// Google Drive 等の差し込み性（ポート境界の妥当性）を担保する検証用。
    struct InMemoryRepository {
        store: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl ImageRepository for InMemoryRepository {
        fn read(&self, loc: &ResourceLocation) -> Result<Vec<u8>> {
            self.store
                .lock()
                .unwrap()
                .get(loc.as_str())
                .cloned()
                .ok_or_else(|| anyhow!("not found: {}", loc.as_str()))
        }

        fn write(&self, loc: &ResourceLocation, bytes: &[u8]) -> Result<()> {
            self.store
                .lock()
                .unwrap()
                .insert(loc.as_str().to_string(), bytes.to_vec());
            Ok(())
        }

        fn exists(&self, loc: &ResourceLocation) -> Result<bool> {
            Ok(self.store.lock().unwrap().contains_key(loc.as_str()))
        }

        fn metadata(&self, loc: &ResourceLocation) -> Result<ResourceMeta> {
            // 非FSバックエンドはヘッダのみ読取の近道がないので、バイト列から素性を得る。
            let bytes = self.read(loc)?;
            let loaded = codec::decode_rgba(&bytes)?;
            Ok(ResourceMeta {
                name: loc.as_str().to_string(),
                width: loaded.width,
                height: loaded.height,
                format: loaded.format,
            })
        }
    }

    #[test]
    fn in_memory_backend_satisfies_port() {
        let repo = InMemoryRepository {
            store: Mutex::new(HashMap::new()),
        };
        let img = image::RgbaImage::from_pixel(3, 2, image::Rgba([10, 20, 30, 255]));
        let bytes = codec::encode_to_bytes(&img, ImageFormat::Png, 90).unwrap();
        let loc = ResourceLocation::try_from("mem://a.png".to_string()).unwrap();

        assert!(!repo.exists(&loc).unwrap());
        repo.write(&loc, &bytes).unwrap();
        assert!(repo.exists(&loc).unwrap());
        assert_eq!(repo.read(&loc).unwrap(), bytes);

        let meta = repo.metadata(&loc).unwrap();
        assert_eq!((meta.width, meta.height), (3, 2));
        assert_eq!(meta.format, ImageFormat::Png);
    }

    #[test]
    fn empty_location_is_rejected() {
        assert!(ResourceLocation::try_from(String::new()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_path_is_rejected() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        // 0x80 は単独では不正な UTF-8 バイト。lossy 変換せず Err になるべき。
        let invalid = Path::new(OsStr::from_bytes(&[0x66, 0x6f, 0x80, 0x6f]));
        assert!(ResourceLocation::try_from(invalid).is_err());
    }
}
