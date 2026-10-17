import type { KeyboardLighting, Rgb } from "./api";

// Stessa matematica di perihub-core/src/effects.rs, per l'anteprima nell'interfaccia.

export const ZONES = 8;

function parse(hex: Rgb): [number, number, number] {
  const v = parseInt(hex.slice(1), 16);
  return [(v >> 16) & 255, (v >> 8) & 255, v & 255];
}

function css([r, g, b]: [number, number, number]): string {
  return `rgb(${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)})`;
}

function hsv(h: number, s: number, v: number): [number, number, number] {
  const hh = (((h % 1) + 1) % 1) * 6;
  const c = v * s;
  const x = c * (1 - Math.abs((hh % 2) - 1));
  const m = v - c;
  const [r, g, b] =
    hh < 1 ? [c, x, 0] : hh < 2 ? [x, c, 0] : hh < 3 ? [0, c, x] : hh < 4 ? [0, x, c] : hh < 5 ? [x, 0, c] : [c, 0, x];
  return [(r + m) * 255, (g + m) * 255, (b + m) * 255];
}

function phase(elapsedMs: number, periodMs: number): number {
  return (elapsedMs / Math.max(200, periodMs)) % 1;
}

/** Colori CSS delle 8 zone all'istante `elapsedMs`. */
export function previewFrame(lighting: KeyboardLighting, elapsedMs: number): string[] {
  switch (lighting.type) {
    case "off":
      return Array(ZONES).fill("rgb(0, 0, 0)");
    case "static":
      return lighting.colors.map((c) => css(parse(c)));
    case "gradient": {
      const a = parse(lighting.from);
      const b = parse(lighting.to);
      return Array.from({ length: ZONES }, (_, i) => {
        const t = i / (ZONES - 1);
        return css([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]);
      });
    }
    case "breathing": {
      const level = 0.05 + 0.95 * (0.5 - 0.5 * Math.cos(2 * Math.PI * phase(elapsedMs, lighting.period_ms)));
      return lighting.colors.map((c) => {
        const [r, g, b] = parse(c);
        return css([r * level, g * level, b * level]);
      });
    }
    case "color_cycle":
      return Array(ZONES).fill(css(hsv(phase(elapsedMs, lighting.period_ms), 1, 1)));
    case "wave":
    case "hardware_rainbow": {
      const period = lighting.type === "wave" ? lighting.period_ms : 4000;
      const reverse = lighting.type === "wave" && lighting.reverse;
      const p = phase(elapsedMs, period);
      return Array.from({ length: ZONES }, (_, i) => css(hsv(reverse ? p + i / ZONES : p - i / ZONES, 1, 1)));
    }
  }
}
