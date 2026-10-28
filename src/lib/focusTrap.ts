/**
 * Acción de Svelte para diálogos modales:
 * - Atrapa Tab/Shift+Tab dentro del contenedor.
 * - Cierra con Escape (opcional).
 * - Enfoca el primer input y restaura el foco al destruirse.
 *
 * Uso: `<div role="dialog" use:trapFocus={{ onClose: () => (open = false) }}>`
 */
export function trapFocus(
  node: HTMLElement,
  options: { onClose?: () => void } = {},
) {
  const previouslyFocused = document.activeElement as HTMLElement | null;
  const selector =
    'a[href], button:not([disabled]), textarea, input, select, [tabindex]:not([tabindex="-1"])';

  function focusables(): HTMLElement[] {
    return Array.from(node.querySelectorAll<HTMLElement>(selector)).filter(
      (el) => el.offsetParent !== null || el === document.activeElement,
    );
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      options.onClose?.();
      return;
    }
    if (event.key !== "Tab") return;

    const items = focusables();
    if (items.length === 0) {
      event.preventDefault();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement as HTMLElement | null;

    if (event.shiftKey && (active === first || !node.contains(active))) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && active === last) {
      event.preventDefault();
      first.focus();
    }
  }

  const initial =
    node.querySelector<HTMLElement>("input, textarea, select") ?? node;
  if (initial === node && !node.hasAttribute("tabindex")) {
    node.setAttribute("tabindex", "-1");
  }
  queueMicrotask(() => initial.focus());

  node.addEventListener("keydown", onKeydown);
  return {
    destroy() {
      node.removeEventListener("keydown", onKeydown);
      previouslyFocused?.focus?.();
    },
  };
}
