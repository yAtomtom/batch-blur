# Batch Blur

複数画像に **ガウス / ブロック(box)ブラー** を一律適用して一括保存するデスクトップアプリ。
Tauri v2 + Rust + React/TypeScript。Windows 対応（将来 macOS）。

## 機能（MVP）

- 複数画像をドラッグ&ドロップ／ファイルダイアログで読み込み（png/jpg/jpeg/webp/bmp）
- ファイル一覧を選択するとフィルタ適用プレビューを表示（↑↓ キーで移動）
- フィルタ種類（ガウス／ブロック）と強さ（半径）を指定
- 上書き保存 / 別名保存（prefix・suffix を独立指定、出力先選択）
- フィルタ設定の Undo(Ctrl+Z) / Redo(Ctrl+Y)
- 異なるサイズ・拡張子の混在に対応、テーマ（ライト/ダーク/システム）

## アーキテクチャ

```
src-tauri/src/
  domain/    純粋コア（image 非依存・単体テスト容易）
    filter.rs  FilterKind, AxisStrength, Region, FilterSpec, FilterStack
    save.rs    SaveMode, resolve_output_path, 衝突検出
    preset.rs  FilterPreset（将来のプリセット入出力シーム）
  imaging/   アダプタ（image/imageproc 依存）
    blur.rs    gaussian / 自前 box blur / 半径スケール補正
    io.rs      decode(EXIF正規化) / encode / 原子的書き込み
  commands.rs  Tauri コマンド（load/preview/export/cancel）
  types.rs     IPC 型
src/         React フロント（features/ 単位で関心分離、providers でテーマ/レイアウト）
```

将来拡張シーム: レイヤー(`features/editor/Layer/`)、選択領域(`Selection/`, `Region`)、
X/Y 別強度(`AxisStrength`)、フィルタ重ね掛け(`FilterStack`)、プリセット(`FilterPreset`)。

## 前提ツール（要インストール）

このリポジトリには Node 依存のみ導入済み。**Rust ツールチェインと OS 依存ライブラリは未導入**。

### 1. Rust（rustup）

```bash
# ~/.bashrc を書き換えたくない場合は --no-modify-path を付ける
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. OS 依存（開発/ビルドに必要）

- **Windows**: [Microsoft C++ Build Tools] と WebView2（Win11 は同梱）。
- **Linux/WSL でビルド確認する場合**（GUI は WSLg 必須）:
  ```bash
  sudo apt update && sudo apt install -y libwebkit2gtk-4.1-dev build-essential \
    curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev pkg-config
  ```
- **macOS（将来）**: Xcode Command Line Tools。

### 3. アイコン生成（初回のみ）

`tauri.conf.json` が `src-tauri/icons/*` を参照する。任意の PNG から生成:

```bash
npx tauri icon path/to/icon.png
```

## 開発・ビルド

```bash
npm install            # 導入済み
npm run tauri dev      # 開発起動（Vite + Rust ホットリロード）
npm run tauri build    # リリースビルド（Windows は NSIS インストーラ）
```

## テスト

```bash
npm test                        # フロント: EditHistory の単体テスト（vitest）
npx tsc --noEmit                # 型チェック
cd src-tauri && cargo test      # Rust: domain/imaging の単体テスト
```

## 検証状況（このコミット時点）

- ✅ `npm test`（EditHistory 7 件、`redo(undo(h)) === h` 含む）
- ✅ `npx tsc --noEmit`（型チェック通過）
- ✅ `npx vite build`（フロント 53 module バンドル成功）
- ✅ **Rust コア（domain + imaging）**: image 0.25.10 / imageproc 0.25.1 でコンパイル成功、
  単体テスト **25 件パス**（blur カーネル/box エッジclamp/encode 往復/パス解決/衝突検出/契約、
  `load_rgba` の EXIF API もコンパイル確認済み）。tauri/webkit 非依存の単体クレートで検証。
- ⏳ Tauri グルー層（`commands.rs`/`types.rs`/`lib.rs`）と GUI 起動は、webkit 導入後の
  `cargo build` / `npm run tauri dev` で最終確認（薄い glue のみ・コアは検証済み）。

### Rust ビルドで詰まりやすい点

- **EXIF 向き正規化** (`src-tauri/src/imaging/io.rs` の `load_rgba`) は image 0.25.5+ の
  `into_decoder`/`orientation`/`apply_orientation` API を使用。もし版差でコンパイルが通らない
  場合、この 1 関数のみ `DynamicImage` 直接デコードへ差し替えれば回避できる（他へ波及しない）。
- **WebP はロスレスのみ**（image クレート制約）。lossy が必要なら `webp` crate を追加。

## 既知の制約 / 将来対応

- 出力衝突は自動リネームせずエラー（隠蔽 fallback を避ける方針）
- 一括書き出しは MVP で逐次実行（`rayon` 並列化は動作確認後に導入予定）
- 構造化エラーコード、ICC/色プロファイル完全保持、tauri-specta 型自動生成は将来対応

## ライセンス

MIT License. 詳細は [LICENSE](LICENSE) を参照。
