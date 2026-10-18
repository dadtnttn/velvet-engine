// The error type of @velvet/tiles. Code that handles a failure branches on `code`; the message is for
// logs and developers. A map that is refused lists every problem it has in `issues`, each with the path
// of the value that broke the rule (for example `layers[2].data`), so one run shows all of them.

/**
 * Why a call failed:
 * - "json": the map or tileset text is not JSON;
 * - "structure": a field is missing, or it has the wrong JSON type;
 * - "value": a value breaks a rule: a range, a grammar, or a reference to a tileset or a tile that does not exist;
 * - "duplicate": two tilesets share a name, two tilesets share a tile id, two tiles of one tileset share an id,
 *   or two properties share a name;
 * - "unsupported": the file uses a part of Tiled's format that this reader refuses (docs/formats/TILEMAP.md);
 * - "data": a layer's data does not decode: bad base64, a corrupt compressed stream, or the wrong number of cells;
 * - "loader": an external tileset could not be loaded, or the map needs one and no loader was given;
 * - "missing-image": a draw call needs the image of a tileset that the caller did not supply;
 * - "bad-input": an argument of a call (a grid, a box, a path option, a cost) is not valid.
 */
export type TilesErrorCode =
  | "json"
  | "structure"
  | "value"
  | "duplicate"
  | "unsupported"
  | "data"
  | "loader"
  | "missing-image"
  | "bad-input";

/** One problem found in a document or in a call. */
export interface TilesIssue {
  /** The rule that broke. */
  readonly code: TilesErrorCode;
  /** A sentence in English for logs and developers. Its wording is not part of the format. */
  readonly message: string;
  /** Where the problem is, such as `layers[2].data`. Absent when it concerns the whole document. */
  readonly path?: string;
}

/** An error thrown by this package. */
export class TilesError extends Error {
  /** Why the call failed. */
  readonly code: TilesErrorCode;
  /** Every problem found. Never empty: a single failure has one issue. */
  readonly issues: readonly TilesIssue[];

  /**
   * @param code - the failure reason.
   * @param message - a sentence for logs and developers.
   * @param issues - the problems found, when there are several. A single issue is made from the code and message.
   * @param options - an optional `cause`, such as the error of a loader or of a decoder.
   */
  constructor(code: TilesErrorCode, message: string, issues: readonly TilesIssue[] = [], options?: ErrorOptions) {
    super(message, options);
    this.name = "TilesError";
    this.code = code;
    this.issues = issues.length > 0 ? issues : [{ code, message }];
  }
}
