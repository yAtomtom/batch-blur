// 依存なしで 512x512 の RGBA PNG プレースホルダを生成する（tauri icon の元画像用）。
// 中央に淡い円を置いたアクセントカラーの単純な図。差し替え前提。
import { deflateSync } from "node:zlib";
import { writeFileSync, mkdirSync } from "node:fs";

const SIZE = 512;

// CRC32（PNG チャンク用）
const crcTable = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = crcTable[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

// ピクセル生成（RGBA, 各行の先頭にフィルタバイト 0）
const raw = Buffer.alloc(SIZE * (1 + SIZE * 4));
const [br, bg, bb] = [0x3b, 0x82, 0xf6]; // 背景 accent
const [cr, cg, cb] = [0xff, 0xff, 0xff]; // 円 白
const cx = SIZE / 2;
const cy = SIZE / 2;
const r = SIZE * 0.32;
for (let y = 0; y < SIZE; y++) {
  const rowStart = y * (1 + SIZE * 4);
  raw[rowStart] = 0; // filter: none
  for (let x = 0; x < SIZE; x++) {
    const d = Math.hypot(x - cx, y - cy);
    const inside = d < r;
    const p = rowStart + 1 + x * 4;
    raw[p] = inside ? cr : br;
    raw[p + 1] = inside ? cg : bg;
    raw[p + 2] = inside ? cb : bb;
    raw[p + 3] = 0xff;
  }
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type RGBA
ihdr[10] = 0;
ihdr[11] = 0;
ihdr[12] = 0;

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", deflateSync(raw)),
  chunk("IEND", Buffer.alloc(0)),
]);

mkdirSync("assets", { recursive: true });
writeFileSync("assets/icon.png", png);
console.log(`wrote assets/icon.png (${png.length} bytes, ${SIZE}x${SIZE})`);
