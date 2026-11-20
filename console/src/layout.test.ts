// TECHNICAL 18.4: the layout values, read from the stylesheets (jsdom computes no layout; the slow test checks a real browser).
import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const theme = readFileSync(join(__dirname, "theme.css"), "utf8");
const kit = readFileSync(join(__dirname, "../kit/styles/kit.css"), "utf8");
const all = theme + "\n" + kit;
const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
/** The declarations of the rule whose selector list contains `sel`, outside any at-rule unless `within` is given. */
function rule(css: string, sel: string): string {
  const m = new RegExp(`(?:^|[{}])\\s*${esc(sel)}\\s*\\{([^}]*)\\}`, "m").exec(css);
  if (!m) throw new Error(`no rule for ${sel}`);
  return m[1];
}
const block = (css: string, head: string): string => {
  const at = css.indexOf(head);
  if (at < 0) throw new Error(`no ${head}`);
  let depth = 0;
  for (let i = css.indexOf("{", at); i < css.length; i++) {
    if (css[i] === "{") depth++;
    if (css[i] === "}" && --depth === 0) return css.slice(css.indexOf("{", at) + 1, i);
  }
  throw new Error("unbalanced");
};
const token = (css: string, name: string) => new RegExp(`${esc(name)}:\\s*([^;]+);`).exec(css)?.[1].trim();

describe("layout", () => {
  it("ac5_light_tokens_exactly_and_dark_values_under_prefers_color_scheme", () => {
    const root = rule(theme, ":root");
    const want = { "--bg": "#F6F6F4", "--panel": "#FFFFFF", "--ink": "#18181B", "--muted": "#5D5E66", "--line": "#E3E3DF", "--accent": "#3355D6" };
    for (const [k, v] of Object.entries(want)) expect(token(root, k)).toBe(v);
    const dark = block(theme, "@media (prefers-color-scheme: dark)");
    for (const k of Object.keys(want)) expect(token(dark, k)).toMatch(/^#[0-9A-Fa-f]{6}$/);
    for (const k of Object.keys(want)) expect(token(dark, k)).not.toBe(want[k as keyof typeof want]);
  });

  it("ac5_sidebar_260_hidden_under_760_column_780", () => {
    expect(token(rule(theme, ":root"), "--sidebar")).toBe("260px");
    expect(rule(theme, ".wa-sidebar")).toMatch(/width:\s*var\(--sidebar\)/);
    expect(block(theme, "@media (max-width: 759.98px)")).toMatch(/\.wa-sidebar\s*\{\s*display:\s*none/);
    expect(rule(theme, ".wa-column")).toMatch(/max-width:\s*780px/);
  });

  it("ac5_heights_and_radii", () => {
    expect(rule(theme, "button")).toMatch(/min-height:\s*36px/);
    expect(rule(theme, ".wa-chip")).toMatch(/height:\s*32px/);
    expect(rule(kit, ".wa-row")).toMatch(/min-height:\s*44px/);
    expect(rule(theme, ".wa-session-row")).toMatch(/min-height:\s*44px/);
    const root = rule(theme, ":root");
    expect(token(root, "--radius-button")).toBe("9px");
    expect(token(root, "--radius-input")).toBe("18px");
    const card = Number(/(\d+)px/.exec(token(root, "--radius-card")!)![1]);
    expect(card).toBeGreaterThanOrEqual(12);
    expect(card).toBeLessThanOrEqual(14);
    expect(rule(theme, "button")).toMatch(/border-radius:\s*var\(--radius-button\)/);
    expect(rule(theme, ".wa-composer")).toMatch(/border-radius:\s*var\(--radius-input\)/);
    expect(rule(kit, ".wa-tool")).toMatch(/border-radius:\s*var\(--radius-card\)/);
  });

  it("ac5_fonts_named_first_and_nothing_outside", () => {
    const root = rule(theme, ":root");
    expect(token(root, "--font-sans")).toMatch(/^"Inter",/);
    expect(token(root, "--font-mono")).toMatch(/^"JetBrains Mono",/);
    expect(all).not.toMatch(/(?:https?:)?\/\/[a-z]/i);
    for (const m of theme.matchAll(/url\("([^"]+)"\)/g)) expect(m[1]).toMatch(/^\.\.\/node_modules\/@fontsource\//);
  });

  it("ac5_motion_hover_and_pressed_rules_sit_under_their_media_queries", () => {
    const motion = block(theme, "@media (prefers-reduced-motion: no-preference)");
    expect(motion).toMatch(/transition:[^;]*150ms/);
    expect(motion).toMatch(/:active\s*\{\s*transform:\s*scale\(0\.96\)/);
    expect(block(theme, "@media (hover: hover)")).toMatch(/:hover/);
    const outside = all.replace(block(theme, "@media (prefers-reduced-motion: no-preference)"), "").replace(block(theme, "@media (hover: hover)"), "");
    expect(outside).not.toMatch(/:hover|:active|transition:|animation:/);
  });

  it("ac7_ask_arguments_output_and_diff_keep_their_line_breaks", () => {
    for (const sel of [/\.wa-args dd\s*\{[^}]*white-space:\s*pre-wrap/, /\.wa-diff, \.wa-output\s*\{[^}]*white-space:\s*pre-wrap/, /\.dl\s*\{[^}]*white-space:\s*pre-wrap/]) expect(kit).toMatch(sel);
  });

  it("s9_1_diff_tints_chevron_turn_and_button_radius", () => {
    expect(kit).toMatch(/\.dl\.add\s*\{[^}]*background:\s*var\(--wa-add\)/);
    expect(kit).toMatch(/\.dl\.del\s*\{[^}]*background:\s*var\(--wa-del\)/);
    expect(kit).toMatch(/\.foldbtn\.open \.chev\s*\{[^}]*transform:\s*rotate\(90deg\)/);
    expect(kit).toMatch(/\.btn\s*\{[^}]*height:\s*36px[^}]*border-radius:\s*var\(--radius-button\)/);
  });
});
