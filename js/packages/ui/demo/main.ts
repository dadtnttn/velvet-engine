// Demo of @velvet/ui. It shows the 16:9 stage in the window (with bars when the window is not
// 16:9), a backdrop canvas drawn at the screen's pixels, and one of each HUD component.
//
// demo/index.html loads this file as a module. Vite is not added yet: open the page with Vite
// later, or with any static server that can serve .ts modules.

import {
  createButton,
  createChip,
  createKeycap,
  createMenu,
  createMeter,
  createModal,
  createPlate,
  createStage,
  createToast,
  createTooltip,
  type Stage,
} from "../src/index";

const host = document.getElementById("game");
if (host === null) throw new Error('demo/index.html needs an element with id "game"');

const stage = createStage(host, { width: 1920, height: 1080 });

// The backdrop: a canvas whose backing store is the stage's pixel size. Drawing with k maps the
// logical units onto those pixels, so the text is sharp on any window.
const backdrop = document.createElement("canvas");
backdrop.style.cssText = "position:absolute;left:0;top:0;width:1920px;height:1080px";
stage.root.append(backdrop);

/** Paints the backdrop at the stage's current pixel size. */
function paintBackdrop(target: Stage, canvas: HTMLCanvasElement): void {
  canvas.width = target.pixelWidth;
  canvas.height = target.pixelHeight;
  const context = canvas.getContext("2d");
  if (context === null) return;
  context.setTransform(target.k, 0, 0, target.k, 0, 0);
  const gradient = context.createLinearGradient(0, 0, target.width, target.height);
  gradient.addColorStop(0, "#2a0f22");
  gradient.addColorStop(1, "#0e0816");
  context.fillStyle = gradient;
  context.fillRect(0, 0, target.width, target.height);
  context.fillStyle = "#f6efe4";
  context.font = "28px sans-serif";
  context.fillText(
    `Lienzo ${target.pixelWidth} x ${target.pixelHeight} pixeles · k ${target.k.toFixed(3)} · escala ${target.scale.toFixed(3)}`,
    60,
    1040,
  );
}
paintBackdrop(stage, backdrop);
stage.onResize(() => {
  paintBackdrop(stage, backdrop);
});

/** A row of elements that wraps when it is full. */
function row(...nodes: HTMLElement[]): HTMLElement {
  const line = document.createElement("div");
  line.style.cssText = "display:flex;flex-wrap:wrap;gap:14px;align-items:center";
  line.append(...nodes);
  return line;
}

const status = document.createElement("p");
status.style.cssText = "margin:0;font-size:22px";
/** Shows a line of text under the buttons. */
function setStatus(text: string): void {
  status.textContent = text;
}

// The main panel: keys, chips, meters, buttons and the status line.
const panel = createPlate({ title: "Persona HUD" });
panel.el.style.cssText = "position:absolute;left:120px;top:120px;width:700px";
stage.root.append(panel.el);
const column = document.createElement("div");
column.style.cssText = "display:flex;flex-direction:column;gap:16px";
panel.body.append(column);

column.append(
  row(
    createKeycap({ key: "KeyE" }).el,
    createKeycap({ key: "Space" }).el,
    createKeycap({ key: "MouseLeft" }).el,
    createKeycap({ key: "MouseWheel" }).el,
  ),
);

column.append(
  row(createChip({ label: "Pociones", count: 3 }).el, createChip({ label: "Llaves", count: null }).el),
);

let health = 70;
const healthBar = createMeter({ label: "Salud", value: health, max: 100, segments: 10 });
const charge = createMeter({ label: "Carga", value: 40, tone: "gold" });
column.append(healthBar.el, charge.el);

// Buttons. The state of the toggle is owned here: the button only shows it.
let paused = false;
const pauseButton = createButton({
  label: "Pausa",
  pressed: paused,
  onPress: () => {
    paused = !paused;
    pauseButton.update({ pressed: paused });
    setStatus(paused ? "En pausa" : "Sigue el juego");
  },
});

const saveButton = createButton({
  label: "Guardar",
  onPress: () => {
    setStatus("Partida guardada");
    toast.show("Partida guardada", "info");
  },
});

const useButton = createButton({
  label: "Usar",
  hotkey: "E",
  onPress: () => {
    health = health >= 100 ? 10 : Math.min(100, health + 10);
    healthBar.update({ value: health });
    setStatus(`Salud ${health}`);
  },
});

const disabledButton = createButton({ label: "Cargar", disabled: true });

const nativeButton = createButton({
  label: "Resolución nativa: sí",
  onPress: () => {
    stage.setNative(!stage.native);
    nativeButton.update({ label: `Resolución nativa: ${stage.native ? "sí" : "no"}` });
  },
});

column.append(row(pauseButton.el, saveButton.el, useButton.el, disabledButton.el), row(nativeButton.el));
column.append(status);

// Tooltip: it follows the save button while the pointer is over it.
const tip = createTooltip({ stage });
stage.root.append(tip.el);
saveButton.el.addEventListener("pointerenter", () => {
  tip.update({ anchor: saveButton.el, text: "Guarda la partida en la ranura 1" });
});
saveButton.el.addEventListener("pointerleave", () => {
  tip.update({ anchor: null });
});

// Toasts: queued notices in the bottom-right corner.
const toast = createToast({ maxVisible: 3, duration: 2500 });
stage.root.append(toast.el);

// The menu, with the keyboard arrows and the mouse.
const menuPlate = createPlate({ title: "Menú" });
menuPlate.el.style.cssText = "position:absolute;left:1180px;top:120px;width:560px";
stage.root.append(menuPlate.el);
menuPlate.body.append(
  createMenu({
    label: "Menú principal",
    items: [
      { id: "continue", label: "Continuar" },
      { id: "options", label: "Opciones" },
      { id: "gallery", label: "Galería", disabled: true },
      { id: "quit", label: "Salir" },
    ],
    onSelect: (id) => {
      setStatus(`Menú: ${id}`);
    },
  }).el,
);

// The modal: it asks before leaving, and its promise gives the choice.
const modal = createModal({
  title: "¿Salir al menú?",
  body: "La partida se guardará antes de salir.",
  actions: [
    { id: "yes", label: "Sí, salir" },
    { id: "no", label: "Seguir jugando" },
  ],
});
stage.root.append(modal.el);
column.append(
  createButton({
    label: "Salir…",
    onPress: () => {
      void modal.open().then((choice) => {
        setStatus(`Elegiste: ${choice ?? "nada (Escape)"}`);
      });
    },
  }).el,
);
