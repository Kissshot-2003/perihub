import type { KeyBinding, SpecialFunction } from "./api";

/** Etichette dei tasti per il layout italiano (usage HID → testo sul tasto). */
const LABELS: Record<number, string> = {
  0x28: "Invio", 0x29: "Esc", 0x2a: "⌫", 0x2b: "Tab", 0x2c: "Spazio",
  0x2d: "'", 0x2e: "ì", 0x2f: "è", 0x30: "+", 0x31: "\\ (US)", 0x32: "ù", 0x33: "ò", 0x34: "à",
  0x35: "\\", 0x36: ",", 0x37: ".", 0x38: "-", 0x39: "Bloc Maiusc",
  0x46: "Stamp", 0x47: "Bloc Scorr", 0x48: "Pausa", 0x49: "Ins", 0x4a: "Home", 0x4b: "Pag ↑",
  0x4c: "Canc", 0x4d: "Fine", 0x4e: "Pag ↓", 0x4f: "→", 0x50: "←", 0x51: "↓", 0x52: "↑",
  0x53: "Bloc Num", 0x54: "Num /", 0x55: "Num *", 0x56: "Num -", 0x57: "Num +", 0x58: "Num Invio",
  0x63: "Num .", 0x64: "<", 0x65: "Menu", 0x67: "Num =", 0x85: "Num ,",
  0x87: "Intl 1", 0x88: "Intl 2", 0x89: "Intl 3", 0x8a: "Intl 4", 0x8b: "Intl 5", 0x90: "Lang 1", 0x91: "Lang 2",
  0xe0: "Ctrl", 0xe1: "Shift", 0xe2: "Alt", 0xe3: "Win", 0xe4: "Ctrl dx", 0xe5: "Shift dx",
  0xe6: "Alt Gr", 0xe7: "Win dx", 0xf0: "SteelSeries",
};

export function keyLabel(usage: number): string {
  if (LABELS[usage]) return LABELS[usage];
  if (usage >= 0x04 && usage <= 0x1d) return String.fromCharCode(65 + usage - 0x04);
  if (usage >= 0x1e && usage <= 0x26) return String(usage - 0x1d);
  if (usage === 0x27) return "0";
  if (usage >= 0x3a && usage <= 0x45) return `F${usage - 0x39}`;
  if (usage >= 0x68 && usage <= 0x73) return `F${usage - 0x68 + 13}`;
  if (usage >= 0x59 && usage <= 0x61) return `Num ${usage - 0x58}`;
  if (usage === 0x62) return "Num 0";
  return `0x${usage.toString(16).padStart(2, "0")}`;
}

/** Funzioni di fabbrica del livello SteelSeries. Il significato di 1 e 2 non è documentato. */
export function functionLabel(code: number): string {
  switch (code) {
    case 3: return "Luminosità –";
    case 4: return "Luminosità +";
    case 5: return "Blocco tasto Win";
    default: return `Funzione di fabbrica ${code}`;
  }
}

export function bindingLabel(binding: KeyBinding | undefined): string {
  if (!binding) return "";
  switch (binding.type) {
    case "key": return keyLabel(binding.usage);
    case "function": return functionLabel(binding.code);
    case "none": return "";
    case "raw": return "?";
  }
}

export interface KeyCap {
  usage: number;
  x: number;
  y: number;
  w: number;
  h: number;
  /** Tasto presente sulla tastiera ma non rimappabile dal firmware. */
  locked?: boolean;
}

type Row = (number | [number, number] | { gap: number } | { lock: number; w: number })[];

function row(y: number, items: Row, startX = 0): KeyCap[] {
  const caps: KeyCap[] = [];
  let x = startX;
  for (const item of items) {
    if (typeof item === "number") {
      caps.push({ usage: item, x, y, w: 1, h: 1 });
      x += 1;
    } else if (Array.isArray(item)) {
      caps.push({ usage: item[0], x, y, w: item[1], h: 1 });
      x += item[1];
    } else if ("gap" in item) {
      x += item.gap;
    } else {
      caps.push({ usage: item.lock, x, y, w: item.w, h: 1, locked: true });
      x += item.w;
    }
  }
  return caps;
}

