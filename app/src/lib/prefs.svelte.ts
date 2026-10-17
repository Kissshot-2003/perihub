// Preferenze dell'interfaccia salvate nel browser integrato (solo comodità: se mancano, si usano i valori predefiniti).

export type LayoutId = "ansi-us" | "iso-uk" | "iso-it";

export const LAYOUTS: { id: LayoutId; label: string }[] = [
  { id: "ansi-us", label: "ANSI (US)" },
  { id: "iso-uk", label: "ISO (UK)" },
  { id: "iso-it", label: "ISO (Italian)" },
];

const LAYOUT_KEY = "perihub.layout";

function initialLayout(): LayoutId {
  try {
    const saved = localStorage.getItem(LAYOUT_KEY);
    if (LAYOUTS.some((l) => l.id === saved)) return saved as LayoutId;
  } catch {
    /* archiviazione non disponibile */
  }
  const lang = navigator.language.toLowerCase();
  if (lang.startsWith("it")) return "iso-it";
  if (lang === "en-gb" || lang === "en-ie") return "iso-uk";
  return "ansi-us";
}

export const prefs = $state({ layout: initialLayout() });

export function setLayout(layout: LayoutId) {
  prefs.layout = layout;
  try {
    localStorage.setItem(LAYOUT_KEY, layout);
  } catch {
    /* archiviazione non disponibile */
  }
}
