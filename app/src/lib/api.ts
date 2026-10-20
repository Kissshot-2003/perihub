import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ---- Types shared with the Rust backend (same field names as serde) ----

export type Rgb = string; // "#rrggbb"

export type KeyboardLighting =
  | { type: "off" }
  | { type: "static"; colors: Rgb[] }
  | { type: "gradient"; from: Rgb; to: Rgb }
  | { type: "breathing"; colors: Rgb[]; period_ms: number }
  | { type: "color_cycle"; period_ms: number }
  | { type: "wave"; period_ms: number; reverse: boolean }
  | { type: "hardware_rainbow" }
  | { type: "reactive"; base: Rgb; flash: Rgb; fade_ms: number };

export interface KeyboardConfig {
  lighting: KeyboardLighting;
  brightness: number;
}

/** A keyboard lighting saved by name, for app profiles. */
export interface LightingPreset {
  name: string;
  lighting: KeyboardLighting;
}

/** Permission to read the keyboard's key presses, for reactive lighting (Linux). */
export interface KeyAccess {
  /** PeriHub can read key presses now (null: keyboard not connected). */
  readable: boolean | null;
  rule_installed: boolean;
  supported: boolean;
  manual_install: string;
  manual_remove: string;
}

export interface AppRule {
  /** Window class on Linux, executable name on Windows. */
  app: string;
  /** Saved lighting to show, or null to keep the keyboard as it is. */
  lighting: string | null;
  /** Mouse onboard profile to activate, or null to keep it as it is. */
  mouse_profile: number | null;
}

export interface AppProfiles {
  enabled: boolean;
  rules: AppRule[];
}

export type FocusSupport = { kind: "supported"; desktop: string } | { kind: "unsupported"; reason: string };

export interface ProfilesInfo {
  support: FocusSupport;
  focused: string | null;
  /** Apps focused recently, most recent first. */
  recent: string[];
  profiles: AppProfiles;
  /** Names of the saved lightings. */
  presets: string[];
}

export type KeyBinding =
  | { type: "key"; usage: number }
  | { type: "function"; code: number; param: number }
  | { type: "none" }
  | { type: "raw"; bytes: number[] };

export interface Keymap {
  normal: Record<string, KeyBinding>;
  steelseries: Record<string, KeyBinding>;
}

export interface KeyInfo {
  usage: number;
  name: string;
}

export interface KeyCatalog {
  targets: KeyInfo[];
  sources: number[];
}

export type SpecialFunction =
  | "tilt_left"
  | "tilt_right"
  | "dpi_up"
  | "dpi_down"
  | "dpi_cycle"
  | "dpi_default"
  | "dpi_shift"
  | "profile_next"
  | "profile_prev"
  | "profile_cycle"
  | "g_shift";

/** One command of a macro. `key` is 0 when only modifiers are pressed; mouse `buttons` is a bit mask. */
export type MacroStep =
  | { type: "key_down"; modifiers: number; key: number }
  | { type: "key_up"; modifiers: number; key: number }
  | { type: "mouse_down"; buttons: number }
  | { type: "mouse_up"; buttons: number }
  | { type: "consumer_down"; usage: number }
  | { type: "consumer_up"; usage: number }
  /** Wheel notches: positive scrolls up. */
  | { type: "wheel"; amount: number }
  | { type: "delay"; ms: number }
  | { type: "noop" };

/** once: when pressed. sequence: `press` when pressed, `release` when released. while_held: again
 * and again while held. forever: until the profile changes (read only, PeriHub does not write it). */
export type MacroKind = "once" | "sequence" | "while_held" | "forever";

export interface Macro {
  kind: MacroKind;
  press: MacroStep[];
  release: MacroStep[];
}

export type ButtonAction =
  | { type: "mouse"; button: number }
  | { type: "key"; modifiers: number; key: number }
  | { type: "consumer"; usage: number }
  | { type: "special"; function: SpecialFunction }
  | { type: "disabled" }
  /** Macro in the mouse's memory; `content` is null if PeriHub cannot read it. A new or changed
   * macro gets its place in memory when the profile is saved (sector 0 = not placed yet). */
  | { type: "macro"; sector: number; offset: number; content: Macro | null }
  | { type: "raw"; bytes: number[] };

export type LedEffect =
  | { mode: "off" }
  | { mode: "fixed"; color: Rgb }
  | { mode: "breathing"; color: Rgb; period_ms: number; brightness: number }
  | { mode: "cycle"; period_ms: number; brightness: number }
  | { mode: "raw"; bytes: number[] };

export interface ProfileSettings {
  report_rate_ms: number;
  dpi_levels: number[];
  default_dpi_index: number;
  shift_dpi_index: number;
  buttons: ButtonAction[];
  gshift_buttons: ButtonAction[];
  leds: LedEffect[];
}

export interface MouseProfile {
  number: number;
  enabled: boolean;
  settings: ProfileSettings;
}

export interface MouseState {
  active_profile: number;
  active_dpi_index: number;
  current_dpi: number;
  profiles: MouseProfile[];
}

