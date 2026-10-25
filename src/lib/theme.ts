// Gestión del tema de color (paletas predefinidas para lectura/escritura).

export type ThemeId = "system" | "light" | "dark" | "sepia" | "paper";

export interface Theme {
  id: ThemeId;
  label: string;
  swatch: string;
}

export const THEMES: Theme[] = [
  { id: "system", label: "Sistema", swatch: "linear-gradient(135deg, #f6f4f0 50%, #1b1916 50%)" },
  { id: "light", label: "Claro", swatch: "#f6f4f0" },
  { id: "dark", label: "Oscuro", swatch: "#1b1916" },
  { id: "sepia", label: "Sepia", swatch: "#f4ecd8" },
  { id: "paper", label: "Papel", swatch: "#ffffff" },
];

const KEY = "pluma.theme";

function isTheme(value: string | null): value is ThemeId {
  return THEMES.some((t) => t.id === value);
}

export function getTheme(): ThemeId {
  const saved = localStorage.getItem(KEY);
  return isTheme(saved) ? saved : "system";
}

function resolve(t: ThemeId): "light" | "dark" | "sepia" | "paper" {
  if (t === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return t;
}

export function applyTheme(t: ThemeId) {
  document.documentElement.dataset.theme = resolve(t);
  localStorage.setItem(KEY, t);
}
