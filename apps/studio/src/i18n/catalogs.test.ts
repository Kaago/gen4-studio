import { describe, expect, it } from "vitest";
import { resources } from "./index";

type Catalog = Record<string, unknown>;

/** Dotted path to every leaf: `{ a: { b: "x" } }` gives `a.b`. */
function flatten(obj: Catalog, prefix = ""): Map<string, unknown> {
  const out = new Map<string, unknown>();
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix === "" ? key : `${prefix}.${key}`;
    if (typeof value === "object" && value !== null && !Array.isArray(value)) {
      for (const [p, v] of flatten(value as Catalog, path)) out.set(p, v);
    } else {
      out.set(path, value);
    }
  }
  return out;
}

/** i18next's `{{name}}` and `{{name, format}}` (`{{count, number}}`): compared by the name. */
const NAMED = /\{\{\s*(\w+)\s*(?:,[^}]*)?\}\}/g;
/** The check catalog's own `{1}` and `{1:id}` (`checks/messages.ts`): compared with the kind. */
const NUMBERED = /(?<!\{)\{(\d+(?::\w+)?)\}(?!\})/g;

/** The placeholders of `value`, each once, sorted: `{{name}}` for a named one (format dropped), `{n}`/`{n:kind}` for a numbered one. */
function placeholders(value: string): string[] {
  const named = [...value.matchAll(NAMED)].map((m) => `{{${m[1]}}}`);
  const numbered = [...value.matchAll(NUMBERED)].map((m) => `{${m[1]}}`);
  return [...new Set([...named, ...numbered])].sort();
}

describe("placeholders", () => {
  it("reads named, formatted and numbered placeholders", () => {
    expect(placeholders("{{count, number}} of {{total}}, again {{ count }}")).toEqual(["{{count}}", "{{total}}"]);
    expect(placeholders("Trainer {0:id} uses {1} and {2:num}, not {x} or {{3}}")).toEqual(["{0:id}", "{1}", "{2:num}", "{{3}}"]);
    expect(placeholders("none")).toEqual([]);
  });

  it("tells a dropped format apart from nothing, and a changed kind apart", () => {
    // The name decides for i18next's placeholders: the format may differ, the name may not.
    expect(placeholders("{{count, number}} Dateien")).toEqual(placeholders("{{count}} files"));
    expect(placeholders("{{count, number}} Dateien")).not.toEqual(placeholders("{{n, number}} files"));
    // A numbered placeholder keeps its kind: `{1:id}` and `{1}` are different.
    expect(placeholders("Header {1:id}")).not.toEqual(placeholders("Header {1}"));
    expect(placeholders("Header {1:id}")).not.toEqual(placeholders("Header"));
  });
});

const de = resources.de as Record<string, Catalog>;
const en = resources.en as Record<string, Catalog>;
const namespaces = [...new Set([...Object.keys(de), ...Object.keys(en)])].sort();

describe("catalogs", () => {
  it("knows the same namespaces in German and English", () => {
    expect(Object.keys(de).sort()).toEqual(Object.keys(en).sort());
  });

  it("knows at least the common namespace", () => {
    expect(namespaces).toContain("common");
  });

  describe.each(namespaces)("namespace %s", (ns) => {
    const flatDe = flatten(de[ns] ?? {});
    const flatEn = flatten(en[ns] ?? {});

    it("has the same keys in de and en", () => {
      const missingInEn = [...flatDe.keys()].filter((k) => !flatEn.has(k));
      const missingInDe = [...flatEn.keys()].filter((k) => !flatDe.has(k));
      const problems: string[] = [];
      if (missingInEn.length > 0) problems.push(`missing in en: ${missingInEn.join(", ")}`);
      if (missingInDe.length > 0) problems.push(`missing in de: ${missingInDe.join(", ")}`);
      expect(problems, `namespace ${ns}: ${problems.join("; ")}`).toEqual([]);
    });

    for (const [lang, flat] of [
      ["de", flatDe],
      ["en", flatEn],
    ] as const) {
      it(`has only non-empty strings in ${lang}`, () => {
        const bad = [...flat].filter(([, v]) => typeof v !== "string" || v.trim().length === 0).map(([k]) => k);
        expect(bad, `namespace ${ns} (${lang}): not a non-empty string: ${bad.join(", ")}`).toEqual([]);
      });

      it(`has a _other for every _one and the reverse in ${lang}`, () => {
        const problems: string[] = [];
        for (const key of flat.keys()) {
          if (key.endsWith("_one") && !flat.has(`${key.slice(0, -"_one".length)}_other`)) problems.push(`${key} has no _other`);
          if (key.endsWith("_other") && !flat.has(`${key.slice(0, -"_other".length)}_one`)) problems.push(`${key} has no _one`);
        }
        expect(problems, `namespace ${ns} (${lang}): ${problems.join("; ")}`).toEqual([]);
      });
    }

    it("has the same placeholders per key in de and en", () => {
      const problems: string[] = [];
      for (const [key, valueDe] of flatDe) {
        const valueEn = flatEn.get(key);
        if (typeof valueDe !== "string" || typeof valueEn !== "string") continue;
        const a = placeholders(valueDe);
        const b = placeholders(valueEn);
        if (a.join(",") !== b.join(",")) problems.push(`${key}: de ${a.join(", ") || "none"} vs en ${b.join(", ") || "none"}`);
      }
      expect(problems, `namespace ${ns}: placeholders differ: ${problems.join("; ")}`).toEqual([]);
    });
  });
});
