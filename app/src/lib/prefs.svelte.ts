// Interface preferences stored in the embedded browser (convenience only: if missing, defaults are used).

export type LayoutId =
  | "ansi-us" | "iso-uk" | "iso-it" | "iso-de" | "iso-fr" | "iso-es" | "iso-pt" | "iso-se" | "iso-no" | "iso-dk";

export const LAYOUTS: { id: LayoutId; label: string }[] = [
  { id: "ansi-us", label: "ANSI (US)" },
  { id: "iso-uk", label: "ISO (UK)" },
  { id: "iso-it", label: "ISO (Italian)" },
  { id: "iso-de", label: "ISO (German)" },
  { id: "iso-fr", label: "ISO (French)" },
  { id: "iso-es", label: "ISO (Spanish)" },
  { id: "iso-pt", label: "ISO (Portuguese)" },
  { id: "iso-se", label: "ISO (Swedish/Finnish)" },
  { id: "iso-no", label: "ISO (Norwegian)" },
  { id: "iso-dk", label: "ISO (Danish)" },
];

/** Default layout for the system language. Brazilian Portuguese (ABNT2) has extra keys, so it stays on ANSI. */
const LANGUAGE_LAYOUTS: [string, LayoutId][] = [
  ["it", "iso-it"], ["en-gb", "iso-uk"], ["en-ie", "iso-uk"], ["de", "iso-de"], ["fr", "iso-fr"],
  ["es", "iso-es"], ["pt-pt", "iso-pt"], ["sv", "iso-se"], ["fi", "iso-se"], ["nb", "iso-no"],
  ["nn", "iso-no"], ["no", "iso-no"], ["da", "iso-dk"],
];

const LAYOUT_KEY = "perihub.layout";

function initialLayout(): LayoutId {
  try {
    const saved = localStorage.getItem(LAYOUT_KEY);
    if (LAYOUTS.some((l) => l.id === saved)) return saved as LayoutId;
  } catch {
    /* storage unavailable */
  }
  const lang = navigator.language.toLowerCase();
  const match = LANGUAGE_LAYOUTS.find(([prefix]) => lang === prefix || lang.startsWith(`${prefix}-`));
  return match?.[1] ?? "ansi-us";
}

export const prefs = $state({ layout: initialLayout() });

export function setLayout(layout: LayoutId) {
  prefs.layout = layout;
  try {
    localStorage.setItem(LAYOUT_KEY, layout);
  } catch {
    /* storage unavailable */
  }
}
