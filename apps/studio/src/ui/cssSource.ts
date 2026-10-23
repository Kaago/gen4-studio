import { readFileSync } from "node:fs";

/** Test helper: the CSS source next to a test file, so a test can assert layout rules jsdom cannot compute. */
export function readCss(relative: string, from: string): string {
  return readFileSync(new URL(relative, from), "utf8");
}

/** The declaration body of the first rule whose selector list contains `selector`; "" when there is none. */
export function ruleBody(css: string, selector: string): string {
  const stripped = css.replace(/\/\*[\s\S]*?\*\//g, "");
  for (const match of stripped.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    if (match[1].split(",").some((s) => s.trim() === selector)) return match[2];
  }
  return "";
}