/** Layout TKL ISO (italiano) della Apex 3 TKL, in unità di tasto. */
export const TKL_ISO: KeyCap[] = [
  ...row(0, [0x29, { gap: 1 }, 0x3a, 0x3b, 0x3c, 0x3d, { gap: 0.5 }, 0x3e, 0x3f, 0x40, 0x41, { gap: 0.5 },
    0x42, 0x43, 0x44, 0x45, { gap: 0.25 }, 0x46, 0x47, 0x48]),
  ...row(1.25, [0x35, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x2d, 0x2e, [0x2a, 2],
    { gap: 0.25 }, 0x49, 0x4a, 0x4b]),
  ...row(2.25, [[0x2b, 1.5], 0x14, 0x1a, 0x08, 0x15, 0x17, 0x1c, 0x18, 0x0c, 0x12, 0x13, 0x2f, 0x30]),
  { usage: 0x28, x: 13.75, y: 2.25, w: 1.25, h: 2 },
  ...row(2.25, [0x4c, 0x4d, 0x4e], 15.25),
  ...row(3.25, [[0x39, 1.75], 0x04, 0x16, 0x07, 0x09, 0x0a, 0x0b, 0x0d, 0x0e, 0x0f, 0x33, 0x34, 0x32]),
  ...row(4.25, [[0xe1, 1.25], 0x64, 0x1d, 0x1b, 0x06, 0x19, 0x05, 0x11, 0x10, 0x36, 0x37, 0x38, [0xe5, 2.75],
    { gap: 1.25 }, 0x52]),
  ...row(5.25, [[0xe0, 1.25], [0xe3, 1.25], [0xe2, 1.25], [0x2c, 6.25], [0xe6, 1.25], [0xf0, 1.25],
    { lock: 0x65, w: 1.25 }, [0xe4, 1.25], { gap: 0.25 }, 0x50, 0x51, 0x4f]),
];

export const LAYOUT_WIDTH = 18.25;
export const LAYOUT_HEIGHT = 6.25;

// ---- Mouse ----

export const MOUSE_BUTTON_NAMES = [
  "Clic sinistro",
  "Clic destro",
  "Clic rotella",
  "Indietro",
  "Avanti",
  "Sniper (DPI Shift)",
  "DPI –",
  "DPI +",
  "Cambio profilo",
  "Rotella a destra",
  "Rotella a sinistra",
];

export const MOUSE_CLICKS: { button: number; label: string }[] = [
  { button: 1, label: "Clic sinistro" },
  { button: 2, label: "Clic destro" },
  { button: 3, label: "Clic centrale" },
  { button: 4, label: "Indietro" },
  { button: 5, label: "Avanti" },
];

export const SPECIAL_FUNCTIONS: { value: SpecialFunction; label: string }[] = [
  { value: "dpi_up", label: "DPI +" },
  { value: "dpi_down", label: "DPI –" },
  { value: "dpi_cycle", label: "DPI a ciclo" },
  { value: "dpi_default", label: "DPI predefiniti" },
  { value: "dpi_shift", label: "Sniper (DPI Shift)" },
  { value: "profile_next", label: "Profilo successivo" },
  { value: "profile_prev", label: "Profilo precedente" },
  { value: "profile_cycle", label: "Profili a ciclo" },
  { value: "g_shift", label: "G-Shift" },
  { value: "tilt_left", label: "Scorrimento a sinistra" },
  { value: "tilt_right", label: "Scorrimento a destra" },
];

export const CONSUMER_KEYS: { usage: number; label: string }[] = [
  { usage: 0xcd, label: "Play / Pausa" },
  { usage: 0xb5, label: "Brano successivo" },
  { usage: 0xb6, label: "Brano precedente" },
  { usage: 0xb7, label: "Stop" },
  { usage: 0xe2, label: "Muto" },
  { usage: 0xe9, label: "Volume +" },
  { usage: 0xea, label: "Volume –" },
  { usage: 0x192, label: "Calcolatrice" },
  { usage: 0x194, label: "Esplora file" },
  { usage: 0x18a, label: "Posta" },
  { usage: 0x223, label: "Browser: home" },
  { usage: 0x224, label: "Browser: indietro" },
  { usage: 0x225, label: "Browser: avanti" },
  { usage: 0x227, label: "Browser: aggiorna" },
];

export const MODIFIERS: { bit: number; label: string }[] = [
  { bit: 0x01, label: "Ctrl" },
  { bit: 0x02, label: "Shift" },
  { bit: 0x04, label: "Alt" },
  { bit: 0x08, label: "Win" },
  { bit: 0x40, label: "Alt Gr" },
];

/** Nomi delle zone LED secondo la posizione HID++ (1 = principale, 2 = logo). */
export function ledZoneName(location: number): string {
  if (location === 1) return "Indicatore DPI";
  if (location === 2) return "Logo G";
  return `Zona ${location}`;
}

export function reportRateLabel(ms: number): string {
  return `${Math.round(1000 / ms)} Hz`;
}
