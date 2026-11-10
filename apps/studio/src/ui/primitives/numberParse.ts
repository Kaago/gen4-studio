import { t } from "../../i18n";

export type NumberBase = "dec" | "hex";

const DEC = /^-?\d+$/;
const HEX = /^-?(?:0x)?[0-9a-f]+$/i;

/** Reads a whole number in the given base; null when the text is empty or not exactly such a number. */
export function parseNumber(text: string, base: NumberBase): number | null {
  const trimmed = text.trim();
  if (base === "dec") {
    if (!DEC.test(trimmed)) return null;
    return finite(Number.parseInt(trimmed, 10));
  }
  if (!HEX.test(trimmed)) return null;
  const negative = trimmed.startsWith("-");
  const digits = trimmed.replace(/^-/, "").replace(/^0x/i, "");
  const n = Number.parseInt(digits, 16);
  return finite(negative ? -n : n);
}

function finite(n: number): number | null {
  if (!Number.isFinite(n)) return null;
  return n === 0 ? 0 : n; // no negative zero
}

/** Decimal as it is, hex as `0x` plus capitals (`0xFF`). */
export function formatNumber(n: number, base: NumberBase): string {
  if (base === "dec") return String(n);
  return `${n < 0 ? "-" : ""}0x${Math.abs(n).toString(16).toUpperCase()}`;
}

export type NumberCheck = { ok: true; value: number } | { ok: false; error: string };

/**
 * Reads `text` and holds it against the bounds. An entry that is unreadable, too small or too large is refused with a
 * sentence that names the way out; it is never turned into the nearest valid number.
 */
export function checkNumber(text: string, base: NumberBase, min: number, max: number | undefined): NumberCheck {
  if (text.trim() === "") return { ok: false, error: t("ui:numberParse.enterNumber") };
  const n = parseNumber(text, base);
  if (n === null) {
    return {
      ok: false,
      error: base === "hex" ? t("ui:numberParse.notHex") : t("ui:numberParse.notInteger"),
    };
  }
  if (max !== undefined && n > max) return { ok: false, error: t("ui:numberParse.atMost", { max: formatNumber(max, base) }) };
  if (n < min) return { ok: false, error: t("ui:numberParse.atLeast", { min: formatNumber(min, base) }) };
  return { ok: true, value: n };
}
