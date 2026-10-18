// The entry point: installs the logger and the error reports, finds the page's host element, and starts the game.
// The stylesheets load first, so that the game's own rules come after the HUD's defaults.

import "@velvet/ui/velvet-ui.css";
import "./style.css";
import { Game } from "./game";
import { createGameLogger, reportErrors } from "./log";

const logger = createGameLogger();
// The error overlay speaks the language of the game once the game has loaded its texts.
let game: Game | null = null;
reportErrors(logger, document.body, () => game?.translate);
logger.info("page started");

const host = document.getElementById("game");
if (host === null) {
  throw new Error("hello-2d: the page has no #game element");
}

Game.start(host, { logger })
  .then((started) => {
    game = started;
  })
  .catch((error: unknown) => {
    const reason = error instanceof Error ? error.message : String(error);
    logger.error(`could not start: ${reason}`);
    // No catalog is loaded yet when this shows, so the message cannot come from @velvet/i18n.
    const notice = document.createElement("p");
    notice.className = "h2d-start-error";
    notice.setAttribute("role", "alert");
    notice.textContent = `Hello 2D could not start: ${reason}`;
    host.replaceChildren(notice);
  });
