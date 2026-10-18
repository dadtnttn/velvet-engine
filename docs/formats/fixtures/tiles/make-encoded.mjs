// One-off script for the tile fixtures. It reads csv.tmj and writes the same map three more times, with the
// cells of every tile layer encoded differently: base64 without compression, base64 of gzip, and base64 of
// zlib. The cells are little-endian 32-bit numbers, as Tiled writes them. Run it from any folder with
//   node docs/formats/fixtures/tiles/make-encoded.mjs
// and it writes base64.tmj, gzip.tmj and zlib.tmj next to csv.tmj. Re-run it only if csv.tmj changes.

import { deflateSync, gzipSync } from "node:zlib";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const source = JSON.parse(readFileSync(join(here, "csv.tmj"), "utf8"));

/** The three encodings: the output file, the compression that the map names, and the function that compresses. */
const variants = [
  { file: "base64.tmj", compression: "", compress: (bytes) => bytes },
  { file: "gzip.tmj", compression: "gzip", compress: (bytes) => gzipSync(bytes) },
  { file: "zlib.tmj", compression: "zlib", compress: (bytes) => deflateSync(bytes) },
];

for (const variant of variants) {
  const map = structuredClone(source);
  map.layers = source.layers.map((layer) => {
    if (layer.type !== "tilelayer") return layer;
    const { data, ...rest } = layer;
    const words = Buffer.alloc(data.length * 4);
    data.forEach((id, index) => words.writeUInt32LE(id, index * 4));
    const encoded = { ...rest, encoding: "base64", data: variant.compress(words).toString("base64") };
    if (variant.compression !== "") encoded.compression = variant.compression;
    return encoded;
  });
  writeFileSync(join(here, variant.file), `${JSON.stringify(map, null, 2)}\n`);
  console.log(`wrote ${variant.file}`);
}
