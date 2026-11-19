import { readFileSync } from "node:fs";
import path from "node:path";

/** An sRGB colour with alpha, channels 0-255, alpha 0-1. */
export interface Rgba {
  r: number;
  g: number;
  b: number;
  a: number;
}

/** Custom property name (with leading `--`) to raw value. */
export type Decls = Record<string, string>;

const CSS_PATH = path.resolve(__dirname, "../../app/globals.css");

/** Reads `src/app/globals.css` without comments. */
export function readGlobalsCss(): string {
  return readFileSync(CSS_PATH, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
}

/** Returns the text inside the braces of the first block whose header starts with `header` (brace matched). */
export function blockBody(css: string, header: string): string {
  const start = css.indexOf(header);
  if (start < 0) throw new Error(`Block not found: ${header}`);
  const open = css.indexOf("{", start);
  let depth = 0;
  for (let i = open; i < css.length; i++) {
    if (css[i] === "{") depth++;
    if (css[i] === "}" && --depth === 0) return css.slice(open + 1, i);
  }
  throw new Error(`Unclosed block: ${header}`);
}

/** Parses `--name: value;` declarations of a block body. */
export function parseDecls(body: string): Decls {
  const out: Decls = {};
  for (const m of body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) out[m[1]] = m[2].trim().replace(/\s+/g, " ");
  return out;
}

/** The token sets of the light default and of the two dark blocks (media query and attribute). */
export function loadTokenBlocks(): { light: Decls; darkMedia: Decls; darkAttr: Decls } {
  const css = readGlobalsCss();
  const media = blockBody(css, "@media (prefers-color-scheme: dark)");
  return {
    light: parseDecls(blockBody(css, ":root {")),
    darkMedia: parseDecls(blockBody(media, ":root:not([data-zui-theme='light'])")),
    darkAttr: parseDecls(blockBody(css, ":root[data-zui-theme='dark']")),
  };
}

/** The `--zui-*` mapping block (the `:root` block that declares `--zui-bg`): library token to app token. */
export function loadZuiMapping(): Decls {
  const css = readGlobalsCss();
  const at = css.indexOf("--zui-bg:");
  if (at < 0) throw new Error("--zui-* mapping not found");
  return parseDecls(blockBody(css, css.slice(css.lastIndexOf(":root {", at))));
}

/** Declarations of the `.app-header` rule (the tokens the library header really receives). */
export function loadHeaderDecls(): Decls {
  return parseDecls(blockBody(readGlobalsCss(), ".app-header {"));
}

/** The hover fill of the library's danger button (`color-mix(...)` with `var(--zui-bad)`), read from its CSS. */
export function libraryDangerHover(): string {
  const lib = readFileSync(path.resolve(__dirname, "../../../node_modules/@zweiund40gmbh/ui/dist/styles.css"), "utf8");
  const m = /\.zui-button\[data-variant='danger'\]:hover:not\(:disabled\)\s*\{\s*background:\s*([^;]+);/.exec(lib);
  if (!m) throw new Error("Danger hover rule not found in the library CSS");
  return m[1].trim();
}

function hex(value: string): Rgba {
  const h = value.slice(1);
  const f = h.length === 3 ? h.replace(/./g, "$&$&") : h;
  const n = parseInt(f, 16);
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255, a: 1 };
}

/**
 * Resolves a token value to a colour: hex, `var(--x)` (looked up in `decls`) and
 * `color-mix(in srgb, X p%, transparent|black)`.
 */
export function resolveColor(value: string, decls: Decls, depth = 0): Rgba {
  if (depth > 10) throw new Error(`var() cycle at ${value}`);
  const v = value.trim();
  if (v.startsWith("#")) return hex(v);
  const ref = /^var\((--[\w-]+)\)$/.exec(v);
  if (ref) {
    const next = decls[ref[1]];
    if (next === undefined) throw new Error(`Unknown token ${ref[1]}`);
    return resolveColor(next, decls, depth + 1);
  }
  const mix = /^color-mix\(in srgb,\s*(.+?)\s+(\d+(?:\.\d+)?)%,\s*(transparent|black)\)$/.exec(v);
  if (mix) {
    const base = resolveColor(mix[1], decls, depth + 1);
    const p = Number(mix[2]) / 100;
    if (mix[3] === "transparent") return { ...base, a: base.a * p };
    return { r: base.r * p, g: base.g * p, b: base.b * p, a: base.a };
  }
  throw new Error(`Unsupported colour value: ${v}`);
}

/** Composites `fg` over an opaque `bg`. */
export function over(fg: Rgba, bg: Rgba): Rgba {
  const mixc = (f: number, b: number) => f * fg.a + b * (1 - fg.a);
  return { r: mixc(fg.r, bg.r), g: mixc(fg.g, bg.g), b: mixc(fg.b, bg.b), a: 1 };
}

function lin(c: number): number {
  const s = c / 255;
  return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}

/** WCAG 2.x contrast ratio of two opaque colours. */
export function contrast(a: Rgba, b: Rgba): number {
  const l = (c: Rgba) => 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
  const [hi, lo] = [l(a), l(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}