export interface LedZone {
  index: number;
  location: number;
  effects: number[];
}

export interface MouseInfo {
  product_id: number;
  /** Model name from PeriHub's list, e.g. "G502 HERO". */
  model: string;
  /** Name reported by the mouse itself. */
  name: string;
  /** Tested with PeriHub: unverified models are read-only. */
  verified: boolean;
  /** Another program left the mouse under software control. */
  host_mode: boolean;
  firmware: { kind: number; name: string; version: string }[];
  dpi: { min: number; max: number; step: number; list: number[] };
  report_rates_ms: number[];
  led_zones: LedZone[];
  onboard: { profile_count: number; button_count: number; has_gshift: boolean; has_dpi_shift: boolean };
  writable: boolean;
}

export interface KeyboardInfo {
  name: string;
  firmware: string;
}

export interface DeviceStatus {
  keyboard: KeyboardInfo | null;
  mouse: MouseInfo | null;
  keyboard_error: string | null;
  mouse_error: string | null;
  /** Running programs that take control of the devices, such as "Logitech G HUB". */
  conflicts: string[];
}

export interface AppSettings {
  start_hidden: boolean;
  autostart: boolean;
  /** Look for a new version on GitHub at startup. */
  check_updates: boolean;
  /** This build can update itself (Windows). */
  updates_supported: boolean;
}

export interface UpdateInfo {
  version: string;
}

export interface KeyChange {
  layer: number;
  source: number;
  binding: KeyBinding;
}

// ---- Commands ----

export const api = {
  status: () => invoke<DeviceStatus>("get_status"),
  onStatus: (handler: (status: DeviceStatus) => void): Promise<UnlistenFn> =>
    listen<DeviceStatus>("devices-changed", (event) => handler(event.payload)),

  keyboardConfig: () => invoke<KeyboardConfig>("kb_get_config"),
  setLighting: (lighting: KeyboardLighting) => invoke<void>("kb_set_lighting", { lighting }),
  setBrightness: (level: number) => invoke<void>("kb_set_brightness", { level }),
  readBrightness: () => invoke<number>("kb_read_brightness"),
  keymap: () => invoke<Keymap>("kb_get_keymap"),
  applyKeymap: (changes: KeyChange[]) => invoke<Keymap>("kb_apply_keymap", { changes }),
  resetKeymap: () => invoke<Keymap>("kb_reset_keymap"),
  keyCatalog: () => invoke<KeyCatalog>("kb_key_catalog"),
  keyAccess: () => invoke<KeyAccess>("kb_key_access"),
  grantKeyAccess: () => invoke<void>("kb_grant_key_access"),
  revokeKeyAccess: () => invoke<void>("kb_revoke_key_access"),
  presets: () => invoke<LightingPreset[]>("kb_get_presets"),
  savePreset: (name: string, lighting: KeyboardLighting) =>
    invoke<LightingPreset[]>("kb_save_preset", { name, lighting }),
  deletePreset: (name: string) => invoke<LightingPreset[]>("kb_delete_preset", { name }),

  mouseState: () => invoke<MouseState>("mouse_get_state"),
  saveProfile: (number: number, settings: ProfileSettings) =>
    invoke<MouseState>("mouse_save_profile", { number, settings }),
  restoreFactory: (number: number) => invoke<MouseState>("mouse_restore_factory", { number }),
  setProfileEnabled: (number: number, enabled: boolean) =>
    invoke<MouseState>("mouse_set_profile_enabled", { number, enabled }),
  setActiveProfile: (number: number) => invoke<MouseState>("mouse_set_active_profile", { number }),
  previewDpi: (dpi: number) => invoke<void>("mouse_preview_dpi", { dpi }),
  previewLed: (zone: number, effect: LedEffect) => invoke<void>("mouse_preview_led", { zone, effect }),
  revertPreview: () => invoke<void>("mouse_revert_preview"),

  profiles: () => invoke<ProfilesInfo>("profiles_get"),
  setProfiles: (profiles: AppProfiles) => invoke<void>("profiles_set", { profiles }),
  onFocus: (handler: (app: string | null) => void): Promise<UnlistenFn> =>
    listen<string | null>("focus-changed", (event) => handler(event.payload)),

  appSettings: () => invoke<AppSettings>("app_get_settings"),
  setAppSettings: (settings: AppSettings) => invoke<AppSettings>("app_set_settings", { settings }),
  quit: () => invoke<void>("app_quit"),
  checkUpdate: () => invoke<UpdateInfo | null>("update_check"),
  installUpdate: () => invoke<void>("update_install"),
};

/** Runs `fn` only after `ms` milliseconds without new calls (for sliders and color pickers). */
export function debounce<A extends unknown[]>(fn: (...args: A) => void, ms: number) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const call = (...args: A) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
  /** Cancels the pending call (e.g. when the component is destroyed). */
  call.cancel = () => clearTimeout(timer);
  return call;
}

export function errorText(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : JSON.stringify(error);
}
