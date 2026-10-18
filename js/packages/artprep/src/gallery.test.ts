// Checks the selection gallery beyond the fixture: escaping of every text and attribute, the refusal of
// unsafe image sources, the empty shot, and the copy button that carries each code.

import { describe, expect, it } from "vitest";
import { galleryHtml, type ShotResult } from "./index";
import { ArtPrepError } from "./error";
import { isSafeImage } from "./results";

/** One shot with the candidates given. */
function shotWith(candidates: ShotResult["candidates"], prompt: string | null = null): ShotResult {
  return { shot: "tess-happy", art: "character.tess.portrait", variant: "happy", prompt, candidates };
}

describe("galleryHtml", () => {
  it("escapes the markup of the prompt, the names and the sources", () => {
    const html = galleryHtml([
      shotWith(
        [{ seed: 101, name: 'character.tess.portrait.happy_101"<b>', image: "a&b<c>.png" }],
        `<script>alert('x')</script> & "quoted"`,
      ),
    ]);
    expect(html).toContain("&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt; &amp; &quot;quoted&quot;");
    expect(html).toContain('data-code="character.tess.portrait.happy_101&quot;&lt;b&gt;"');
    expect(html).toContain('src="a&amp;b&lt;c&gt;.png"');
    expect(html).not.toContain("<script>alert");
    expect(html).not.toContain("<b>");
  });

  it("gives each candidate a copy button that carries its code", () => {
    const html = galleryHtml([
      shotWith([
        { seed: 101, name: "character.tess.portrait.happy_101", image: "101.png" },
        { seed: 102, name: "character.tess.portrait.happy_102", image: "102.png" },
      ]),
    ]);
    expect(html).toContain('<button type="button" data-code="character.tess.portrait.happy_101">Copy code</button>');
    expect(html).toContain('<button type="button" data-code="character.tess.portrait.happy_102">Copy code</button>');
  });

  it("says that a shot has no candidates yet, with no grid", () => {
    const html = galleryHtml([shotWith([])]);
    expect(html).toContain('<p class="empty">No candidates yet.</p>');
    expect(html).not.toContain('class="grid"');
  });

  it("uses the default title when none is given, and the given one when it is", () => {
    expect(galleryHtml([])).toContain("<title>Art gallery</title>");
    expect(galleryHtml([], { title: "Round 2 & 3" })).toContain("<title>Round 2 &amp; 3</title>");
  });

  it("accepts a relative path, an https address and a data:image URL", () => {
    expect(() =>
      galleryHtml([
        shotWith([
          { seed: 101, name: "a_101", image: "shot/a_101.png" },
          { seed: 102, name: "a_102", image: "https://example.com/a_102.png" },
          { seed: 103, name: "a_103", image: "data:image/png;base64,iVBORw0KGgo=" },
        ]),
      ]),
    ).not.toThrow();
  });

  it("refuses an image source that could run code or leave the folder", () => {
    expect(() => galleryHtml([shotWith([{ seed: 101, name: "a_101", image: "javascript:alert(1)" }])])).toThrow(
      ArtPrepError,
    );
    expect(() => galleryHtml([shotWith([{ seed: 101, name: "a_101", image: "/etc/x.png" }])])).toThrow(ArtPrepError);
  });
});

describe("isSafeImage", () => {
  it("accepts relative paths with slashes and refuses the other forms", () => {
    expect(isSafeImage("shot/a.png")).toBe(true);
    expect(isSafeImage("https://example.com/a.png")).toBe(true);
    expect(isSafeImage("http://localhost:8188/a.png")).toBe(true);
    expect(isSafeImage("data:image/webp;base64,AAAA")).toBe(true);
    expect(isSafeImage("")).toBe(false);
    expect(isSafeImage("data:text/html;base64,AAAA")).toBe(false);
    expect(isSafeImage("file:///c/a.png")).toBe(false);
    expect(isSafeImage("C:/a.png")).toBe(false);
    expect(isSafeImage("shot\\a.png")).toBe(false);
  });
});
