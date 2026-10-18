// Loading catalog files over URLs, for the browser and for Velvet Desktop: each file is fetched,
// keeps its name, and the files go to a locale, which sorts them by name.

import { LocaleError } from "./format";
import { Locale, type CatalogFile, type LocaleOptions } from "./locale";

/** The part of a fetch response that loading uses. A browser Response has all of it. */
export interface CatalogResponse {
  readonly ok: boolean;
  readonly status: number;
  text(): Promise<string>;
}

/** Fetches one URL and answers its response. The default is the global `fetch`. */
export type CatalogFetch = (url: string) => Promise<CatalogResponse>;

const defaultFetch: CatalogFetch = (url) => fetch(url);

/**
 * Fetches catalog files, in parallel.
 * @param urls - the URLs of the files. Each file keeps the name at the end of its URL.
 * @param fetchFn - the fetch to use. Defaults to the global `fetch`.
 * @throws LocaleError with code "fetch" when a response is not ok.
 */
export async function readCatalogUrls(
  urls: readonly string[],
  fetchFn: CatalogFetch = defaultFetch,
): Promise<CatalogFile[]> {
  return Promise.all(
    urls.map(async (url) => {
      const response = await fetchFn(url);
      if (!response.ok) {
        throw new LocaleError("fetch", `${url} answered with status ${response.status}`);
      }
      return { name: fileNameOf(url), text: await response.text() };
    }),
  );
}

/**
 * Loads one language from catalog URLs: fetches the files, then builds the locale from them.
 * @param language - the language tag, in canonical case.
 * @param urls - the catalog file URLs.
 * @param fetchFn - the fetch to use. Defaults to the global `fetch`.
 * @param options - an optional reference locale for the holes check.
 */
export async function loadLocaleFromUrls(
  language: string,
  urls: readonly string[],
  fetchFn?: CatalogFetch,
  options?: LocaleOptions,
): Promise<Locale> {
  return new Locale(language, await readCatalogUrls(urls, fetchFn), options);
}

/** The file name at the end of a URL, without its query string or fragment. */
export function fileNameOf(url: string): string {
  const path = url.split(/[?#]/u)[0] ?? url;
  return path.slice(path.lastIndexOf("/") + 1);
}
