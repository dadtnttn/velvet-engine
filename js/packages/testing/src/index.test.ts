import { describe, expect, it } from "vitest";
import * as testing from "./index";

describe("the package index", () => {
  it("exports the tools that the README names", () => {
    const names = [
      "TestingError",
      "hashState",
      "hashText",
      "stableStringify",
      "createRandom",
      "summarize",
      "percentile",
      "createStageMap",
      "logicalToClient",
      "clientToLogical",
      "placementOf",
      "FakeGamepads",
      "readNavigatorPads",
      "createDriver",
      "Driver",
      "InputScript",
      "createRecorder",
      "parseReplay",
      "parseReplayText",
      "serializeReplay",
      "REPLAY_FORMAT",
      "REPLAY_VERSION",
      "DEFAULT_CHECK_EVERY",
      "startReplay",
      "checkReplay",
      "saveEnvelopeOf",
      "serializeDom",
      "compareSnapshot",
      "VOLATILE_ATTRIBUTES",
      "runSoak",
    ];
    for (const name of names) expect(testing, name).toHaveProperty(name);
  });

  it("names the replay format as velvet-replay, version 1", () => {
    expect(testing.REPLAY_FORMAT).toBe("velvet-replay");
    expect(testing.REPLAY_VERSION).toBe(1);
  });
});
