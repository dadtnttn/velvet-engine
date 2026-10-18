// Decodes the cells of a tile layer, or of one chunk of an infinite layer. Tiled writes the cells as a
// CSV list of numbers, or as base64 of little-endian 32-bit numbers, optionally compressed with gzip or
// zlib (both read with the platform's DecompressionStream, so decoding is asynchronous). Zstandard is
// refused. Each id is split into its tile id and its flip flags (see gid.ts).

import { GID_LIMIT, splitGid } from "./gid";
import { itemPath, type Problems } from "./reader";

/** What a layer, or a chunk, says about its data. */
export interface CellSource {
  /** The raw JSON value of the `data` field. */
  readonly data: unknown;
  /** How the cells are written: "csv" (a list of numbers) or "base64". */
  readonly encoding: "csv" | "base64";
  /** The compression of the base64 text. "" when none. */
  readonly compression: "" | "gzip" | "zlib" | "zstd";
  /** The number of columns of the cells. */
  readonly width: number;
  /** The number of rows of the cells. */
  readonly height: number;
  /** The path of the `data` field, for problems. */
  readonly path: string;
  /** Says whether a global tile id names a tile of the map. */
  readonly knownGid: (gid: number) => boolean;
}

/** The decoded cells: the id and the flips of each, row by row. */
interface CellData {
  readonly gids: Uint32Array;
  readonly flips: Uint8Array;
}

/**
 * Decodes the cells of one layer or chunk.
 * @param problems - where the problems go.
 * @param source - what the data says about itself.
 * @returns the cells, or undefined when the data has a problem (the problem is recorded).
 */
export async function decodeCells(problems: Problems, source: CellSource): Promise<CellData | undefined> {
  if (source.compression === "zstd") {
    problems.add(
      "unsupported",
      "zstd compression is not supported: save the map with gzip, zlib or no compression",
      source.path,
    );
    return undefined;
  }
  const ids = source.encoding === "csv" ? readCsv(problems, source) : await readBase64(problems, source);
  if (ids === undefined) return undefined;

  const count = source.width * source.height;
  if (ids.length !== count) {
    problems.add(
      "data",
      `the data has ${ids.length} cells, but the layer is ${source.width} x ${source.height} = ${count}`,
      source.path,
    );
    return undefined;
  }

  const gids = new Uint32Array(count);
  const flips = new Uint8Array(count);
  let problem = false;
  let reportedUnknown = false;
  for (let index = 0; index < count; index++) {
    const split = splitGid(ids[index] ?? 0);
    if (split.hexRotation) {
      problems.add("value", "the hexagonal rotation flag is only for hexagonal maps", source.path);
      return undefined;
    }
    if (split.gid !== 0 && !source.knownGid(split.gid) && !reportedUnknown) {
      const column = index % source.width;
      const row = Math.floor(index / source.width);
      problems.add(
        "value",
        `the cell (${column}, ${row}) uses the tile id ${split.gid}, which no tileset has`,
        source.path,
      );
      reportedUnknown = true;
      problem = true;
    }
    gids[index] = split.gid;
    flips[index] = split.flips;
  }
  return problem ? undefined : { gids, flips };
}

/** Reads a CSV list: each item is a tile id, as a number. Compression is not allowed with CSV. */
function readCsv(problems: Problems, source: CellSource): Uint32Array | undefined {
  if (!Array.isArray(source.data)) {
    problems.add("structure", "csv data must be a list of numbers", source.path);
    return undefined;
  }
  if (source.compression !== "") {
    problems.add("value", "compressed data must be base64, not csv", source.path);
    return undefined;
  }
  const ids = new Uint32Array(source.data.length);
  let valid = true;
  source.data.forEach((value: unknown, index) => {
    if (typeof value !== "number" || !Number.isInteger(value) || value < 0 || value > GID_LIMIT) {
      problems.add("value", `the cell is not a tile id from 0 to ${GID_LIMIT}`, itemPath(source.path, index));
      valid = false;
      return;
    }
    ids[index] = value;
  });
  return valid ? ids : undefined;
}

/** Reads base64 text, decompresses it when it is gzip or zlib, and reads its little-endian 32-bit ids. */
async function readBase64(problems: Problems, source: CellSource): Promise<Uint32Array | undefined> {
  if (typeof source.data !== "string") {
    problems.add("structure", "base64 data must be a string", source.path);
    return undefined;
  }
  let bytes: Uint8Array<ArrayBuffer>;
  try {
    bytes = decodeBase64(source.data);
  } catch {
    problems.add("data", "the data is not valid base64", source.path);
    return undefined;
  }
  if (source.compression === "gzip" || source.compression === "zlib") {
    try {
      bytes = await inflate(bytes, source.compression === "gzip" ? "gzip" : "deflate");
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      problems.add("data", `the ${source.compression} data is corrupt: ${detail}`, source.path);
      return undefined;
    }
  }
  if (bytes.byteLength % 4 !== 0) {
    problems.add("data", `the data is ${bytes.byteLength} bytes, not a whole number of 32-bit cells`, source.path);
    return undefined;
  }
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const ids = new Uint32Array(bytes.byteLength / 4);
  for (let index = 0; index < ids.length; index++) {
    ids[index] = view.getUint32(index * 4, true);
  }
  return ids;
}

/** Decodes base64 text into bytes. Throws when the text is not valid base64. */
function decodeBase64(text: string): Uint8Array<ArrayBuffer> {
  const binary = atob(text);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

/** Decompresses bytes with the platform's stream: "gzip" is the gzip format, "deflate" is the zlib format. */
async function inflate(bytes: Uint8Array<ArrayBuffer>, format: "gzip" | "deflate"): Promise<Uint8Array<ArrayBuffer>> {
  const input = new ReadableStream<BufferSource>({
    start(controller) {
      controller.enqueue(bytes);
      controller.close();
    },
  });
  const reader = input.pipeThrough(new DecompressionStream(format)).getReader();
  const parts: Uint8Array<ArrayBuffer>[] = [];
  let length = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    parts.push(value);
    length += value.byteLength;
  }
  const out = new Uint8Array(length);
  let offset = 0;
  for (const part of parts) {
    out.set(part, offset);
    offset += part.byteLength;
  }
  return out;
}
