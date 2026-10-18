// @vitest-environment happy-dom
// Tests for the DOM helpers in happy-dom. The style writes of an element are counted, because a
// frame should write the transform and the opacity once, however many animations move them. The
// tests also cover the motion that an element keeps between animations, the reduced-motion jump,
// and placeElement.

import { afterEach, describe, expect, it, vi } from "vitest";
import { animateElement, isReducedMotion, placeElement } from "./dom";
import { Ticker } from "./ticker";

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.replaceChildren();
});

/**
 * An element in the page whose style writes are counted. happy-dom's `style` is a proxy that does
 * not accept defineProperty, so the element gets its own plain style object, which the helpers
 * write through in the same way.
 */
function countedElement(): { el: HTMLElement; writes: { transform: number; opacity: number } } {
  const el = document.createElement("div");
  document.body.append(el);
  const writes = { transform: 0, opacity: 0 };
  const style: Record<string, string> = {};
  for (const name of ["transform", "opacity"] as const) {
    let stored = "";
    Object.defineProperty(style, name, {
      configurable: true,
      enumerable: true,
      get: () => stored,
      set: (value: string) => {
        writes[name] += 1;
        stored = value;
      },
    });
  }
  Object.defineProperty(el, "style", { configurable: true, value: style });
  return { el, writes };
}

/** Makes `matchMedia` answer the reduced-motion query with `on`. */
function reducedMotion(on: boolean): void {
  vi.stubGlobal("matchMedia", (query: string) => ({ matches: on && query.includes("reduce"), media: query }));
}

describe("writes once per frame", () => {
  it("writes the transform and the opacity once a frame, however many animations move them", () => {
    const { el, writes } = countedElement();
    const ticker = new Ticker();
    const options = { duration: 1, ticker, skipIfReduced: false };
    animateElement(el, { x: 100 }, options);
    animateElement(el, { rotate: 90 }, options);
    animateElement(el, { opacity: 0 }, options);
    expect(writes.transform).toBe(0);
    ticker.advance(0.5);
    expect(writes.transform).toBe(1);
    expect(writes.opacity).toBe(1);
    expect(el.style.transform).toBe("translate(50px, 0px) rotate(45deg) scale(1)");
    expect(el.style.opacity).toBe("0.5");
    ticker.advance(0.25);
    expect(writes.transform).toBe(2);
    expect(writes.opacity).toBe(2);
  });

  it("writes nothing in a frame where nothing moved", () => {
    const { el, writes } = countedElement();
    const ticker = new Ticker();
    animateElement(el, { x: 100 }, { duration: 1, ticker, skipIfReduced: false });
    ticker.advance(0.5);
    const before = writes.transform;
    ticker.advance(0);
    expect(writes.transform).toBe(before);
  });

  it("keeps the motion of an element between animations", () => {
    const { el } = countedElement();
    const ticker = new Ticker();
    animateElement(el, { x: 100 }, { duration: 1, ticker, skipIfReduced: false });
    ticker.advance(1);
    animateElement(el, { opacity: 0.2 }, { duration: 1, ticker, skipIfReduced: false });
    ticker.advance(1);
    expect(el.style.transform).toBe("translate(100px, 0px) rotate(0deg) scale(1)");
    expect(el.style.opacity).toBe("0.2");
  });

  it("targets the element, so that killing by target stops its animation", async () => {
    const { el } = countedElement();
    const ticker = new Ticker();
    const animation = animateElement(el, { x: 10 }, { duration: 1, ticker, skipIfReduced: false });
    ticker.killTweensOf(el);
    await expect(animation.finished).resolves.toBe("cancelled");
  });
});

describe("reduced motion", () => {
  it("jumps to the end state at once when the user asks for reduced motion", () => {
    reducedMotion(true);
    const { el } = countedElement();
    animateElement(el, { x: 40, opacity: 0.5 }, { duration: 1, ticker: new Ticker() });
    expect(isReducedMotion()).toBe(true);
    expect(el.style.transform).toBe("translate(40px, 0px) rotate(0deg) scale(1)");
    expect(el.style.opacity).toBe("0.5");
  });

  it("animates anyway when skipIfReduced is false", () => {
    reducedMotion(true);
    const { el } = countedElement();
    const ticker = new Ticker();
    animateElement(el, { x: 40 }, { duration: 1, ticker, skipIfReduced: false });
    ticker.advance(0.5);
    expect(el.style.transform).toBe("translate(20px, 0px) rotate(0deg) scale(1)");
  });

  it("reads the reduced-motion query", () => {
    reducedMotion(false);
    expect(isReducedMotion()).toBe(false);
    reducedMotion(true);
    expect(isReducedMotion()).toBe(true);
  });
});

describe("placeElement", () => {
  it("writes at once, and keeps the parts that it was not given", () => {
    const { el } = countedElement();
    placeElement(el, { x: 10 });
    expect(el.style.transform).toBe("translate(10px, 0px) rotate(0deg) scale(1)");
    placeElement(el, { opacity: 0.4 });
    expect(el.style.transform).toBe("translate(10px, 0px) rotate(0deg) scale(1)");
    expect(el.style.opacity).toBe("0.4");
  });

  it("clamps the opacity to the range from 0 to 1", () => {
    const { el } = countedElement();
    placeElement(el, { opacity: 1.5 });
    expect(el.style.opacity).toBe("1");
    placeElement(el, { opacity: -1 });
    expect(el.style.opacity).toBe("0");
  });

  it("refuses a part that is not a finite number, and an animation with no part", () => {
    const { el } = countedElement();
    expect(() => placeElement(el, { x: Number.NaN })).toThrow(RangeError);
    expect(() => animateElement(el, {}, { duration: 1, ticker: new Ticker() })).toThrow(RangeError);
  });
});
