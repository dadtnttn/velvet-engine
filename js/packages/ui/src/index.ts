// @velvet/ui: the 16:9 stage that every Velvet JS game draws in, the board view (a pannable, zoomable
// world inside the stage, for cards and maps), the Persona HUD primitives (plates, buttons, keycaps,
// chips, meters, tooltips, toasts, modals and menus) that sit over it, the form controls (sliders, toggles
// and choices) and the settings panel that binds them to a game's settings.
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
  BoardInsets,
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
export { releaseFocus } from "./components/dom";
export { createPlate } from "./components/plate";
export type { Plate, PlateProps } from "./components/plate";
export { createButton } from "./components/button";
export type { ButtonProps } from "./components/button";
export { createKeycap, keyLabel } from "./components/keycap";
export type { KeycapProps, KeycapTextCase } from "./components/keycap";
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
export { createRebindPanel, type RebindAction, type RebindConflict, type RebindDevice, type RebindDisplay, type RebindPanel, type RebindPanelOptions, type RebindPanelProps, type RebindPolicy, type RebindPort, type RebindRequest, type RebindResult, type RebindSlot, type RebindStatus, type TranslateFn } from "./rebind-panel";
export { createScreenStack } from "./screens";
export type { Screen, ScreenKind, ScreenListener, ScreenStack, ScreenStackOptions } from "./screens";
export { createSaveSlots, formatPlayTime, formatSaveTime, isSafeThumbnail } from "./save-slots";
export type { SaveSlotDraft, SaveSlotMeta, SaveSlotRecord, SaveSlotStore, SaveSlots, SaveSlotsMode, SaveSlotsOptions, SaveSlotsProps } from "./save-slots";
export { createNotices } from "./notices";
export type { NoticeAction, NoticeAnchor, NoticeRecord, NoticeSpec, NoticeTone, Notices, NoticesOptions, NoticesProps } from "./notices";
export { createDialogueBox } from "./dialogue-box";
export type { BacklogEntry, DialogueBox, DialogueBoxOptions, DialogueBoxProps, DialogueChoice, DialogueEnd, DialogueHandOver, DialogueLine, DialogueOption, DialogueSession, DialogueShown } from "./dialogue-box";
export { createDirtyMarks, setActive, setShown, setText } from "./dirty";
export type { DirtyMarks, DirtyOptions } from "./dirty";
export { createBar } from "./bar";
export type { BarProps, BarTone } from "./bar";
export { createSlider } from "./components/slider";
export type { Slider, SliderOptions } from "./components/slider";
export { createToggle } from "./components/toggle";
export type { Toggle, ToggleOptions } from "./components/toggle";
export { createChoice } from "./components/choice";
export type { Choice, ChoiceOption, ChoiceOptions, ChoiceValue } from "./components/choice";
export { createSettingsPanel } from "./settings-panel";
export type {
  ChoiceItem,
  SettingItem,
  SettingsPanel,
  SettingsPanelOptions,
  SettingsPanelStore,
  SettingsSection,
  SliderItem,
  ToggleItem,
} from "./settings-panel";
