import { describe, expect, it } from "vitest";
import { contrast, libraryDangerHover, loadHeaderDecls, loadTokenBlocks, loadZuiMapping, over, resolveColor, type Decls } from "./css-tokens";

/**
 * Single source of truth for the colour contrasts of `src/app/globals.css`: WCAG 2.x, text 4.5:1,
 * borders and focus rings 3:1. Every pair listed here is really used (app or @zweiund40gmbh/ui).
 */
const blocks = loadTokenBlocks();
const themes: Record<string, Decls> = {
  light: blocks.light,
  dark: { ...blocks.light, ...blocks.darkAttr },
};

const SURFACES = ["--bg", "--layer", "--field", "--hover", "--th"];
const TEXT = ["--text", "--text2", "--text3", "--accent-strong"];
const STATUS = ["--ok", "--warn", "--bad", "--info"];
const SOFT: Record<string, string> = { "--ok": "--ok-soft", "--warn": "--warn-soft", "--bad": "--bad-soft", "--info": "--info-soft" };

interface Pair {
  fg: string;
  bg: string;
  min: number;
}

/**
 * Foreground/background token pairs with their minimum ratio. Accent and status text on `--hover` is
 * deliberately only checked against 3:1 below: hover is a transient state.
 */
function pairs(): Pair[] {
  const list: Pair[] = [];
  for (const bg of SURFACES) {
    for (const fg of TEXT) list.push({ fg, bg, min: 4.5 });
  }
  for (const bg of SURFACES.filter((s) => s !== "--hover")) {
    for (const fg of STATUS) list.push({ fg, bg, min: 4.5 });
  }
  for (const fg of ["--text", "--text2", "--text3"]) list.push({ fg, bg: "--gray-soft", min: 4.5 });
  for (const bg of ["--bg", "--layer", "--field"]) {
    list.push({ fg: "--accent", bg, min: 4.5 }, { fg: "--border-strong", bg, min: 3 });
    list.push({ fg: "--accent", bg, min: 3 });
  }
  for (const [fg, soft] of Object.entries(SOFT)) list.push({ fg, bg: soft, min: 4.5 });
  list.push(
    { fg: "--on-accent", bg: "--accent-fill", min: 4.5 },
    { fg: "--on-accent", bg: "--accent-strong", min: 4.5 }, // hover fill of primary buttons
    { fg: "--on-bad", bg: "--bad", min: 4.5 },
    { fg: "--hdr-ink", bg: "--hdr", min: 4.5 },
    { fg: "--hdr-ink", bg: "--hdr-hover", min: 4.5 },
    { fg: "--hdr-accent", bg: "--hdr", min: 4.5 },
    { fg: "--hdr-accent", bg: "--hdr-hover", min: 4.5 },
    { fg: "--hdr-focus", bg: "--hdr", min: 3 },
    { fg: "--hdr-focus", bg: "--hdr-hover", min: 3 },
    { fg: "--hdr-accent-ink", bg: "--hdr-accent", min: 4.5 },
  );
  return list;
}

const HEADER_PAIRS: Pair[] = [
  { fg: "--zui-header-text", bg: "--zui-header-bg", min: 4.5 },
  { fg: "--zui-header-text", bg: "--zui-header-hover", min: 4.5 },
  { fg: "--zui-accent-strong", bg: "--zui-header-bg", min: 4.5 },
  { fg: "--zui-accent-strong", bg: "--zui-header-hover", min: 4.5 },
  { fg: "--zui-accent", bg: "--zui-header-bg", min: 4.5 },
  { fg: "--zui-accent", bg: "--zui-header-hover", min: 4.5 },
  { fg: "--zui-accent-contrast", bg: "--zui-accent", min: 4.5 },
  { fg: "--zui-focus", bg: "--zui-header-bg", min: 3 },
  { fg: "--zui-focus", bg: "--zui-header-hover", min: 3 },
];

describe.each(Object.entries(themes))("Kontraste (%s)", (_name, decls) => {
  const col = (t: string) => resolveColor(`var(${t})`, decls);

  it.each(pairs().map((p) => [`${p.fg} auf ${p.bg} (${p.min}:1)`, p] as const))("%s", (_label, p) => {
    const ratio = contrast(col(p.fg), col(p.bg));
    expect(ratio, `${p.fg} auf ${p.bg}: ${ratio.toFixed(2)}`).toBeGreaterThanOrEqual(p.min);
  });

  it.each(HEADER_PAIRS.map((p) => [`${p.fg} auf ${p.bg} (${p.min}:1)`, p] as const))(
    "Kopfzeile: %s",
    (_label, p) => {
      const header = { ...decls, ...loadZuiMapping(), ...loadHeaderDecls() };
      const c = (t: string) => resolveColor(`var(${t})`, header);
      const ratio = contrast(c(p.fg), c(p.bg));
      expect(ratio, `${p.fg} auf ${p.bg}: ${ratio.toFixed(2)}`).toBeGreaterThanOrEqual(p.min);
    },
  );

  it("Kopfzeile setzt die Tokens, die die Bibliothek wirklich liest", () => {
    const h = loadHeaderDecls();
    for (const t of ["--zui-header-bg", "--zui-header-text", "--zui-header-hover", "--zui-accent-strong"]) {
      expect(h[t], t).toBeDefined();
    }
  });

  it.each(["--bg", "--layer", "--field"])("Text auf Akzent-Tönung über %s", (bg) => {
    const tint = over(col("--accent-soft"), col(bg));
    for (const fg of TEXT) {
      const ratio = contrast(col(fg), tint);
      expect(ratio, `${fg} auf Tönung über ${bg}: ${ratio.toFixed(2)}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("Danger-Hover der Bibliothek hält den Text", () => {
    const hover = resolveColor(libraryDangerHover(), { ...decls, ...loadZuiMapping() });
    expect(contrast(col("--on-bad"), hover)).toBeGreaterThanOrEqual(4.5);
  });

  it("--accent-strong unterscheidet sich von --accent (sichtbarer Hover)", () => {
    expect(contrast(col("--accent-strong"), col("--accent"))).toBeGreaterThan(1);
    expect(decls["--accent-strong"]).not.toEqual(decls["--accent"]);
  });

  it("--zui-*-Zuordnung zeigt auf die richtigen App-Tokens", () => {
    const m = loadZuiMapping();
    expect(m["--zui-focus"]).toBe("var(--accent)");
    expect(m["--zui-accent-contrast"]).toBe("var(--on-accent)");
    expect(m["--zui-accent-strong"]).toBe("var(--accent-strong)");
    expect(m["--zui-bad-contrast"]).toBe("var(--on-bad)");
    for (const [k, v] of Object.entries(m)) {
      if (/^--zui-(radius|shadow|font-)/.test(k)) continue;
      expect(v, k).toMatch(/^var\(--[\w-]+\)$/);
      if (k !== "--zui-overlay") expect(() => resolveColor(v, decls), k).not.toThrow();
    }
  });

  it("Akzent und Status auf der Hover-Fläche (vorübergehend) mindestens 3:1", () => {
    for (const fg of ["--accent", ...STATUS]) {
      const ratio = contrast(col(fg), col("--hover"));
      expect(ratio, `${fg} auf --hover: ${ratio.toFixed(2)}`).toBeGreaterThanOrEqual(3);
    }
  });
});

describe("Dunkel-Blöcke", () => {
  it("Media-Query und Attribut deklarieren dasselbe", () => {
    expect(blocks.darkMedia).toEqual(blocks.darkAttr);
  });
});
