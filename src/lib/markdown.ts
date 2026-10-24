import { marked } from "marked";
import TurndownService from "turndown";

const turndown = new TurndownService({
  headingStyle: "atx",
  codeBlockStyle: "fenced",
  bulletListMarker: "-",
  emDelimiter: "*",
});

/** Markdown → HTML (para cargar contenido en el editor WYSIWYG). */
export function markdownToHtml(markdown: string): string {
  return marked.parse(markdown, { async: false }) as string;
}

/** HTML → Markdown (para persistir el contenido del editor WYSIWYG). */
export function htmlToMarkdown(html: string): string {
  return turndown.turndown(html);
}
