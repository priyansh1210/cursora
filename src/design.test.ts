// @ts-expect-error type error without @types/node package
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

// Read from disk: vitest turns an imported stylesheet into an empty string.
const css: string = readFileSync(new URL("./styles.css", import.meta.url), "utf8");

/** The custom properties declared in the block that starts with `selector {`. */
function tokens(selector: string): Record<string, string> {
  const start = css.indexOf(`${selector} {`);
  const body = css.slice(start, css.indexOf("}", start));
  return Object.fromEntries([...body.matchAll(/(--[\w-]+):\s*([^;]+);/g)].map((m) => [m[1], m[2].trim()]));
}
const light = tokens(":root");
const dark = { ...light, ...tokens(':root[data-theme="dark"]') };

type RGBA = [number, number, number, number];
function parse(color: string): RGBA {
  const hex = color.match(/^#([0-9a-f]{3}|[0-9a-f]{6})$/i)?.[1];
  if (hex) {
    const full = hex.length === 3 ? [...hex].map((c) => c + c).join("") : hex;
    return [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16)).concat(1) as RGBA;
  }
  const rgb = color.match(/^rgb\((\d+) (\d+) (\d+) \/ (\d+)%\)$/);
  if (rgb) return [Number(rgb[1]), Number(rgb[2]), Number(rgb[3]), Number(rgb[4]) / 100];
  throw new Error(`not a colour: ${color}`);
}
/** A colour as seen on an opaque background. */
const over = ([r, g, b, a]: RGBA, [R, G, B]: RGBA): RGBA => [r * a + R * (1 - a), g * a + G * (1 - a), b * a + B * (1 - a), 1];
const luminance = ([r, g, b]: RGBA) => {
  const lin = (c: number) => ((c /= 255) <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
};
function contrast(theme: Record<string, string>, fg: string, bg: string, under = "--bg") {
  const base = over(parse(theme[bg]), parse(theme[under]));
  const [a, b] = [luminance(over(parse(theme[fg]), base)), luminance(base)].sort((x, y) => y - x);
  return (a + 0.05) / (b + 0.05);
}

// [foreground, background, minimum]: 4.5 for text, 3 for focus rings and the switch's parts (WCAG 1.4.3, 1.4.11).
const PAIRS: [string, string, number][] = [
  ...["--bg", "--surface", "--surface-2", "--sidebar", "--hover", "--selected"].flatMap((bg): [string, string, number][] => [
    ["--fg", bg, 4.5],
    ["--muted", bg, 4.5],
  ]),
  ["--accent-text", "--surface", 4.5],
  ["--accent-text", "--bg", 4.5],
  ["--accent-text", "--accent-soft", 4.5],
  ["--fg", "--accent-soft", 4.5],
  ["--on-accent", "--accent", 4.5],
  ["--error", "--surface", 4.5],
  ["--error", "--error-soft", 4.5],
  ["--hl-keyword", "--code-bg", 4.5],
  ["--hl-string", "--code-bg", 4.5],
  ["--hl-number", "--code-bg", 4.5],
  ["--hl-title", "--code-bg", 4.5],
  ["--focus", "--bg", 3],
  ["--focus", "--surface", 3],
  ["--muted", "--surface", 3], // the switch, off
  ["--accent", "--surface", 3], // the switch, on
  ["--switch-thumb-on", "--accent", 3],
];

describe("design tokens", () => {
  it("DSN-1.1 colours appear only as theme tokens", () => {
    const literals = css.split("\n").filter((line) => /#[0-9a-f]{3,8}\b|\b(rgb|hsl)a?\(/i.test(line) && !/^\s*--[\w-]+:/.test(line));
    expect(literals).toEqual([]);
  });

  it("DSN-1.1 the dark theme is the same whether the system or the user picks it", () => {
    const start = css.indexOf("@media (prefers-color-scheme: dark)");
    const system = Object.fromEntries([...css.slice(start, css.indexOf("}", start)).matchAll(/(--[\w-]+):\s*([^;]+);/g)].map((m) => [m[1], m[2].trim()]));
    expect(system).toEqual(tokens(':root[data-theme="dark"]'));
  });

  it("DSN-1.2 no text is smaller than 12 px", () => {
    const sizes = [...css.matchAll(/(?:font-size|font|--text-[\w-]+):[^;]*?(\d+(?:\.\d+)?)px/g)].map((m) => Number(m[1]));
    expect(sizes.length).toBeGreaterThan(5);
    expect(sizes.filter((px) => px < 12)).toEqual([]);
  });

  it("DSN-1.3 text, focus rings and switches meet WCAG AA in both themes", () => {
    const failing = [
      ["light", light],
      ["dark", dark],
    ].flatMap(([name, theme]) =>
      PAIRS.filter(([fg, bg, min]) => contrast(theme as Record<string, string>, fg, bg) < min).map(([fg, bg, min]) => `${name}: ${fg} on ${bg} is ${contrast(theme as Record<string, string>, fg, bg).toFixed(2)}, needs ${min}`),
    );
    expect(failing).toEqual([]);
  });
});
