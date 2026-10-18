// Moves an audio parameter smoothly. A sudden jump in a gain makes a click, so every change of
// volume, mute or ducking is a short ramp from the value that the parameter has now.

import type { AudioContextLike, AudioParamLike } from "./context";

/**
 * Ramps `param` linearly from its current value to `target`, over `seconds` from now. A time of zero
 * or less jumps to the target at once. Scheduled changes after now are removed first, so a new ramp
 * replaces an old one that has not finished.
 */
export function rampTo(context: AudioContextLike, param: AudioParamLike, target: number, seconds: number): void {
  const now = context.currentTime;
  param.cancelScheduledValues(now);
  if (seconds <= 0) {
    param.setValueAtTime(target, now);
    return;
  }
  param.setValueAtTime(param.value, now);
  param.linearRampToValueAtTime(target, now + seconds);
}
