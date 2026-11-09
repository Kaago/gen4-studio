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

const PLACEHOLDER = /\{\{(\w+)\}\}/g;

function placeholders(value: string): string[] {
  return [...new Set([...value.matchAll(PLACEHOLDER)].map((m) => m[1]))].sort();
}

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
        if (a.join(",") !== b.join(",")) problems.push(`${key}: de {{${a.join("}}, {{")}}} vs en {{${b.join("}}, {{")}}}`);
      }
      expect(problems, `namespace ${ns}: placeholders differ: ${problems.join("; ")}`).toEqual([]);
    });
  });
});
