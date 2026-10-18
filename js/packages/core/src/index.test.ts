// Tests for the package entry point: the runtime names it exports.

import { describe, expect, it } from "vitest";
import * as core from "./index";

describe("@velvet/core entry point", () => {
  it("exports the runtime API", () => {
    expect(typeof core.GameLoop).toBe("function");
    expect(typeof core.createFrameScheduler).toBe("function");
    expect(typeof core.GameClock).toBe("function");
    expect(typeof core.EventEmitter).toBe("function");
    expect(typeof core.ActionMap).toBe("function");
    expect(typeof core.markHandled).toBe("function");
    expect(core.SLOT_PATTERN).toBeInstanceOf(RegExp);
    expect(typeof core.getBridge).toBe("function");
    expect(typeof core.isValidSlot).toBe("function");
    expect(typeof core.createSaveStore).toBe("function");
    expect(typeof core.SaveError).toBe("function");
    expect(core.SAVE_FORMAT).toBe("velvet-save");
    expect(typeof core.createMemoryBackend).toBe("function");
    expect(typeof core.createLocalStorageBackend).toBe("function");
    expect(typeof core.createBridgeBackend).toBe("function");
    expect(typeof core.getLocalStorage).toBe("function");
  });
});
