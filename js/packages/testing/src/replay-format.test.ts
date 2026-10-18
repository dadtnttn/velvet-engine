import { describe, expect, it } from "vitest";
import { TestingError } from "./errors";
import { REPLAY_FORMAT, REPLAY_VERSION, parseReplay, parseReplayText, serializeReplay } from "./replay-format";

/** A small valid document, which each rejection test changes in one place. */
function validDocument(): Record<string, unknown> {
  return {
    format: REPLAY_FORMAT,
    version: REPLAY_VERSION,
    game: "test-game",
    rate: 60,
    frames: 120,
    seeds: { coins: 7, board: "18446744073709551615" },
    save: null,
    checkEvery: 60,
    checkpoints: [
      { frame: 60, hash: "aaaaaaaaaaaaaaaa" },
      { frame: 120, hash: "bbbbbbbbbbbbbbbb" },
    ],
    inputs: [
      { frame: 0, type: "pad", pad: 0, connected: true, id: "Pad", buttons: [{ pressed: false, value: 0 }], axes: [0, 0] },
      { frame: 3, type: "key", code: "Space", down: true },
      { frame: 3, type: "button", button: "left", down: true, x: 10, y: 20, skipped: true },
      { frame: 4, type: "pointer", x: 12.5, y: 20 },
      { frame: 4, type: "wheel", deltaX: 0, deltaY: -120, deltaMode: 0 },
      { frame: 90, type: "pad", pad: 0, connected: false },
      { frame: 120, type: "blur" },
    ],
  };
}

/** Parses a document and returns the failure, or fails the test when it is accepted. */
function failureOf(document: unknown): TestingError {
  try {
    parseReplay(document);
  } catch (error) {
    if (error instanceof TestingError) return error;
    throw error;
  }
  throw new Error("expected the document to be refused");
}

