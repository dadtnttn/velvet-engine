// Preloading: decodes the pictures of art ids before they are drawn, so that the first frame that
// uses them does not stall. In a browser each file is decoded with Image.decode(). A file that the
// browser cannot decode is marked broken, and the next format is tried. Outside a browser (Node,
// tests) there is no Image, so nothing is decoded and every request counts as done at once.

import type { ArtResolver } from "./resolver";
import type { ArtRequest, ArtSource } from "./types";

/** How far a preload has got. Sent after each request is done, in the order that they finish. */
export interface PreloadProgress {
  /** The requests done so far, this one included. */
  readonly done: number;
  /** The distinct requests of the call. */
  readonly total: number;
  /** The id of the request that just finished. */
  readonly id: string;
  /** The variant of that request, or null. */
  readonly variant: string | null;
  /** What the request ended with: the file that decoded, or a placeholder. */
  readonly source: ArtSource;
}

/** The outcome of a preload. */
export interface PreloadResult {
  /** The paths of the files that could not be decoded, sorted. Their requests went on to another file, or to a placeholder. */
  readonly broken: readonly string[];
}

/**
 * Decodes one image in the browser. Resolves to false when the browser cannot read or decode it.
 * It uses Image.decode() when the browser has it, and the load and error events otherwise.
 * @param url - the address of the image.
 */
function decodeImage(url: string): Promise<boolean> {
  return new Promise<boolean>((resolve) => {
    const image = new Image();
    if (typeof image.decode === "function") {
      image.src = url;
      image.decode().then(
        () => resolve(true),
        () => resolve(false),
      );
    } else {
      image.onload = () => resolve(true);
      image.onerror = () => resolve(false);
      image.src = url;
    }
  });
}

/**
 * The requests of a call as objects, with repeats removed. The first of each repeated pair is kept,
 * in the order of the call.
 * @param requests - art ids, or requests with a variant.
 */
function distinctRequests(requests: readonly (string | ArtRequest)[]): ArtRequest[] {
  const seen = new Set<string>();
  const distinct: ArtRequest[] = [];
  for (const item of requests) {
    const request: ArtRequest = typeof item === "string" ? { id: item } : item;
    const key = `${request.id}#${request.variant ?? ""}`;
    if (!seen.has(key)) {
      seen.add(key);
      distinct.push(request);
    }
  }
  return distinct;
}

/**
 * Settles one request: decodes the picture it resolves to. A file that fails is marked broken, and
 * the resolver then picks the next file, until one decodes or a placeholder is reached.
 * @returns the source that the request ended with.
 */
async function settle(
  resolver: ArtResolver,
  request: ArtRequest,
  decodable: boolean,
  broken: string[],
): Promise<ArtSource> {
  for (;;) {
    const picture = resolver.resolve(request.id, request.variant);
    const path = picture.path;
    if (!decodable || path === null || (await decodeImage(picture.url))) {
      return picture.source;
    }
    broken.push(path);
    resolver.markBroken(path);
  }
}

/**
 * Decodes the pictures of some art ids, so that they are ready when they are drawn.
 * @param resolver - the resolver that finds the files.
 * @param requests - art ids, or requests with a variant. Repeats are decoded once.
 * @param onProgress - called after each distinct request is done.
 * @returns the files that could not be decoded.
 * @throws ArtError with code "unknown-id" or "unknown-variant" for a request the manifest does not
 *   declare. Every request is checked before any image is decoded.
 */
export async function preloadArt(
  resolver: ArtResolver,
  requests: readonly (string | ArtRequest)[],
  onProgress?: (progress: PreloadProgress) => void,
): Promise<PreloadResult> {
  const distinct = distinctRequests(requests);
  for (const request of distinct) {
    resolver.resolve(request.id, request.variant);
  }
  const decodable = typeof Image === "function";
  const broken: string[] = [];
  let done = 0;
  await Promise.all(
    distinct.map(async (request) => {
      const source = await settle(resolver, request, decodable, broken);
      done += 1;
      onProgress?.({
        done,
        total: distinct.length,
        id: request.id,
        variant: request.variant ?? null,
        source,
      });
    }),
  );
  return { broken: broken.sort() };
}
