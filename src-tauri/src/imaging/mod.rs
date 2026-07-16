//! imaging アダプタ: bytes<->pixels の純粋コーデックとブラーカーネル。
//! image / imageproc に依存する層（ドメインは非依存に保つ）。ファイルシステム
//! アクセスは持たず、ストレージ入出力は repository モジュールへ分離する。

pub mod blur;
pub mod codec;
