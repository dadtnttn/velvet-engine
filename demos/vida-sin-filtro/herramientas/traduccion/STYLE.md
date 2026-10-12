# Vida Sin Filtro — translation guide

You are localizing **Vida Sin Filtro** ("Life Unfiltered"), a life simulator in the
spirit of BitLife with a Persona 5–style interface (red/black/white, bold ALL-CAPS
labels). The player lives a whole life month by month: childhood, school, jobs,
relationships, crime, routes like king/president/cult prophet/adult star, 197 countries.
The source language is Mexican-flavored Spanish. The tone is **irreverent, witty,
colloquial and adult** (sex, drugs, crime are mentioned but never explicit; nothing
sexual ever involves minors — keep it that way). Write the way a great native game
writer would, not a literal translation: jokes must land in the target language.

## Input / output

Each chunk is a JSON list of entries: `id`, `src` (Spanish), optional `holes`
(what each `{n}` contains), and `ctx` (where it is used: a function/argument or a
data field, then `file:line`). Write a JSON object `{ "<id>": "<translation>", ... }`
with **every id** of the chunk, valid JSON (escape `"` and `\`), UTF-8, no comments.

## Placeholders (critical)

* `{0}`, `{1}`… are runtime values. Keep every placeholder whose kind is NAME,
  NUMBER, MONEY or TEXT exactly once (you may reorder them). Never invent `{n}`
  numbers that the source does not have. Never translate inside the braces.
* GENDER holes receive the Spanish ending `o` (male) / `a` (female) (or `O`/`A` in
  caps, `''`/`a` for some). LOLA holes receive `lo`/`la`, ELELLA `él`/`ella`,
  UNUNA `Un`/`Una`. You can drop these holes or use a select to pick words:
  `{0|o=tired|a=tired}` → the key is the raw Spanish value, `*` is the fallback:
  `{1|lo=him|la=her|*=them}`, `{0|él=he|ella=she|*=they}`.
* The **player's** gender is also available everywhere as `{@g|m=…|f=…}`. Use it
  whenever your language needs the player's gender and the source does not mark it
  (Russian/Polish/Hebrew/Hindi past tense and adjectives, Hebrew 2nd person verbs,
  French/Italian/Portuguese participles…). Example (ru): `Fuiste al cine.` →
  `{@g|m=Ты сходил|f=Ты сходила} в кино.` Only the variable `g` exists. Keep each
  option short and make both branches complete grammatical alternatives.
* NUMBER holes can drive plural forms: `{0|#one=год|#few=года|#many=лет}` picks by
  your language's plural category of the number (one/few/many/other; Russian &
  Polish use one/few/many, most others one/other). Write the number itself with a
  plain `{0}` next to it: `{0} {0|#one=год|#few=года|#many=лет}`.
* TEXT holes are translated separately and inserted (a job title, a country name,
  another sentence). Avoid grammar that depends on that value's gender/case when
  you can (e.g. "Job: {0}" instead of an article that must agree).
* NAME holes are people's names: keep them as `{n}`.
* For other people mentioned only by name, gender is unknown unless the source
  gives it (via a GENDER hole): prefer phrasing that does not need it.
* `{` `}` that are not placeholders do not occur; never add braces.

## Form

* Address the player informally (tú → you/du/tu/ты/ty/你/너…). NPC lines inside
  «…» are dialogue: use your language's normal quotation marks.
* Keep leading/trailing spaces and the final punctuation style of the source.
* ALL-CAPS sources stay ALL-CAPS (in scripts that have case). Lowercase fragments
  (`rey padre`, `un reloj`) stay lowercase fragments that fit mid-sentence.
* UI labels (ctx mentions button, btn, tag, tab, title, menu, label, cats, toggle,
  small_button, menu_word, ctext, head, or a short ALL-CAPS source) must stay
  **short**: at most ~1.3× the source length; abbreviate if needed.
* Keep `$` amounts, numbers, `+18`, emoji and symbols (·, ›, ★, %) as they are.
* Brands stay: Velvet Engine, Velvetgram, VelvetFans, VelvetHub, Flama, Vida Sin
  Filtro may be translated as the game title in your language (e.g. "LIFE UNFILTERED").
  "Alma Gemela" (dating app) → your language's word for "Soulmate".
* Country names: standard exonym in your language. Cities: the usual name in your
  language/script (transliterate for non-Latin scripts). Demonyms (`mexicano`,
  `mexicana`) → the adjective/noun as your language writes it.
* Country clichés (`ctx` cl / cus): translate the joke; keep local slang words in
  quotes as in the source (e.g. 'ahorita') and make their meaning clear from context.
* Spanish wordplay or Mexican slang: adapt to an equivalent that works.
* If an entry is clearly an internal code word (a single lowercase English token
  like `hookup`, `charm`, `kind`), copy it unchanged.
* Use the glossary for recurring terms (stats, menus, apps, routes) so the whole
  game is consistent.
