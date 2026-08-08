import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Tauri は固定ポートの dev server を前提とする。
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  // Tauri: フロントの clearScreen 抑止で Rust エラーを見えるようにする
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      // src-tauri の変更は Vite が監視しない（Rust 側は tauri が担当）
      ignored: ["**/src-tauri/**"],
    },
  },
  // vitest 設定（jsdom不要な純粋ロジックのみ対象。node環境で実行）
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
