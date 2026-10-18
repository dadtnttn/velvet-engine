// Tests for the package's public surface, and for the page host that the engine uses in a browser.

import { afterEach, describe, expect, it, vi } from "vitest";
import * as audio from "./index";
import { QUIET_HOST, browserHost } from "./host";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the package index", () => {
  it("exports the entry points and the format constants", () => {
    expect(typeof audio.createAudio).toBe("function");
    expect(typeof audio.parseAudioManifest).toBe("function");
    expect(typeof audio.parseAudioManifestLenient).toBe("function");
    expect(typeof audio.AudioError).toBe("function");
    expect(typeof audio.volumeToGain).toBe("function");
    expect(typeof audio.defaultSettingsStore).toBe("function");
    expect(audio.AUDIO_FORMAT).toBe("velvet-audio");
    expect(audio.AUDIO_VERSION).toBe(1);
    expect(audio.BUILT_IN_BUSES).toEqual(["music", "sfx", "voice", "ui"]);
    expect(audio.AUDIO_FORMATS).toEqual(["opus", "ogg", "mp3", "wav"]);
  });

  it("does not export the test doubles", () => {
    expect(Object.keys(audio)).not.toContain("FakeAudioContext");
    expect(Object.keys(audio)).not.toContain("createRig");
  });
});

describe("the quiet host", () => {
  it("is never hidden, and no listener is ever called", () => {
    const listener = vi.fn();
    const release = QUIET_HOST.onGesture(listener);
    QUIET_HOST.onVisibilityChange(listener);
    release();
    expect(QUIET_HOST.hidden).toBe(false);
    expect(listener).not.toHaveBeenCalled();
  });
});

describe("browserHost", () => {
  it("is null without a page", () => {
    expect(browserHost()).toBeNull();
  });

  it("reads the visibility of the document, and adds and removes its listener", () => {
    const listeners = new Map<string, () => void>();
    const document = {
      hidden: true,
      addEventListener: vi.fn((type: string, listener: () => void) => listeners.set(type, listener)),
      removeEventListener: vi.fn(),
    };
    vi.stubGlobal("document", document);
    vi.stubGlobal("window", { addEventListener: vi.fn(), removeEventListener: vi.fn() });

    const host = browserHost();
    expect(host?.hidden).toBe(true);
    const listener = vi.fn();
    const release = host?.onVisibilityChange(listener);
    expect(document.addEventListener).toHaveBeenCalledWith("visibilitychange", listener);
    release?.();
    expect(document.removeEventListener).toHaveBeenCalledWith("visibilitychange", listener);
  });

  it("listens for presses, keys and touches on the window, and stops all of them", () => {
    vi.stubGlobal("document", { hidden: false, addEventListener: vi.fn(), removeEventListener: vi.fn() });
    const window = { addEventListener: vi.fn(), removeEventListener: vi.fn() };
    vi.stubGlobal("window", window);

    const listener = vi.fn();
    const release = browserHost()?.onGesture(listener);
    const types = window.addEventListener.mock.calls.map((call) => call[0]);
    expect(types).toEqual(["pointerdown", "keydown", "touchend"]);
    release?.();
    expect(window.removeEventListener.mock.calls.map((call) => call[0])).toEqual(["pointerdown", "keydown", "touchend"]);
  });
});
