// @velvet/scene2d: a retained 2D scene drawn with Canvas 2D, at the stage's native pixels. Nodes (groups,
// sprites, nine-slices, text and shapes) sit in layers with a parallax factor, a camera moves the world, and
// atlases and animations bring sprite sheets in. Hit tests use the same transforms as the drawing.
// See README.md.
//
// Import from the package root, for example:
//   import { createScene2D, Sprite, createAnimation, followStage } from "@velvet/scene2d";

export { createScene2D, Scene2D } from "./scene";
export type { Scene2DCanvas, Scene2DOptions, SceneStats } from "./scene";
export type { Context2D, DrawPass, MeasuredText, TextMeasure } from "./pass";
export { Node, Group } from "./node";
export type { NodeOptions } from "./node";
export { Layer } from "./layer";
export type { LayerOptions } from "./layer";
export { Sprite, ImageNode } from "./sprite";
export type { ImageNodeOptions, SpriteFrame, SpriteImage, SpriteOptions } from "./sprite";
export { NineSlice } from "./nine-slice";
export type { NineSliceInsets, NineSliceOptions } from "./nine-slice";
export { Text } from "./text";
export type { TextAlign, TextBaseline, TextOptions, TextOutline, TextShadow } from "./text";
export { Shape } from "./shape";
export type { ShapeGeometry, ShapeOptions } from "./shape";
export { Camera2D } from "./camera";
export type { CameraTarget, FollowOptions } from "./camera";
export { Animation, createAnimation } from "./animation";
export type { AnimationOptions, PlayDirection } from "./animation";
export { Atlas, AsepriteSheet, parseAseprite, parseTexturePacker } from "./atlas";
export type { AsepriteTag, AtlasSize } from "./atlas";
export { followStage } from "./stage";
export type { StageSize, StageSource } from "./stage";
export { Matrix2D } from "./math";
export type { Point2D, Rect2D } from "./math";
