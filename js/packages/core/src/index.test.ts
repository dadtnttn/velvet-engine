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

  it("exports the frame hooks' tools, the clock events and the settings store", () => {
    expect(typeof core.createSettingsStore).toBe("function");
    expect(core.SETTINGS_FORMAT).toBe("velvet-settings");
    expect(typeof core.chooseSettingsBackend).toBe("function");
    expect(typeof core.GameLoop.prototype.on).toBe("function");
    expect(typeof core.GameClock.prototype.on).toBe("function");
    expect(typeof core.EventEmitter.prototype.tap).toBe("function");
    expect(typeof core.EventEmitter.prototype.names).toBe("function");
    expect(typeof core.EventEmitter.prototype.listenerCount).toBe("function");
  });

  it("exports the logger, its sinks and the error reports", () => {
    expect(core.LOG_LEVELS).toEqual(["debug", "info", "warn", "error"]);
    expect(typeof core.createLogger).toBe("function");
    expect(typeof core.isLogLevel).toBe("function");
    expect(typeof core.createConsoleSink).toBe("function");
    expect(typeof core.createBridgeSink).toBe("function");
    expect(typeof core.installErrorReporter).toBe("function");
    expect(typeof core.captureError).toBe("function");
    expect(typeof core.formatReport).toBe("function");
  });
});
