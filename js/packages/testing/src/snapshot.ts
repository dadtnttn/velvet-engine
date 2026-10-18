// Snapshots: a stable text for a DOM subtree, and a comparison of that text with a stored one. A snapshot
// asserts what the screen shows. The attributes are sorted, the volatile ones (generated ids, and the
// references to them) are left out, and the text is normalised, so the same screen gives the same text.
// Where the stored text lives is up to the test. In Vitest, `expect(text).toMatchFileSnapshot(path)` does
// the storing and the updating, and `compareSnapshot` gives a readable difference for any other check.

/** Attributes that change from run to run without the screen changing: generated ids and the references to them. */
export const VOLATILE_ATTRIBUTES: readonly string[] = [
  "id",
  "for",
  "aria-activedescendant",
  "aria-controls",
  "aria-describedby",
  "aria-labelledby",
  "aria-owns",
];

/** Elements that never have content, so they are written without a closing tag. */
const VOID_ELEMENTS: ReadonlySet<string> = new Set([
  "area",
  "base",
  "br",
  "col",
  "embed",
  "hr",
  "img",
  "input",
  "link",
  "meta",
  "param",
  "source",
  "track",
  "wbr",
]);

/** The DOM node types that the serializer writes. Other nodes, such as comments, are left out. */
const ELEMENT_NODE = 1;
const TEXT_NODE = 3;

/** The most lines of difference that a comparison reports. */
const MAX_DIFF_LINES = 30;

/** Options for {@link serializeDom}. */
export interface DomSnapshotOptions {
  /**
   * The attributes to leave out: a list of names, or a test on each name and value. Default:
   * {@link VOLATILE_ATTRIBUTES}.
   */
  readonly drop?: readonly string[] | ((name: string, value: string) => boolean);
  /** Spaces per level of nesting. Default 2. */
  readonly indent?: number;
}

/** The outcome of comparing a snapshot with a stored one. */
export interface SnapshotComparison {
  /** True when the texts are the same. */
  readonly match: boolean;
  /** Empty on a match. Otherwise the first lines that differ, with `-` for the stored line and `+` for the new one. */
  readonly diff: string;
}

/**
 * Writes a DOM subtree as stable text: one element or text per line, attributes sorted by name, volatile
 * attributes left out, whitespace in text collapsed, and comments dropped. The text ends with a line break.
 *
 * The screen's value of a form field (`input.value`) is a property, not an attribute, so it is not in the
 * text. A test that needs it should read it and compare it separately.
 */
export function serializeDom(root: Element, options: DomSnapshotOptions = {}): string {
  const indent = options.indent ?? 2;
  const dropped = droppedTest(options.drop ?? VOLATILE_ATTRIBUTES);
  const lines: string[] = [];
  writeNode(root, 0, indent, dropped, lines);
  return `${lines.join("\n")}\n`;
}

/**
 * Compares a new snapshot with a stored one. A missing stored text (null or undefined) is a mismatch,
 * with a note that says so, so that a first run does not pass by accident.
 */
export function compareSnapshot(actual: string, expected: string | null | undefined): SnapshotComparison {
  if (expected === null || expected === undefined) {
    return { match: false, diff: "there is no stored snapshot yet: store the new text to create it" };
  }
  if (actual === expected) return { match: true, diff: "" };
  const actualLines = actual.split("\n");
  const expectedLines = expected.split("\n");
  const lines: string[] = [];
  const count = Math.max(actualLines.length, expectedLines.length);
  for (let index = 0; index < count && lines.length < MAX_DIFF_LINES; index += 1) {
    const before = expectedLines[index];
    const after = actualLines[index];
    if (before === after) continue;
    lines.push(`line ${index + 1}`);
    lines.push(`  - ${before ?? "(no line)"}`);
    lines.push(`  + ${after ?? "(no line)"}`);
  }
  return { match: false, diff: lines.join("\n") };
}

/** The test that decides whether an attribute is left out. */
function droppedTest(drop: readonly string[] | ((name: string, value: string) => boolean)): (name: string, value: string) => boolean {
  if (typeof drop === "function") return drop;
  const names = new Set(drop.map((name) => name.toLowerCase()));
  return (name) => names.has(name.toLowerCase());
}

/** Writes one node and, for an element, its children. */
function writeNode(
  node: Node,
  depth: number,
  indent: number,
  dropped: (name: string, value: string) => boolean,
  lines: string[],
): void {
  const pad = " ".repeat(indent * depth);
  if (node.nodeType === TEXT_NODE) {
    const text = (node.textContent ?? "").replace(/\s+/g, " ").trim();
    if (text !== "") lines.push(`${pad}${escapeText(text)}`);
    return;
  }
  if (node.nodeType !== ELEMENT_NODE) return;
  const element = node as Element;
  const tag = element.tagName.toLowerCase();
  const attributes = Array.from(element.attributes)
    .filter((attribute) => !dropped(attribute.name, attribute.value))
    .sort((left, right) => (left.name < right.name ? -1 : left.name > right.name ? 1 : 0));
  const attributeText = attributes.map((attribute) => ` ${attribute.name}="${escapeAttribute(attribute.value)}"`).join("");
  if (VOID_ELEMENTS.has(tag) && element.childNodes.length === 0) {
    lines.push(`${pad}<${tag}${attributeText} />`);
    return;
  }
  const inner: string[] = [];
  for (const child of Array.from(element.childNodes)) writeNode(child, depth + 1, indent, dropped, inner);
  if (inner.length === 0) {
    lines.push(`${pad}<${tag}${attributeText}></${tag}>`);
    return;
  }
  lines.push(`${pad}<${tag}${attributeText}>`, ...inner, `${pad}</${tag}>`);
}

/** Escapes text for the snapshot: the ampersand and the angle brackets. */
function escapeText(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/** Escapes an attribute value: the text escapes, and the double quote. */
function escapeAttribute(value: string): string {
  return escapeText(value).replace(/"/g, "&quot;");
}
