// The selection gallery: one static HTML page that shows the candidates of each shot in a grid. Each
// candidate shows its code, which is its output name, and a button that copies the code. The page is
// self-contained: its style and its one script are inline, and it loads nothing else. Every text and
// every attribute is escaped (docs/formats/ART_PREP.md, "Selection gallery").

import { ArtPrepError } from "./error";
import { isSafeImage } from "./results";
import type { GalleryOptions, ShotResult } from "./types";

/** The style of the page. Colours follow the system theme. */
const STYLE = `:root { color-scheme: light dark; --bg: #f7f5f2; --fg: #1d1b19; --muted: #6b645c; --card: #ffffff; --line: #d9d2c8; --accent: #7a3e1d; }
@media (prefers-color-scheme: dark) { :root { --bg: #161412; --fg: #efe9e1; --muted: #a79d91; --card: #211e1b; --line: #3a342e; --accent: #e0a16d; } }
body { margin: 0; padding: 24px; background: var(--bg); color: var(--fg); font: 16px/1.5 system-ui, sans-serif; }
header, main { max-width: 1400px; margin: 0 auto; }
h1 { font-size: 1.6rem; margin: 0 0 4px; }
h2 { font-size: 1.15rem; margin: 32px 0 4px; word-break: break-all; }
.meta { color: var(--muted); margin: 0 0 12px; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 16px; }
figure { margin: 0; background: var(--card); border: 1px solid var(--line); border-radius: 8px; overflow: hidden; }
img { display: block; width: 100%; height: auto; }
figcaption { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; justify-content: space-between; padding: 8px; font-size: 0.85rem; }
code { word-break: break-all; }
button { font: inherit; padding: 4px 10px; border-radius: 6px; border: 1px solid var(--accent); background: transparent; color: var(--accent); cursor: pointer; }
details { margin: 8px 0; color: var(--muted); }
pre { white-space: pre-wrap; word-break: break-word; }
.empty { color: var(--muted); font-style: italic; }
@media (max-width: 600px) { body { padding: 16px; } }`;

/** The one script of the page: the copy button. It uses the clipboard API, with a fallback for older views. */
const SCRIPT = `document.addEventListener("click", function (event) {
  var target = event.target;
  var button = target && target.closest ? target.closest("button[data-code]") : null;
  if (!button) return;
  var code = button.getAttribute("data-code") || "";
  var label = "Copy code";
  var show = function (text) {
    button.textContent = text;
    setTimeout(function () { button.textContent = label; }, 1200);
  };
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(code).then(function () { show("Copied"); }, function () { show("Copy failed"); });
    return;
  }
  var area = document.createElement("textarea");
  area.value = code;
  area.setAttribute("readonly", "");
  area.style.position = "fixed";
  area.style.opacity = "0";
  document.body.appendChild(area);
  area.select();
  var copied = false;
  try { copied = document.execCommand("copy"); } catch (error) { copied = false; }
  area.remove();
  show(copied ? "Copied" : "Copy failed");
});`;

/** Escapes the five characters that matter in HTML text and in quoted attributes. */
function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/** Writes a count with its noun, in the singular for one and the plural otherwise. */
function countOf(count: number, singular: string, plural: string): string {
  return `${count} ${count === 1 ? singular : plural}`;
}

/** The section of one shot: its heading, its prompt, and the grid of its candidates. */
function sectionHtml(shot: ShotResult): string {
  const picture = shot.variant === null ? shot.art : `${shot.art} (variant ${shot.variant})`;
  const prompt =
    shot.prompt === null ? "" : `\n    <details><summary>Prompt</summary><pre>${escapeHtml(shot.prompt)}</pre></details>`;
  const figures = shot.candidates
    .map(
      (candidate) => `<figure>
        <img src="${escapeHtml(candidate.image)}" alt="${escapeHtml(candidate.name)}" loading="lazy" decoding="async">
        <figcaption><code>${escapeHtml(candidate.name)}</code>
        <button type="button" data-code="${escapeHtml(candidate.name)}">Copy code</button></figcaption>
      </figure>`,
    )
    .join("\n      ");
  const grid =
    shot.candidates.length === 0
      ? `<p class="empty">No candidates yet.</p>`
      : `<div class="grid">\n      ${figures}\n    </div>`;
  const count = countOf(shot.candidates.length, "candidate", "candidates");
  return `  <section class="shot" id="shot-${escapeHtml(shot.shot)}">
    <h2>${escapeHtml(shot.shot)}</h2>
    <p class="meta">${escapeHtml(picture)} · ${count}</p>${prompt}
    ${grid}
  </section>`;
}

/**
 * Builds the gallery page for the results of a batch.
 * @param results - the results of each shot, in the order of the page.
 * @param options - the page title. The default is "Art gallery".
 * @returns the HTML of the page, as one string.
 * @throws ArtPrepError when an image source is not safe to show (see isSafeImage).
 */
export function galleryHtml(results: readonly ShotResult[], options: GalleryOptions = {}): string {
  for (const shot of results) {
    for (const candidate of shot.candidates) {
      if (!isSafeImage(candidate.image)) {
        throw new ArtPrepError(
          "value",
          `the image of ${candidate.name} must be a relative path, an https address or a data:image URL`,
        );
      }
    }
  }
  const title = escapeHtml(options.title ?? "Art gallery");
  const candidates = results.reduce((total, shot) => total + shot.candidates.length, 0);
  const counts = `${countOf(results.length, "shot", "shots")}, ${countOf(candidates, "candidate", "candidates")}`;
  const summary = `${counts}. Copy the code of each chosen candidate: it is the file name without the extension.`;
  const sections = results.map(sectionHtml).join("\n");
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${title}</title>
<style>
${STYLE}
</style>
</head>
<body>
<header>
  <h1>${title}</h1>
  <p class="meta">${escapeHtml(summary)}</p>
</header>
<main>
${sections}
</main>
<script>
${SCRIPT}
</script>
</body>
</html>
`;
}
