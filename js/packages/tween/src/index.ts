// @velvet/tween: motion for any game or tool. Easings, tweens, timelines and springs, run by a
// ticker that follows the core's game clock (so pause and game speed apply), or runs in real time
// for UI. Helpers animate the transform and opacity of DOM elements, once per frame.
//
// Import from the package root, for example:
//   import { tween, timeline, Ticker, quadOut } from "@velvet/tween";

export {
  backIn,
  backInOut,
  backOut,
  bounceIn,
  bounceInOut,
  bounceOut,
  circIn,
  circInOut,
  circOut,
  cubicBezier,
  cubicIn,
  cubicInOut,
  cubicOut,
  ease,
  elasticIn,
  elasticInOut,
  elasticOut,
  expoIn,
  expoInOut,
  expoOut,
  isEasingName,
  linear,
  quadIn,
  quadInOut,
  quadOut,
  quartIn,
  quartInOut,
  quartOut,
  quintIn,
  quintInOut,
  quintOut,
  sineIn,
  sineInOut,
  sineOut,
  steps,
} from "./easing";
export type { EasingFunction, EasingName, StepPosition } from "./easing";
export { Ticker, createRealtimeDriver, sharedTicker } from "./ticker";
export type { Playable, RealtimeDriver, RealtimeDriverOptions, TickerOptions } from "./ticker";
export type { Outcome, PlaybackOptions, TimelineChild } from "./playback";
export { interpolate } from "./interpolate";
export type { TweenValue } from "./interpolate";
export { formatColour, mixColours, parseColour } from "./colour";
export type { Colour } from "./colour";
export { Tween, tween } from "./tween";
export type { TweenOptions } from "./tween";
export { Timeline, timeline } from "./timeline";
export type { TimelineOptions, TimelinePosition, TimelineSpan } from "./timeline";
export { Spring, criticalDamping, spring } from "./spring";
export type { SpringOptions } from "./spring";
export { animateElement, isReducedMotion, placeElement } from "./dom";
export type { DomMotion, ElementAnimateOptions } from "./dom";
