import { readFileSync } from "node:fs";

/** Test helper: the CSS source next to a test file, so a test can assert layout rules jsdom cannot compute. */
export function readCss(relative: string, from: string): string {
  return readFileSync(new URL(relative, from), "utf8");
}

/** The declarations of every plain rule whose selector list contains `selector` (grouped rules included), joined; "" when there is none. */
export function ruleBody(css: string, selector: string): string {
  const stripped = css.replace(/\/\*[\s\S]*?\*\//g, "");
  return [...stripped.matchAll(/([^{}]+)\{([^{}]*)\}/g)]
    .filter((match) => match[1].split(",").some((s) => s.trim() === selector))
    .map((match) => match[2])
    .join("\n");
}
