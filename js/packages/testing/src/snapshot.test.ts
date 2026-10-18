// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import { compareSnapshot, serializeDom } from "./snapshot";

/** A small screen: a menu with a button that has generated ids, an image, and a comment. */
function screen(): HTMLElement {
  const root = document.createElement("div");
  root.className = "menu";
  root.setAttribute("data-x", "1");
  root.innerHTML = [
    '<button id="btn-1234" aria-controls="panel-9" class="vu-button" style="left: 10px">  Play  </button>',
    '<img src="a.png" alt="cover">',
    "<!-- a comment that is not on screen -->",
  ].join("\n");
  return root;
}

describe("serializeDom", () => {
  it("writes elements and text with sorted attributes, without volatile ids, and with collapsed text", () => {
    const text = serializeDom(screen());
    expect(text).toBe(
      [
        '<div class="menu" data-x="1">',
        '  <button class="vu-button" style="left: 10px">',
        "    Play",
        "  </button>",
        '  <img alt="cover" src="a.png" />',
        "</div>",
        "",
      ].join("\n"),
    );
  });

  it("drops exactly the attributes that a list names, which replaces the default list", () => {
    const text = serializeDom(screen(), { drop: ["class", "style"], indent: 0 });
    // The generated id and the reference are kept, since the list does not name them.
    expect(text).toContain('<button aria-controls="panel-9" id="btn-1234">\nPlay\n</button>');
    expect(text).not.toContain("class=");
    expect(text).not.toContain("style=");
  });

  it("takes a test on the name and the value when a list is not enough", () => {
    const text = serializeDom(screen(), { drop: (name, value) => name === "data-x" && value === "1" });
    expect(text).not.toContain("data-x");
    expect(text).toContain('class="menu"');
  });

  it("escapes text and attribute values", () => {
    const root = document.createElement("p");
    root.setAttribute("title", 'say "hi" & <go>');
    root.textContent = "1 < 2 & 3 > 2";
    expect(serializeDom(root, { indent: 0 })).toBe(
      ['<p title="say &quot;hi&quot; &amp; &lt;go&gt;">', "1 &lt; 2 &amp; 3 &gt; 2", "</p>", ""].join("\n"),
    );
  });

  it("writes an element with no children on one line", () => {
    const box = document.createElement("div");
    box.className = "empty";
    expect(serializeDom(box, { indent: 0 })).toBe('<div class="empty"></div>\n');
  });
});

describe("compareSnapshot", () => {
  it("matches the same text", () => {
    expect(compareSnapshot("a\nb\n", "a\nb\n")).toEqual({ match: true, diff: "" });
  });

  it("names the lines that differ, with the stored and the new text", () => {
    const result = compareSnapshot("a\nB\nc\n", "a\nb\nc\n");
    expect(result.match).toBe(false);
    expect(result.diff).toBe("line 2\n  - b\n  + B");
  });

  it("reports a missing line on either side", () => {
    const result = compareSnapshot("a\nb\n", "a\n");
    expect(result.diff).toContain("(no line)");
  });

  it("refuses to match when there is no stored text, and says what to do", () => {
    const result = compareSnapshot("a\n", null);
    expect(result.match).toBe(false);
    expect(result.diff).toMatch(/no stored snapshot yet/);
  });
});
