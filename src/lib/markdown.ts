import { marked } from "marked";
import TurndownService from "turndown";
import DOMPurify from "dompurify";
import { convertFileSrc } from "@tauri-apps/api/core";

const turndown = new TurndownService({
  headingStyle: "atx",
  codeBlockStyle: "fenced",
  bulletListMarker: "-",
  emDelimiter: "*",
});

/** Una ruta de archivo local (absoluta) que debe mostrarse vía `asset://`. */
function isLocalPath(path: string): boolean {
  return path.startsWith("/") || /^[A-Za-z]:[\\/]/.test(path);
}

// Turndown no convierte tablas por defecto; añadimos soporte GFM propio.
turndown.addRule("table", {
  filter: (node) => node.nodeName === "TABLE",
  replacement: (_content, node) => {
    const el = node as HTMLElement;
    const rows = Array.from(el.querySelectorAll("tr"));
    if (rows.length === 0) return "";
    const lines = rows.map((row) => {
      const cells = Array.from(row.children).map(
        (c) => c.textContent?.trim() ?? "",
      );
      return `| ${cells.join(" | ")} |`;
    });
    const cols = rows[0].children.length;
    lines.splice(1, 0, `|${" --- |".repeat(cols)}`);
    return `\n\n${lines.join("\n")}\n\n`;
  },
});

// Las imágenes conservan su ruta absoluta en `data-path` y sus dimensiones.
turndown.addRule("image", {
  filter: "img",
  replacement: (_content, node) => {
    const el = node as HTMLElement;
    const alt = el.getAttribute("alt") ?? "";
    const src = el.getAttribute("data-path") ?? el.getAttribute("src") ?? "";
    const width = el.getAttribute("width");
    const height = el.getAttribute("height");
    if (width || height) {
      const attrs = [`src="${src}"`, `alt="${alt}"`];
      if (width) attrs.push(`width="${width}"`);
      if (height) attrs.push(`height="${height}"`);
      return `<img ${attrs.join(" ")}>`;
    }
    return `![${alt}](${src})`;
  },
});

/** Markdown → HTML (para cargar contenido en el editor WYSIWYG). */
export function markdownToHtml(markdown: string): string {
  const raw = marked.parse(markdown, { async: false }) as string;
  // El manuscrito puede venir de archivos externos: nunca inyectamos HTML
  // crudo en el DOM sin sanitizar. `data-path` se conserva para las imágenes
  // locales que se resuelven vía `asset://`.
  const html = DOMPurify.sanitize(raw, {
    USE_PROFILES: { html: true },
    ADD_ATTR: ["data-path", "width", "height"],
  });
  // Convierte rutas locales de imagen a URLs de asset y añade `data-path`.
  return html.replace(
    /<img\b([^>]*?)\bsrc="([^"]+)"/g,
    (_match, attrs: string, src: string) => {
      if (!isLocalPath(src)) return `<img${attrs} src="${src}"`;
      return `<img${attrs} src="${convertFileSrc(src)}" data-path="${src}"`;
    },
  );
}

/** HTML → Markdown (para persistir el contenido del editor WYSIWYG). */
export function htmlToMarkdown(html: string): string {
  return turndown.turndown(html);
}
