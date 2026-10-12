import MarkdownIt from "markdown-it";
import hljs from "highlight.js/lib/common";

// Raw HTML is escaped, never parsed, and markdown-it refuses javascript:/vbscript:/file:/data: links (BUB-4.2).
// Images are off so an answer can't make the bubble fetch a URL the user never chose (PRI-4.1).
const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
  // Only fenced blocks that name their language: guessing would re-scan every block on each streamed token.
  highlight: (code, lang) => (lang && hljs.getLanguage(lang) ? hljs.highlight(code, { language: lang, ignoreIllegals: true }).value : ""),
}).disable("image");

export const render = (markdown: string): string => md.render(markdown);
