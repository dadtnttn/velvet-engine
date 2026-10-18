// Tests for the bridge helpers: the slot-name rule, the shape check, and the lookup of
// window.velvet.

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  BridgeUnavailableError,
  SLOT_PATTERN,
  captureScreenshot,
  getBridge,
  isValidSlot,
  isVelvetBridge,
  openExternalLink,
  type VelvetBridge,
} from "./bridge";
import { createFakeBridge, errorOf } from "./test-support";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("SLOT_PATTERN and isValidSlot", () => {
  it("accepts letters, digits, underscores and dashes, up to 64 characters", () => {
    for (const slot of ["slot1", "Auto_save-2", "a", "a".repeat(64)]) {
      expect(SLOT_PATTERN.test(slot)).toBe(true);
      expect(isValidSlot(slot)).toBe(true);
    }
  });

  it("rejects empty names, separators, dots, spaces, accents and names over 64 characters", () => {
    for (const slot of ["", "a/b", "..", "a.json", "a b", "niño", "a".repeat(65)]) {
      expect(SLOT_PATTERN.test(slot)).toBe(false);
      expect(isValidSlot(slot)).toBe(false);
    }
  });
});

describe("isVelvetBridge", () => {
  it("accepts an object with the whole contract", () => {
    const { bridge } = createFakeBridge();
    expect(isVelvetBridge(bridge)).toBe(true);
  });

  it("rejects values that are not objects", () => {
    for (const value of [null, undefined, "desktop", 3, []]) {
      expect(isVelvetBridge(value)).toBe(false);
    }
  });

  it("rejects another platform", () => {
    const { bridge } = createFakeBridge();
    const candidate: unknown = { ...bridge, platform: "browser" };
    expect(isVelvetBridge(candidate)).toBe(false);
  });

  it("rejects a bridge that lacks a save method, a game field or the log function", () => {
    const { bridge } = createFakeBridge();
    const withoutRemove: unknown = {
      ...bridge,
      saves: { list: bridge.saves.list, read: bridge.saves.read, write: bridge.saves.write },
    };
    const withoutTitle: unknown = { ...bridge, game: { id: "demo", version: "0.1.0" } };
    const withoutLog: unknown = { ...bridge, log: "not a function" };
    expect(isVelvetBridge(withoutRemove)).toBe(false);
    expect(isVelvetBridge(withoutTitle)).toBe(false);
    expect(isVelvetBridge(withoutLog)).toBe(false);
  });
});

describe("getBridge", () => {
  it("returns null when window.velvet is missing", () => {
    vi.stubGlobal("velvet", undefined);
    expect(getBridge()).toBeNull();
  });

  it("returns the injected bridge", () => {
    const { bridge } = createFakeBridge();
    vi.stubGlobal("velvet", bridge);
    expect(getBridge()).toBe(bridge);
  });

  it("ignores a global that only partly looks like the bridge", () => {
    vi.stubGlobal("velvet", { platform: "desktop" });
    expect(getBridge()).toBeNull();
  });
});

/**
 * A bridge with the newer calls: a shell that records the links it is given, and a picture of
 * the page. The fake host keeps the rest of the contract.
 */
function bridgeWithNewCalls(): { bridge: VelvetBridge; opened: string[] } {
  const { bridge } = createFakeBridge();
  const opened: string[] = [];
  const extended: VelvetBridge = {
    ...bridge,
    shell: {
      openExternal: async (url) => {
        opened.push(url);
      },
    },
    window: {
      ...bridge.window,
      capture: async () => "data:image/png;base64,iVBORw0KGgo=",
    },
  };
  return { bridge: extended, opened };
}

describe("the newer bridge calls are optional", () => {
  it("a host from before them still counts as a bridge", () => {
    const { bridge } = createFakeBridge();
    expect(bridge.shell).toBeUndefined();
    expect(bridge.window.capture).toBeUndefined();
    expect(isVelvetBridge(bridge)).toBe(true);
  });

  it("a host with them counts too", () => {
    const { bridge } = bridgeWithNewCalls();
    expect(isVelvetBridge(bridge)).toBe(true);
  });
});

describe("openExternalLink", () => {
  it("rejects with no-desktop outside Velvet Desktop", async () => {
    vi.stubGlobal("velvet", undefined);
    const error = await errorOf(() => openExternalLink("https://example.com"));
    expect(error).toBeInstanceOf(BridgeUnavailableError);
    expect(error).toMatchObject({ code: "no-desktop" });
    expect(String((error as Error).message)).toContain("browser");
  });

  it("rejects with unsupported when the host predates the shell", async () => {
    const { bridge } = createFakeBridge();
    vi.stubGlobal("velvet", bridge);
    const error = await errorOf(() => openExternalLink("https://example.com"));
    expect(error).toBeInstanceOf(BridgeUnavailableError);
    expect(error).toMatchObject({ code: "unsupported" });
  });

  it("hands the link to the host", async () => {
    const { bridge, opened } = bridgeWithNewCalls();
    vi.stubGlobal("velvet", bridge);
    await openExternalLink("mailto:someone@example.com");
    expect(opened).toEqual(["mailto:someone@example.com"]);
  });

  it("passes on the host's own refusal, with its code", async () => {
    const { bridge } = bridgeWithNewCalls();
    const refusal = Object.assign(new Error("the link is refused"), { code: "invalid_url" });
    vi.stubGlobal("velvet", {
      ...bridge,
      shell: {
        openExternal: async () => {
          throw refusal;
        },
      },
    });
    const error = await errorOf(() => openExternalLink("javascript:alert(1)"));
    expect(error).toBe(refusal);
  });
});

describe("captureScreenshot", () => {
  it("rejects with no-desktop outside Velvet Desktop", async () => {
    vi.stubGlobal("velvet", undefined);
    const error = await errorOf(() => captureScreenshot());
    expect(error).toMatchObject({ code: "no-desktop" });
  });

  it("rejects with unsupported when the host predates the picture", async () => {
    const { bridge } = createFakeBridge();
    vi.stubGlobal("velvet", bridge);
    const error = await errorOf(() => captureScreenshot());
    expect(error).toMatchObject({ code: "unsupported" });
  });

  it("returns the PNG data URL from the host", async () => {
    const { bridge } = bridgeWithNewCalls();
    vi.stubGlobal("velvet", bridge);
    await expect(captureScreenshot()).resolves.toBe("data:image/png;base64,iVBORw0KGgo=");
  });
});
