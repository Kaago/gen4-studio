import { readFileSync } from "node:fs";

/** Where this file lives; the catalogs are found from it. Held in a constant: Vite would take `new URL(…, import.meta.url)` for an asset. */
const HERE = import.meta.url;

/** A string as a literal piece of a regular expression. */
const literal = (text: string): string => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/**
 * Test helper: one sentence of the kernel's catalog (`crates/g4-kernel/locales/<lang>.yml`), so a test can assert that the studio's own words
 * for a kernel refusal still mirror what the kernel says. `section` is the top-level key (`git`), `key` the entry under it
 * (`not_installed`). The text is what stands between the quotes, as written (no escape is resolved, `%{name}` stays); undefined when the
 * section or the key is not in the catalog. Only tests import this: the app never reads the kernel's files.
 */
export function kernelSentence(lang: "de" | "en", section: string, key: string): string | undefined {
  // Relative to this file (`apps/studio/src/testing`), not to the folder the test run starts in.
  const catalog = readFileSync(new URL(`../../../../crates/g4-kernel/locales/${lang}.yml`, HERE), "utf8");
  const body = new RegExp(`^${literal(section)}:\\n((?:(?: .*)?\\n)*)`, "m").exec(catalog)?.[1];
  if (body === undefined) return undefined;
  return new RegExp(`^ {2}${literal(key)}: "((?:[^"\\\\]|\\\\.)*)"$`, "m").exec(body)?.[1];
}
