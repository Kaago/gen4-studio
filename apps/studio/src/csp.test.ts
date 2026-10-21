import { describe, expect, it } from "vitest";
import conf from "../src-tauri/tauri.conf.json";

/** Splits a CSP string into directive name -> sources. */
function directives(csp: string): Map<string, string[]> {
  const map = new Map<string, string[]>();
  for (const part of csp.split(";")) {
    const [name, ...sources] = part.trim().split(/\s+/);
    if (name) map.set(name, sources);
  }
  return map;
}

// three's GLTFLoader turns every embedded texture into a blob: URL and loads it through
// fetch() (connect-src) or an Image element (img-src), so both must allow blob:.
describe.each([
  ["csp", conf.app.security.csp],
  ["devCsp", conf.app.security.devCsp],
])("%s", (_name, csp) => {
  it("allows blob: texture URLs for fetch and images", () => {
    const d = directives(csp);
    expect(d.get("connect-src")).toContain("blob:");
    expect(d.get("img-src")).toContain("blob:");
  });

  it("keeps connect-src narrow", () => {
    const sources = directives(csp).get("connect-src") ?? [];
    expect(sources).not.toContain("*");
    expect(sources).not.toContain("https:");
    expect(sources).not.toContain("http:");
  });
});
