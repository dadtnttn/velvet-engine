// Tests for decoding tile cells: CSV and base64 (plain, gzip and zlib), the split of the flip flags from the
// ids, and the problems that a layer's data can have. Also the split itself, on its own.

import { deflateSync, gzipSync } from "node:zlib";
import { describe, expect, it } from "vitest";
import { decodeCells, type CellSource } from "./data";
import { splitGid } from "./gid";
import { Problems } from "./reader";
import { TILE_FLIP_DIAGONAL, TILE_FLIP_HORIZONTAL, TILE_FLIP_VERTICAL } from "./types";
import { base64Cells } from "./test-support";

/** A source for a 4 by 1 layer of ids 1 to 8 are known; everything else is unknown. */
function source(overrides: Partial<CellSource>): CellSource {
  return {
    data: [],
    encoding: "csv",
    compression: "",
    width: 4,
    height: 1,
    path: "layers[0].data",
    knownGid: (gid) => gid >= 1 && gid <= 8,
    ...overrides,
  };
}

/** Decodes a source and returns the cells and the problems. */
async function decode(overrides: Partial<CellSource>): Promise<{
  cells: Awaited<ReturnType<typeof decodeCells>>;
  problems: Problems;
}> {
  const problems = new Problems();
  const cells = await decodeCells(problems, source(overrides));
  return { cells, problems };
}

describe("the split of a tile id", () => {
  it("separates the id from the three flip bits", () => {
    expect(splitGid(2)).toEqual({ gid: 2, flips: 0, hexRotation: false });
    expect(splitGid(0x80000000 + 2).flips).toBe(TILE_FLIP_HORIZONTAL);
    expect(splitGid(0x40000000 + 2).flips).toBe(TILE_FLIP_VERTICAL);
    expect(splitGid(0x20000000 + 2).flips).toBe(TILE_FLIP_DIAGONAL);
    const all = splitGid(0xe0000000 + 2);
    expect(all).toEqual({ gid: 2, flips: 7, hexRotation: false });
  });

  it("reports the hexagonal rotation bit apart from the flips", () => {
    expect(splitGid(0x10000000 + 5)).toEqual({ gid: 5, flips: 0, hexRotation: true });
  });
});

describe("decoding CSV cells", () => {
  it("splits the flips from each id", async () => {
    const { cells } = await decode({ width: 6, data: [2147483650, 1073741826, 536870914, 3758096386, 0, 2] });
    expect(Array.from(cells?.gids ?? [])).toEqual([2, 2, 2, 2, 0, 2]);
    expect(Array.from(cells?.flips ?? [])).toEqual([1, 2, 4, 7, 0, 0]);
  });

  it("refuses a list whose length is not the size of the layer", async () => {
    const { cells, problems } = await decode({ data: [1, 2, 3] });
    expect(cells).toBeUndefined();
    expect(problems.issues[0]).toEqual(expect.objectContaining({ code: "data", path: "layers[0].data" }));
  });

  it("refuses a value that is not a tile id", async () => {
    const { problems } = await decode({ data: [1, -1, 2, 3] });
    expect(problems.issues[0]).toEqual(expect.objectContaining({ code: "value", path: "layers[0].data[1]" }));
  });

  it("refuses csv data that claims to be compressed", async () => {
    const { problems } = await decode({ data: [1, 2, 3, 4], compression: "gzip" });
    expect(problems.issues[0]?.code).toBe("value");
  });

  it("refuses an id that no tileset has, and names the first such cell", async () => {
    const { cells, problems } = await decode({ data: [1, 9, 0, 12] });
    expect(cells).toBeUndefined();
    expect(problems.issues).toHaveLength(1);
    expect(problems.issues[0]?.message).toContain("(1, 0)");
    expect(problems.issues[0]?.message).toContain("9");
  });

  it("refuses the hexagonal rotation bit in an orthogonal map", async () => {
    const { problems } = await decode({ data: [1, 0x10000001, 0, 0] });
    expect(problems.issues[0]?.code).toBe("value");
  });

  it("refuses a list that is not a list", async () => {
    const { problems } = await decode({ data: "1,2,3,4" });
    expect(problems.issues[0]?.code).toBe("structure");
  });
});

describe("decoding base64 cells", () => {
  const ids = [2, 2147483650, 0, 4, 1073741826, 0, 0, 8];
  const flat = [2, 2, 0, 4, 2, 0, 0, 8];
  const flatFlips = [0, 1, 0, 0, 2, 0, 0, 0];

  it("reads plain base64 of little-endian 32-bit ids", async () => {
    const { cells } = await decode({ encoding: "base64", data: base64Cells(ids), width: 4, height: 2 });
    expect(Array.from(cells?.gids ?? [])).toEqual(flat);
    expect(Array.from(cells?.flips ?? [])).toEqual(flatFlips);
  });

  it("reads gzip base64 with the gzip format", async () => {
    const bytes = Buffer.alloc(ids.length * 4);
    ids.forEach((id, index) => bytes.writeUInt32LE(id, index * 4));
    const data = gzipSync(bytes).toString("base64");
    const { cells } = await decode({ encoding: "base64", compression: "gzip", data, width: 4, height: 2 });
    expect(Array.from(cells?.gids ?? [])).toEqual(flat);
  });

  it("reads zlib base64 with the zlib format", async () => {
    const bytes = Buffer.alloc(ids.length * 4);
    ids.forEach((id, index) => bytes.writeUInt32LE(id, index * 4));
    const data = deflateSync(bytes).toString("base64");
    const { cells } = await decode({ encoding: "base64", compression: "zlib", data, width: 4, height: 2 });
    expect(Array.from(cells?.flips ?? [])).toEqual(flatFlips);
  });

  it("refuses zstd, which is not supported", async () => {
    const { problems } = await decode({ encoding: "base64", compression: "zstd", data: "AAAA" });
    expect(problems.issues[0]?.code).toBe("unsupported");
  });

  it("refuses text that is not base64", async () => {
    const { problems } = await decode({ encoding: "base64", data: "!!!!" });
    expect(problems.issues[0]).toEqual(expect.objectContaining({ code: "data", path: "layers[0].data" }));
  });

  it("refuses gzip data that is corrupt", async () => {
    const { problems } = await decode({ encoding: "base64", compression: "gzip", data: "bm90IGd6aXA=" });
    expect(problems.issues[0]?.code).toBe("data");
  });

  it("refuses base64 whose length is not a whole number of cells", async () => {
    const { problems } = await decode({ encoding: "base64", data: Buffer.from([1, 2, 3]).toString("base64") });
    expect(problems.issues[0]?.code).toBe("data");
  });

  it("refuses base64 with the wrong number of cells", async () => {
    const { problems } = await decode({ encoding: "base64", data: base64Cells([1, 2]), width: 4, height: 1 });
    expect(problems.issues[0]?.code).toBe("data");
  });
});
