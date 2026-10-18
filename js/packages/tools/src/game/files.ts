// Finds out whether a file of the game exists, spelled exactly as the data names it. The formats that name files
// (audio manifests, Tiled maps) are read by servers that tell upper and lower case apart, so a name that differs
// only in case is missing here too, even on a case-blind file system such as Windows's.

import { readdir } from "node:fs/promises";
import { join, parse, relative, resolve, sep } from "node:path";

/** Answers whether files exist with the exact spelling of their names. It keeps each folder it has read. */
export class FileIndex {
  private readonly listings = new Map<string, Promise<ReadonlyMap<string, boolean> | undefined>>();

  /**
   * True when a file exists at a path, and every name on the way has exactly the spelling that the path gives.
   * @param path - the path of the file. A relative path is taken from the working folder.
   */
  async isFile(path: string): Promise<boolean> {
    const full = resolve(path);
    const { root } = parse(full);
    const names = relative(root, full).split(sep).filter((name) => name !== "");
    let folder = root;
    for (const [index, name] of names.entries()) {
      const entries = await this.listing(folder);
      const isFolder = entries?.get(name);
      if (isFolder === undefined) return false;
      if (index === names.length - 1) return !isFolder;
      if (!isFolder) return false;
      folder = join(folder, name);
    }
    return false;
  }

  /**
   * The names in a folder, each with true for a folder and false for a file. A folder that cannot be read has none.
   * @param folder - the absolute path of the folder.
   */
  private listing(folder: string): Promise<ReadonlyMap<string, boolean> | undefined> {
    let found = this.listings.get(folder);
    if (found === undefined) {
      found = readdir(folder, { withFileTypes: true }).then(
        (entries) => new Map(entries.map((entry): [string, boolean] => [entry.name, entry.isDirectory()])),
        () => undefined,
      );
      this.listings.set(folder, found);
    }
    return found;
  }
}
