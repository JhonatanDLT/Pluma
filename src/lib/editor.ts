/**
 * Preferencias tipográficas del editor, persistidas en `localStorage` y
 * aplicadas como variables CSS para que las compartan WYSIWYG y código fuente.
 */

export interface EditorSettings {
  /** Tamaño del texto en píxeles. */
  fontSize: number;
  /** Interlineado (multiplicador). */
  lineHeight: number;
  /** Ancho máximo de la columna de escritura en píxeles. */
  measure: number;
}

const KEY = "pluma.editor";

export const DEFAULT_EDITOR_SETTINGS: EditorSettings = {
  fontSize: 18,
  lineHeight: 1.75,
  measure: 720,
};

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export function getEditorSettings(): EditorSettings {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return { ...DEFAULT_EDITOR_SETTINGS };
    const parsed = JSON.parse(raw) as Partial<EditorSettings>;
    return {
      fontSize: clamp(parsed.fontSize ?? DEFAULT_EDITOR_SETTINGS.fontSize, 14, 26),
      lineHeight: clamp(
        parsed.lineHeight ?? DEFAULT_EDITOR_SETTINGS.lineHeight,
        1.3,
        2.2,
      ),
      measure: clamp(parsed.measure ?? DEFAULT_EDITOR_SETTINGS.measure, 520, 900),
    };
  } catch {
    return { ...DEFAULT_EDITOR_SETTINGS };
  }
}

export function saveEditorSettings(settings: EditorSettings): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(settings));
  } catch {
    // Sin localStorage se usan los valores por defecto durante la sesión.
  }
}

export function applyEditorSettings(settings: EditorSettings): void {
  const root = document.documentElement;
  root.style.setProperty("--editor-font-size", `${settings.fontSize}px`);
  root.style.setProperty("--editor-line-height", String(settings.lineHeight));
  root.style.setProperty("--editor-measure", `${settings.measure}px`);
}
