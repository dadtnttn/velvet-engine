// The page hooks of the engine: whether the page is hidden, and the user gestures that allow audio
// to start. In a browser they come from the document and the window. Without a page (Node, tests)
// there is nothing to watch, so the quiet host below is used.

import type { AudioHost } from "./api";

/** The events that count as a user gesture for the browser's audio rule. */
const GESTURE_EVENTS = ["pointerdown", "keydown", "touchend"] as const;

/** A host with no page: never hidden, and no gesture ever comes. */
export const QUIET_HOST: AudioHost = {
  hidden: false,
  onVisibilityChange: () => () => undefined,
  onGesture: () => () => undefined,
};

/** The host of the page the code runs in, or null when there is no document. */
export function browserHost(): AudioHost | null {
  if (typeof document === "undefined" || typeof window === "undefined") return null;
  return {
    get hidden(): boolean {
      return document.hidden;
    },
    onVisibilityChange(listener: () => void): () => void {
      document.addEventListener("visibilitychange", listener);
      return () => document.removeEventListener("visibilitychange", listener);
    },
    onGesture(listener: () => void): () => void {
      for (const type of GESTURE_EVENTS) window.addEventListener(type, listener, { capture: true });
      return () => {
        for (const type of GESTURE_EVENTS) window.removeEventListener(type, listener, { capture: true });
      };
    },
  };
}
