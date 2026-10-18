// The entry point: installs the error log, finds the page's host element, and starts the game. The stylesheets
// load first, so that the game's own rules come after the HUD's defaults.

import "@velvet/ui/velvet-ui.css";
import "./style.css";
import { Game } from "./game";
import { describeError, installErrorLog, logLine } from "./platform";

installErrorLog();
logLine("info", "hello-vn: page started");

const host = document.getElementById("game");
if (host === null) {
  throw new Error("hello-vn: the page has no #game element");
}

Game.start(host).catch((error: unknown) => {
  const reason = describeError(error);
  logLine("error", `hello-vn: could not start: ${reason}`);
  // No catalog is loaded yet when this shows, so the message cannot come from @velvet/i18n.
  const notice = document.createElement("p");
  notice.className = "vn-start-error";
  notice.setAttribute("role", "alert");
  notice.textContent = `Hello VN could not start: ${reason}`;
  host.replaceChildren(notice);
});
