// @velvet/ui: the 16:9 stage that every Velvet JS game draws in, the board view (a pannable, zoomable
// world inside the stage, for cards and maps), and the Persona HUD primitives (plates, buttons, keycaps,
// chips, meters, tooltips, toasts, modals and menus) that sit over it.
//
// Import from the package root, for example:
//   import { createStage, createBoardView, createButton, createMenu } from "@velvet/ui";
// and load the look once, with:
//   import "@velvet/ui/velvet-ui.css";

export { createStage, Stage, computeStageLayout } from "./stage";
export type { LogicalPoint, LogicalRect, StageLayout, StageListener, StageOptions } from "./stage";
export { createBoardView } from "./board";
export type {
  BoardDrag,
  BoardDragCallbacks,
  BoardListener,
  BoardOptions,
  BoardView,
  BoardViewState,
  DragOptions,
  ItemMove,
  ItemSpec,
  WorldPoint,
  WorldRect,
} from "./board";
export type { VuComponent } from "./components/component";
export { createPlate } from "./components/plate";
export type { Plate, PlateProps } from "./components/plate";
export { createButton } from "./components/button";
export type { ButtonProps } from "./components/button";
export { createKeycap, keyLabel } from "./components/keycap";
export type { KeycapProps } from "./components/keycap";
export { createChip } from "./components/chip";
export type { ChipProps } from "./components/chip";
export { createMeter } from "./components/meter";
export type { MeterProps, MeterTone } from "./components/meter";
export { createTooltip } from "./components/tooltip";
export type { TooltipOptions, TooltipProps } from "./components/tooltip";
export { createToast } from "./components/toast";
export type { Toast, ToastProps, ToastTone } from "./components/toast";
export { createModal } from "./components/modal";
export type { Modal, ModalAction, ModalProps } from "./components/modal";
export { createMenu } from "./components/menu";
export type { Menu, MenuItem, MenuProps } from "./components/menu";