describe("parseReplay", () => {
  it("accepts a valid document, and keeps its fields", () => {
    const replay = parseReplay(validDocument());
    expect(replay.game).toBe("test-game");
    expect(replay.seeds).toEqual({ coins: 7, board: "18446744073709551615" });
    expect(replay.checkpoints).toHaveLength(2);
    expect(replay.inputs).toHaveLength(7);
    expect(replay.inputs[2]).toEqual({
      frame: 3,
      type: "button",
      button: "left",
      down: true,
      x: 10,
      y: 20,
      skipped: true,
    });
  });

  it("round-trips through serializeReplay with the same content", () => {
    const replay = parseReplay(validDocument());
    expect(parseReplayText(serializeReplay(replay))).toEqual(replay);
  });

  it("writes the seeds in sorted order and ends with a line break", () => {
    const text = serializeReplay(parseReplay(validDocument()));
    expect(text.indexOf('"board"')).toBeLessThan(text.indexOf('"coins"'));
    expect(text.endsWith("}\n")).toBe(true);
  });

  it("refuses text that is not JSON", () => {
    expect(() => parseReplayText("{ not json")).toThrow(/not valid JSON/);
  });

  it("refuses another format and another version, and says which", () => {
    expect(failureOf({ ...validDocument(), format: "velvet-save" }).message).toMatch(/^format must be/);
    expect(failureOf({ ...validDocument(), version: 2 }).message).toMatch(/^version must be 1/);
  });

  it("refuses a rate that is not positive, and a frame count that is not a whole number", () => {
    expect(failureOf({ ...validDocument(), rate: 0 }).message).toMatch(/^rate must be above 0/);
    expect(failureOf({ ...validDocument(), frames: 1.5 }).message).toMatch(/^frames must be a whole number/);
  });

  it("refuses a seed that is neither a safe number nor decimal digits", () => {
    expect(failureOf({ ...validDocument(), seeds: { coins: -1 } }).message).toMatch(/^seeds\.coins/);
    expect(failureOf({ ...validDocument(), seeds: { coins: "12a" } }).message).toMatch(/^seeds\.coins/);
  });

  it("refuses a save that is not an envelope of the save format", () => {
    const save = { format: "velvet-save", version: 1, game: "test-game", savedAt: "x" };
    expect(failureOf({ ...validDocument(), save }).message).toMatch(/^save\.data is required/);
    expect(failureOf({ ...validDocument(), save: { ...save, format: "other", data: 1 } }).message).toMatch(
      /^save\.format must be/,
    );
  });

  it("refuses checkpoints that are off the interval, out of order, or past the end", () => {
    const off = { ...validDocument(), checkpoints: [{ frame: 50, hash: "a" }] };
    expect(failureOf(off).message).toMatch(/^checkpoints\[0\]\.frame must be a multiple of checkEvery/);
    const unordered = {
      ...validDocument(),
      checkpoints: [
        { frame: 120, hash: "a" },
        { frame: 60, hash: "b" },
      ],
    };
    expect(failureOf(unordered).message).toMatch(/^checkpoints\[1\]\.frame must be above the previous/);
    const late = { ...validDocument(), checkpoints: [{ frame: 180, hash: "a" }] };
    expect(failureOf(late).message).toMatch(/at most 120/);
  });

  it("refuses inputs that go back in frame order, or fall after the last frame", () => {
    const inputs = [
      { frame: 5, type: "key", code: "KeyA", down: true },
      { frame: 4, type: "key", code: "KeyB", down: true },
    ];
    expect(failureOf({ ...validDocument(), inputs }).message).toMatch(/^inputs\[1\]\.frame must not be below 5/);
    const after = [{ frame: 121, type: "blur" }];
    expect(failureOf({ ...validDocument(), inputs: after }).message).toMatch(/^inputs\[0\]\.frame must be at most 120/);
  });

  it("refuses an input of a type it does not know, and names the field", () => {
    const inputs = [{ frame: 1, type: "gamepad-rumble" }];
    expect(failureOf({ ...validDocument(), inputs }).message).toMatch(/^inputs\[0\]\.type must be key, button/);
  });

  it("refuses a key code that is not letters or digits, and a skipped flag that is false", () => {
    const badKey = [{ frame: 1, type: "key", code: "Space Bar", down: true }];
    expect(failureOf({ ...validDocument(), inputs: badKey }).message).toMatch(/^inputs\[0\]\.code/);
    const falseFlag = [{ frame: 1, type: "key", code: "KeyA", down: true, skipped: false }];
    expect(failureOf({ ...validDocument(), inputs: falseFlag }).message).toMatch(/^inputs\[0\]\.skipped must be true/);
  });

  it("refuses a pad whose button value is out of range, and a pad index above the range", () => {
    const badValue = [
      { frame: 1, type: "pad", pad: 0, connected: true, id: "Pad", buttons: [{ pressed: true, value: 2 }], axes: [] },
    ];
    expect(failureOf({ ...validDocument(), inputs: badValue }).message).toMatch(/^inputs\[0\]\.buttons\[0\]\.value/);
    const badPad = [{ frame: 1, type: "pad", pad: 16, connected: false }];
    expect(failureOf({ ...validDocument(), inputs: badPad }).message).toMatch(/^inputs\[0\]\.pad must be at most 15/);
  });

  it("refuses a wheel delta mode that is not 0, 1 or 2", () => {
    const inputs = [{ frame: 1, type: "wheel", deltaX: 0, deltaY: 1, deltaMode: 3 }];
    expect(failureOf({ ...validDocument(), inputs }).message).toMatch(/^inputs\[0\]\.deltaMode must be at most 2/);
  });

  it("refuses a mouse button name that is not one of the three", () => {
    const inputs = [{ frame: 1, type: "button", button: "back", down: true, x: 0, y: 0 }];
    expect(failureOf({ ...validDocument(), inputs }).message).toMatch(/^inputs\[0\]\.button must be left, middle or right/);
  });

  it("gives every refusal the bad-replay code", () => {
    expect(failureOf({ ...validDocument(), version: 9 }).code).toBe("bad-replay");
  });
});
