// The overlay's stylesheet. It is added once per document, under one id. Every rule is scoped to the
// vdt- classes, so it changes nothing else on the page. The root and the panel are fixed to the window,
// so the layout of the game's host does not move. Only the panel takes pointer events; the root and the
// watch list let clicks through to the game.

/** The id of the stylesheet element. Its presence means the rules are already in the page. */
export const STYLE_ID = "velvet-devtools-style";

/** The rules. Colours are custom properties on the root, so a game can restyle the tools if it wants. */
const CSS = `
.vdt-root {
  --vdt-bg: rgb(12 12 18 / 0.94);
  --vdt-line: #3b3b4c;
  --vdt-ink: #e8e6ef;
  --vdt-muted: #8d8aa0;
  --vdt-accent: #7fd1ff;
  --vdt-warn: #ffcf6b;
  --vdt-bad: #ff7b8a;
  position: fixed;
  inset: 0;
  z-index: 2147483000;
  pointer-events: none;
  font: 12px/1.45 ui-monospace, Consolas, "Cascadia Mono", monospace;
  color: var(--vdt-ink);
  text-align: left;
}
.vdt-root [hidden] { display: none !important; }
.vdt-hud {
  position: absolute;
  top: 8px;
  left: 8px;
  display: grid;
  gap: 2px;
  padding: 4px 8px;
  background: rgb(0 0 0 / 0.55);
  border-radius: 4px;
  pointer-events: none;
}
.vdt-hud-row { display: flex; gap: 10px; justify-content: space-between; min-width: 140px; }
.vdt-hud-name { color: var(--vdt-muted); }
.vdt-hud-value { color: var(--vdt-accent); font-variant-numeric: tabular-nums; }
.vdt-panel {
  position: absolute;
  top: 8px;
  right: 8px;
  width: min(460px, calc(100vw - 16px));
  max-height: calc(100vh - 16px);
  display: flex;
  flex-direction: column;
  background: var(--vdt-bg);
  border: 1px solid var(--vdt-line);
  border-radius: 6px;
  box-shadow: 0 8px 28px rgb(0 0 0 / 0.45);
  pointer-events: auto;
  overflow: hidden;
}
.vdt-head { display: flex; align-items: center; gap: 8px; padding: 6px 10px; border-bottom: 1px solid var(--vdt-line); }
.vdt-title { font-weight: 700; color: var(--vdt-accent); }
.vdt-hint { color: var(--vdt-muted); flex: 1; }
.vdt-tabs { display: flex; gap: 2px; padding: 4px 6px 0; border-bottom: 1px solid var(--vdt-line); }
.vdt-tab {
  appearance: none;
  border: 1px solid transparent;
  border-bottom: 0;
  border-radius: 4px 4px 0 0;
  background: transparent;
  color: var(--vdt-muted);
  font: inherit;
  padding: 4px 8px;
  cursor: pointer;
}
.vdt-tab[aria-selected="true"] { color: var(--vdt-ink); background: #22222e; border-color: var(--vdt-line); }
.vdt-pages { overflow: auto; min-height: 0; }
.vdt-page { padding: 8px 10px 10px; display: grid; gap: 8px; }
.vdt-button {
  appearance: none;
  border: 1px solid var(--vdt-line);
  border-radius: 4px;
  background: #22222e;
  color: var(--vdt-ink);
  font: inherit;
  padding: 2px 8px;
  cursor: pointer;
}
.vdt-button:hover { filter: brightness(1.25); }
.vdt-button:disabled { opacity: 0.4; cursor: default; filter: none; }
.vdt-button[aria-pressed="true"] { background: var(--vdt-accent); color: #0b0b10; border-color: var(--vdt-accent); }
.vdt-buttons { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
.vdt-status { color: var(--vdt-muted); min-height: 1.45em; white-space: pre-wrap; }
.vdt-status.is-error { color: var(--vdt-bad); }
.vdt-kv { display: grid; grid-template-columns: max-content 1fr; gap: 2px 14px; margin: 0; }
.vdt-kv dt { color: var(--vdt-muted); }
.vdt-kv dd { margin: 0; font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
.vdt-graph { width: 100%; height: 96px; background: #0a0a0f; border: 1px solid var(--vdt-line); border-radius: 4px; }
.vdt-graph-line { fill: none; stroke: var(--vdt-accent); stroke-width: 1.4; vector-effect: non-scaling-stroke; }
.vdt-graph-ref { stroke: var(--vdt-line); stroke-dasharray: 4 4; vector-effect: non-scaling-stroke; }
.vdt-graph-ref.is-warn { stroke: var(--vdt-warn); }
.vdt-graph-ref.is-bad { stroke: var(--vdt-bad); }
.vdt-note { color: var(--vdt-muted); }
.vdt-input {
  box-sizing: border-box;
  width: 100%;
  background: #0a0a0f;
  color: var(--vdt-ink);
  border: 1px solid var(--vdt-line);
  border-radius: 4px;
  padding: 4px 6px;
  font: inherit;
}
.vdt-list { list-style: none; margin: 0; padding: 0; display: grid; gap: 1px; }
.vdt-event { display: grid; grid-template-columns: 5.5em 1fr; gap: 0 8px; padding: 1px 2px; }
.vdt-event:nth-child(odd) { background: rgb(255 255 255 / 0.03); }
.vdt-event-time { color: var(--vdt-muted); font-variant-numeric: tabular-nums; }
.vdt-event-name { color: var(--vdt-accent); }
.vdt-event-summary { color: var(--vdt-ink); overflow-wrap: anywhere; }
.vdt-view { border: 1px solid var(--vdt-line); border-radius: 4px; padding: 6px 8px; display: grid; gap: 4px; }
.vdt-view-head { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.vdt-view-name { font-weight: 700; flex: 1; overflow-wrap: anywhere; }
.vdt-tree { list-style: none; margin: 0; padding: 0; overflow-x: auto; }
.vdt-row { display: flex; align-items: baseline; gap: 4px; white-space: nowrap; min-height: 1.45em; }
.vdt-caret {
  appearance: none;
  border: 0;
  background: transparent;
  color: var(--vdt-muted);
  font: inherit;
  width: 1.2em;
  padding: 0;
  cursor: pointer;
  flex: none;
}
.vdt-caret-gap { display: inline-block; width: 1.2em; flex: none; }
.vdt-label { color: var(--vdt-accent); }
.vdt-label::after { content: ": "; color: var(--vdt-muted); }
.vdt-label:empty::after { content: ""; }
.vdt-row.vdt-kind-circular .vdt-preview { color: var(--vdt-warn); }
.vdt-row.vdt-kind-getter .vdt-preview { color: var(--vdt-muted); font-style: italic; }
.vdt-row.vdt-kind-array .vdt-preview,
.vdt-row.vdt-kind-object .vdt-preview,
.vdt-row.vdt-kind-map .vdt-preview,
.vdt-row.vdt-kind-set .vdt-preview { color: var(--vdt-ink); }
.vdt-more { font-size: inherit; }
.vdt-log {
  margin: 0;
  max-height: 260px;
  overflow: auto;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  background: #0a0a0f;
  border: 1px solid var(--vdt-line);
  border-radius: 4px;
  padding: 6px 8px;
}
.vdt-line-input { color: var(--vdt-accent); }
.vdt-line-output { color: var(--vdt-ink); }
.vdt-line-error { color: var(--vdt-bad); }
.vdt-prompt { display: grid; gap: 4px; }
.vdt-prompt-hint { color: var(--vdt-muted); }
`;

/** Adds the overlay's rules to the document, once. Later calls see the element and do nothing. */
export function installStyles(doc: Document): void {
  if (doc.getElementById(STYLE_ID) !== null) return;
  const style = doc.createElement("style");
  style.id = STYLE_ID;
  style.textContent = CSS;
  doc.head.append(style);
}
