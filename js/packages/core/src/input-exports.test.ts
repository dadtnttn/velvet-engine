// Tests for the package entry point's input exports: the names that games import for gamepads, rebinding,
// the bindings file and the pad menu keys. The older exports are checked in index.test.ts.

import { describe, expect, it } from "vitest";
import * as core from "./index";

describe("@velvet/core input exports", () => {
  it("exports the gamepad, rebinding, bindings and menu key names", () => {
    expect(typeof core.ActionMap).toBe("function");
    expect(typeof core.describeBinding).toBe("function");
    expect(typeof core.createRebindPort).toBe("function");
    expect(typeof core.startPadMenuKeys).toBe("function");
    expect(typeof core.createBindingsStore).toBe("function");
    expect(typeof core.parseBindings).toBe("function");
    expect(typeof core.serializeBindings).toBe("function");
    expect(typeof core.applyBindings).toBe("function");
    expect(core.BINDINGS_FORMAT).toBe("velvet-input-bindings");
    expect(core.BINDINGS_VERSION).toBe(1);
    expect(core.BINDINGS_KEY).toBe("input.bindings");
    expect(core.PAD_BUTTON_NAMES).toContain("a");
    expect(core.PAD_AXIS_NAMES).toEqual(["leftX", "leftY", "rightX", "rightY"]);
    expect(core.DEFAULT_DEADZONE).toBe(0.2);
    expect(core.DEFAULT_PAD_THRESHOLD).toBe(0.5);
  });
});
