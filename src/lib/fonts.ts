// Gestión de tipografía: fuentes por rol (texto, interfaz, código) y fuentes
// personalizadas cargadas por el usuario.

import { convertFileSrc } from "@tauri-apps/api/core";

export type FontSlot = "serif" | "ui" | "mono";

export interface FontSettings {
  serif: string;
  ui: string;
  mono: string;
}

export interface CustomFont {
  name: string;
  path: string;
}

const KEY = "pluma.fonts";
const CUSTOM_KEY = "pluma.custom-fonts";

const DEFAULTS: FontSettings = {
  serif:
    '"Iowan Old Style", "Palatino Linotype", Palatino, Georgia, "Times New Roman", serif',
  ui: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Inter, Helvetica, Arial, sans-serif',
  mono: 'ui-monospace, "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace',
};

export const FONT_PRESETS: Record<FontSlot, { label: string; value: string }[]> = {
  serif: [
    { label: "Palatino / Iowan", value: '"Iowan Old Style", "Palatino Linotype", Palatino, Georgia, serif' },
    { label: "Georgia", value: 'Georgia, "Times New Roman", serif' },
    { label: "Times New Roman", value: '"Times New Roman", Times, serif' },
    { label: "Garamond", value: 'Garamond, "Adobe Garamond Pro", Georgia, serif' },
  ],
  ui: [
    { label: "Sistema", value: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif' },
    { label: "Inter", value: 'Inter, -apple-system, "Segoe UI", sans-serif' },
    { label: "Helvetica / Arial", value: 'Helvetica, Arial, sans-serif' },
  ],
  mono: [
    { label: "Sistema", value: 'ui-monospace, "SF Mono", Menlo, Consolas, monospace' },
    { label: "JetBrains Mono", value: '"JetBrains Mono", ui-monospace, monospace' },
    { label: "Courier New", value: '"Courier New", Courier, monospace' },
  ],
};

export function getFontSettings(): FontSettings {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return { ...DEFAULTS, ...JSON.parse(raw) };
  } catch {
    /* ignorar */
  }
  return { ...DEFAULTS };
}

export function saveFontSettings(settings: FontSettings) {
  localStorage.setItem(KEY, JSON.stringify(settings));
}

export function applyFontSettings(settings: FontSettings) {
  const root = document.documentElement.style;
  root.setProperty("--font-serif", settings.serif);
  root.setProperty("--font-ui", settings.ui);
  root.setProperty("--font-mono", settings.mono);
}

// --- Fuentes personalizadas (archivos cargados por el usuario) ---

export function getCustomFonts(): CustomFont[] {
  try {
    const raw = localStorage.getItem(CUSTOM_KEY);
    if (raw) return JSON.parse(raw) as CustomFont[];
  } catch {
    /* ignorar */
  }
  return [];
}

function saveCustomFonts(fonts: CustomFont[]) {
  localStorage.setItem(CUSTOM_KEY, JSON.stringify(fonts));
}

async function loadFontFace(name: string, path: string): Promise<string> {
  const src = convertFileSrc(path);
  const face = new FontFace(name, `url("${src}")`);
  await face.load();
  document.fonts.add(face);
  return name;
}

/** Registra una fuente personalizada desde un archivo (.ttf/.otf/.woff). */
export async function addCustomFont(path: string): Promise<CustomFont> {
  const base =
    path.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") ?? "Fuente personalizada";
  const name = base.replace(/[-_]+/g, " ").trim() || "Fuente personalizada";
  await loadFontFace(name, path);
  const fonts = getCustomFonts();
  if (!fonts.some((f) => f.path === path)) {
    fonts.push({ name, path });
    saveCustomFonts(fonts);
  }
  return { name, path };
}

export async function removeCustomFont(name: string): Promise<void> {
  const fonts = getCustomFonts().filter((f) => f.name !== name);
  saveCustomFonts(fonts);
}

/** Vuelve a registrar las fuentes personalizadas al iniciar la app. */
export async function registerCustomFonts(): Promise<void> {
  const fonts = getCustomFonts();
  await Promise.all(
    fonts.map((f) => loadFontFace(f.name, f.path).catch(() => undefined)),
  );
}
