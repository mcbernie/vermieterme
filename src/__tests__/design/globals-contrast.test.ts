import { describe, expect, it } from "vitest";
import { contrast, loadTokenBlocks, over, resolveColor, type Decls } from "./css-tokens";

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
    { fg: "--on-accent", bg: "--accent-hover", min: 4.5 },
    { fg: "--on-bad", bg: "--bad", min: 4.5 },
    { fg: "--hdr-ink", bg: "--hdr", min: 4.5 },
    { fg: "--hdr-ink", bg: "--hdr-hover", min: 4.5 },
    { fg: "--hdr-accent", bg: "--hdr", min: 4.5 },
    { fg: "--hdr-accent", bg: "--hdr-hover", min: 4.5 },
    { fg: "--hdr-focus", bg: "--hdr", min: 3 },
    { fg: "--hdr-focus", bg: "--hdr-hover", min: 3 },
    { fg: "--hdr-accent-ink", bg: "--hdr-accent", min: 4.5 },
    { fg: "--band-ink", bg: "--band", min: 4.5 },
    { fg: "--btn2-ink", bg: "--btn2", min: 4.5 },
    { fg: "--btn2-ink", bg: "--btn2-hover", min: 4.5 },
    { fg: "--toast-ink", bg: "--toast", min: 4.5 },
  );
  return list;
}

describe.each(Object.entries(themes))("Kontraste (%s)", (_name, decls) => {
  const col = (t: string) => resolveColor(`var(${t})`, decls);

  it.each(pairs().map((p) => [`${p.fg} auf ${p.bg} (${p.min}:1)`, p] as const))("%s", (_label, p) => {
    const ratio = contrast(col(p.fg), col(p.bg));
    expect(ratio, `${p.fg} auf ${p.bg}: ${ratio.toFixed(2)}`).toBeGreaterThanOrEqual(p.min);
  });

  it.each(["--bg", "--layer", "--field"])("Text auf Akzent-Tönung über %s", (bg) => {
    const tint = over(col("--accent-soft"), col(bg));
    for (const fg of TEXT) {
      const ratio = contrast(col(fg), tint);
      expect(ratio, `${fg} auf Tönung über ${bg}: ${ratio.toFixed(2)}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("Danger-Hover (15 % Schwarz) hält den Text", () => {
    const hover = resolveColor("color-mix(in srgb, var(--bad) 85%, black)", decls);
    expect(contrast(col("--on-bad"), hover)).toBeGreaterThanOrEqual(4.5);
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
