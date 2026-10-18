# @velvet/i18n

The translated texts of a Velvet JS game. It reads catalog files in the `velvet-locale` format
(`docs/formats/LOCALE.md`), fills the holes of each text, plans the plural form, and falls back from
one language to the next when a text is missing.

It has no dependencies. It runs in the browser, in Velvet Desktop and in Node (for tests), and it
never reads files itself: the caller hands it the file names and their text.

## Layout of a game's texts

    locales/
      es/                  the source language: its id keys hold the source text
        00_menus.json
        10_shop.json
      en/
        00_menus.json
        10_shop.json
      pt-BR/
        10_shop.json       only the shop is translated; the menus fall back to en

Each file is a catalog:

```json
{
  "format": "velvet-locale",
  "version": 1,
  "language": "en",
  "entries": {
    "menu.start": "Start",
    "shop.buy": "Buy {item} for {gold} coins",
    "shop.coins": { "one": "{count} coin", "other": "{count} coins" }
  }
}
```

## Usage

```ts
import { I18n, loadLocaleFromUrls } from "@velvet/i18n";

const i18n = new I18n({
  source: "es",
  fallback: "en",
  onMissing: (key, language) => console.warn(`no text for ${key} in ${language}`),
  onIssue: (issue) => console.warn(issue.message),
});

// The source language first: the other languages are checked against it.
await i18n.addLanguageFromUrls("es", ["locales/es/00_menus.json", "locales/es/10_shop.json"]);
await i18n.addLanguageFromUrls("en", ["locales/en/00_menus.json", "locales/en/10_shop.json"]);
await i18n.addLanguageFromUrls("pt-BR", ["locales/pt-BR/10_shop.json"]);

i18n.setLanguage("en");
i18n.t("menu.start");                              // "Start"
i18n.t("shop.buy", { item: "lantern", gold: 5 }); // "Buy lantern for 5 coins"
i18n.plural("shop.coins", 1);                      // "1 coin"

i18n.setLanguage("pt-BR");
i18n.t("menu.start");                              // "Start": pt-BR has no entry, so en answers
i18n.lookup("menu.start");                         // { text: "Start", language: "en" }
i18n.direction;                                    // "ltr"

const stop = i18n.onChange((language) => updateMenus(language));
```

A game can also read one language without the chain:

```ts
const locale = await loadLocaleFromUrls("en", ["locales/en/00_menus.json"]);
locale.t("menu.start"); // "Start"; a key with no entry is shown as itself
```

## The lookup chain

For the language in use, a key is looked up in this order, and the first catalog that has it
answers: the language, its base (`pt-BR`, then `pt`), the fallback language, the source language.
If none has it, the key itself is shown as a template, and `onMissing` is called once per language
and key. The source language never reports a missing key, because its text is the key.

## API

| Name | What it does |
|---|---|
| `new I18n({ source, fallback?, onMissing?, onIssue? })` | The texts of a game. `fallback` defaults to `source`. |
| `addLanguage(tag, files)` | Loads a language from `{ name, text }` files. The source must be loaded first. Returns the `Locale`. |
| `addLanguageFromUrls(tag, urls, fetch?)` | Fetches the files, then loads them. |
| `setLanguage(tag)` | Makes a language the one in use. Throws `LocaleError` for a malformed tag or a tag with no catalogs. |
| `language`, `languages`, `chain`, `direction` | The language in use, the loaded tags, the lookup chain, and the direction. |
| `t(key, args?)`, `plural(key, count, args?)` | The text, with its holes filled. |
| `lookup(key, args?, count?)` | The same, with the language that answered (`undefined`: the key itself). |
| `has(key)`, `locale(tag)` | Whether a key has a text in the chain; the `Locale` of a tag. |
| `onChange(listener)` | Calls the listener with each new language. Returns a function that removes it. |
| `loadLocaleFromUrls(tag, urls, fetch?, options?)` | One language as a `Locale`, without a chain. |
| `Locale` | One language: `t`, `plural`, `has`, `keys`, `holesOf`, `issues`, `meta`, `direction`. `meta.sameAsSource` lists the keys the language keeps as the source text on purpose (LOCALE.md). |
| `parseCatalog(text, name?)`, `validateCatalog(source, translation)` | Reads one file; checks a translation against its source, with its own load issues (for tooling). |
| `parseTemplate`, `renderTemplate`, `isLanguageTag`, `baseLanguageTag`, `textDirection` | The rules, for tools and for games that need them. |

Arguments are a list for `{0}`, `{1}` and an object for `{gold}`. A hole with no argument is shown
as `{name}` and reported through `onIssue` as `missing-argument`, once per language, key and name.

## Errors and warnings

A catalog file with a problem is reported through `onIssue` (or `Locale.issues`), with a code from
`LOCALE.md`: `version`, `language-mismatch`, `value`, `template`, `holes` and the others. An error
means the data is not used, so the lookup falls back; a warning means it is used. Failed loads and
misuse throw a `LocaleError`, whose `code` says why: `language`, `unknown-language`,
`duplicate-language`, `source-missing`, `duplicate-file` or `fetch`.

## Tests

From `js/`, run `pnpm test`. `src/fixtures.test.ts` runs every case of the shared conformance suite,
`docs/formats/fixtures/locale/cases.json`, which the Rust reader will run too. The tests read the
fixtures with `node:fs`; library code never imports it.

## Known gaps

- `duplicate-key` is not reported: `JSON.parse` keeps the last of two equal keys without a trace.
- Text composition (splitting sentences, separators, quotation marks) is not here. A game that
  needs it does it before it calls `t`.
