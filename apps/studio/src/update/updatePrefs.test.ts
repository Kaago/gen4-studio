import { describe, expect, it, vi } from "vitest";
import { CHECK_INTERVAL_MS, DEFAULT_UPDATE_PREFS, UPDATE_PREFS_KEY, isCheckDue, readUpdatePrefs, writeUpdatePrefs } from "./updatePrefs";
import type { UpdatePrefs } from "./updatePrefs";

/** A `Storage` kept in a map; `getItem` and `setItem` may be made to throw. */
function storage(initial: Record<string, string> = {}, opts: { getThrows?: boolean; setThrows?: boolean } = {}): Storage & { map: Map<string, string> } {
  const map = new Map(Object.entries(initial));
  return {
    map,
    get length() {
      return map.size;
    },
    clear: () => map.clear(),
    getItem: (k: string) => {
      if (opts.getThrows) throw new Error("blocked");
      return map.get(k) ?? null;
    },
    key: (i: number) => [...map.keys()][i] ?? null,
    removeItem: (k: string) => void map.delete(k),
    setItem: (k: string, v: string) => {
      if (opts.setThrows) throw new Error("blocked");
      map.set(k, v);
    },
  };
}

const NOW = Date.parse("2026-11-12T12:00:00Z");
const HOUR = 3_600_000;
const iso = (ms: number) => new Date(ms).toISOString();
const auto = (lastCheck: string | null): UpdatePrefs => ({ consent: "auto", lastCheck, skipped: null });

describe("update preferences", () => {
  it("has_the_documented_constants", () => {
    expect(UPDATE_PREFS_KEY).toBe("g4.update.v1");
    expect(CHECK_INTERVAL_MS).toBe(86_400_000);
    expect(DEFAULT_UPDATE_PREFS).toEqual({ consent: "ask", lastCheck: null, skipped: null });
  });

  it("reads_the_defaults_without_an_entry", () => {
    expect(readUpdatePrefs(storage())).toEqual(DEFAULT_UPDATE_PREFS);
  });

  it("reads_the_defaults_from_broken_json", () => {
    expect(readUpdatePrefs(storage({ [UPDATE_PREFS_KEY]: "{kaputt" }))).toEqual(DEFAULT_UPDATE_PREFS);
  });

  it("reads_the_defaults_from_a_value_that_is_not_an_object", () => {
    for (const raw of ["null", "42", '"auto"', "[]", "true"]) {
      expect(readUpdatePrefs(storage({ [UPDATE_PREFS_KEY]: raw }))).toEqual(DEFAULT_UPDATE_PREFS);
    }
  });

  it("reads_the_defaults_from_invalid_fields", () => {
    const raw = JSON.stringify({ consent: "vielleicht", lastCheck: 5, skipped: ["x"] });
    expect(readUpdatePrefs(storage({ [UPDATE_PREFS_KEY]: raw }))).toEqual(DEFAULT_UPDATE_PREFS);
  });

  it("reads_the_defaults_when_the_storage_throws", () => {
    expect(readUpdatePrefs(storage({ [UPDATE_PREFS_KEY]: '{"consent":"auto"}' }, { getThrows: true }))).toEqual(DEFAULT_UPDATE_PREFS);
  });

  it("keeps_each_valid_field_on_its_own", () => {
    expect(readUpdatePrefs(storage({ [UPDATE_PREFS_KEY]: '{"consent":"auto"}' }))).toEqual({ consent: "auto", lastCheck: null, skipped: null });
    expect(readUpdatePrefs(storage({ [UPDATE_PREFS_KEY]: '{"consent":"off","lastCheck":"kein Datum","skipped":1}' }))).toEqual({
      consent: "off",
      lastCheck: "kein Datum",
      skipped: null,
    });
    expect(readUpdatePrefs(storage({ [UPDATE_PREFS_KEY]: '{"consent":"x","skipped":"1.0.2"}' }))).toEqual({ consent: "ask", lastCheck: null, skipped: "1.0.2" });
  });

  it("round_trips_through_write_and_read", () => {
    const s = storage();
    const prefs: UpdatePrefs = { consent: "auto", lastCheck: iso(NOW), skipped: "1.0.2" };
    writeUpdatePrefs(prefs, s);
    expect(s.map.has(UPDATE_PREFS_KEY)).toBe(true);
    expect(readUpdatePrefs(s)).toEqual(prefs);
  });

  it("write_does_not_throw_when_the_storage_does", () => {
    expect(() => writeUpdatePrefs(auto(null), storage({}, { setThrows: true }))).not.toThrow();
  });

  it("uses_the_local_storage_by_default", () => {
    const s = storage();
    vi.stubGlobal("localStorage", s);
    try {
      writeUpdatePrefs({ consent: "off", lastCheck: null, skipped: null });
      expect(s.map.get(UPDATE_PREFS_KEY)).toBe('{"consent":"off","lastCheck":null,"skipped":null}');
      expect(readUpdatePrefs()).toEqual({ consent: "off", lastCheck: null, skipped: null });
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it("is_quiet_when_there_is_no_local_storage_at_all", () => {
    vi.stubGlobal("localStorage", undefined);
    try {
      expect(() => writeUpdatePrefs(auto(null))).not.toThrow();
      expect(readUpdatePrefs()).toEqual(DEFAULT_UPDATE_PREFS);
    } finally {
      vi.unstubAllGlobals();
    }
  });
});

describe("isCheckDue", () => {
  it("is_false_for_ask_and_off", () => {
    expect(isCheckDue({ consent: "ask", lastCheck: null, skipped: null }, NOW)).toBe(false);
    expect(isCheckDue({ consent: "off", lastCheck: null, skipped: null }, NOW)).toBe(false);
    expect(isCheckDue({ consent: "ask", lastCheck: iso(NOW - 48 * HOUR), skipped: null }, NOW)).toBe(false);
    expect(isCheckDue({ consent: "off", lastCheck: iso(NOW - 48 * HOUR), skipped: null }, NOW)).toBe(false);
  });

  it("is_true_for_auto_that_never_checked", () => {
    expect(isCheckDue(auto(null), NOW)).toBe(true);
  });

  it("is_false_23_hours_after_the_last_check", () => {
    expect(isCheckDue(auto(iso(NOW - 23 * HOUR)), NOW)).toBe(false);
  });

  it("is_true_exactly_24_hours_after_the_last_check", () => {
    expect(isCheckDue(auto(iso(NOW - 24 * HOUR)), NOW)).toBe(true);
  });

  it("is_true_when_the_last_check_lies_in_the_future", () => {
    expect(isCheckDue(auto(iso(NOW + HOUR)), NOW)).toBe(true);
  });

  it("is_true_when_the_last_check_is_no_date", () => {
    expect(isCheckDue(auto("kein Datum"), NOW)).toBe(true);
  });
});
