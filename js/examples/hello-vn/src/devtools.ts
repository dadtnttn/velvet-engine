// The developer tools of hello-vn, for development builds only. The game loads this module with a dynamic import
// behind `import.meta.env.DEV`, so a production build does not contain it. The tools are an F3 overlay with a
// watch on the story's variables, a state view of the story, and a console command that jumps to a scene.

import type { EventEmitter, GameClock, GameLoop } from "@velvet/core";
import { createDevtools, type Devtools } from "@velvet/devtools";
import type { Stage } from "@velvet/ui";
import { GLASS_VAR, SCENES } from "./config";
import type { GameEvents } from "./game";
import type { Story } from "./story";

/** What the tools read and drive. The game passes its own parts; the tools hold no state of their own. */
export interface DevtoolsTarget {
  /** The element the overlay is added to. */
  readonly host: HTMLElement;
  /** The game clock: the tools pause, resume and step it. */
  readonly clock: GameClock;
  /** The game loop, which gives the tools the frame times. */
  readonly loop: GameLoop;
  /** The stage, which the performance panel describes. */
  readonly stage: Stage;
  /** The game's events, which the event log shows. */
  readonly events: EventEmitter<GameEvents>;
  /** The story, for the watches and the state view. */
  readonly story: Story;
  /** Starts a scene of the story, for the `escena` command. */
  readonly jump: (scene: string) => void;
}

/**
 * Makes the developer tools of the game, with its watches, its state view and its console command.
 * @param target - the parts the tools read and drive.
 * @returns the tools; `destroy` removes them.
 */
export function mountDevtools(target: DevtoolsTarget): Devtools {
  const tools = createDevtools<GameEvents>({
    host: target.host,
    enabled: true,
    clock: target.clock,
    loop: target.loop,
    events: target.events,
    stage: target.stage,
    // The place and the sea glass chips are at the top left, so the pinned numbers go to the top right.
    hudCorner: "top-right",
  });
  tools.watch("lente", () => Number(target.story.vars.values[GLASS_VAR] ?? 0));
  tools.watch("final", () => target.story.endingNumber);
  tools.watch("cajas", () => target.story.stepCount);
  tools.inspect("historia", () => (target.story.started ? target.story.snapshot() : null));
  tools.command("escena", {
    args: [{ name: "escena", type: "string", choices: [...SCENES] }],
    help: "salta a una escena de la historia, con las variables que hay",
    run: ({ escena }) => {
      const scene = String(escena);
      target.jump(scene);
      return `escena: ${scene}`;
    },
  });
  return tools;
}
