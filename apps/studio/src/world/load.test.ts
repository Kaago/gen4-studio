import { describe, expect, it } from "vitest";
import { MODEL_CONCURRENCY, limitConcurrency, textureNote } from "./load";

describe("limitConcurrency", () => {
  it("never exceeds the cap and reports progress", async () => {
    let running = 0,
      peak = 0;
    const seen: number[] = [];
    const tasks = Array.from({ length: 10 }, (_, i) => async () => {
      running++;
      peak = Math.max(peak, running);
      await new Promise((r) => setTimeout(r, 5));
      running--;
      return i;
    });
    const results = await limitConcurrency(tasks, 3, (done) => seen.push(done));
    expect(peak).toBeLessThanOrEqual(3);
    expect(peak).toBe(3);
    expect(results.map((r) => (r.status === "fulfilled" ? r.value : -1)).sort((a, b) => a - b)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    expect(seen[seen.length - 1]).toBe(10);
    expect(seen).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
  });

  it("a rejected task does not stop the others", async () => {
    const results = await limitConcurrency(
      [
        async () => {
          throw new Error("x");
        },
        async () => 1,
      ],
      2,
    );
    expect(results[0].status).toBe("rejected");
    expect(results[1].status).toBe("fulfilled");
  });

  it("keeps result order, reports the total and handles an empty list", async () => {
    const totals = new Set<number>();
    const results = await limitConcurrency(
      [() => new Promise<string>((r) => setTimeout(() => r("slow"), 15)), async () => "fast"],
      2,
      (_, total) => totals.add(total),
    );
    expect(results.map((r) => (r.status === "fulfilled" ? r.value : null))).toEqual(["slow", "fast"]);
    expect([...totals]).toEqual([2]);
    expect(await limitConcurrency([], 4)).toEqual([]);
  });

  it("starts tasks in order and a synchronous throw is a rejection", async () => {
    const started: number[] = [];
    const tasks = [0, 1, 2, 3].map((i) => () => {
      started.push(i);
      if (i === 1) throw new Error("sync");
      return Promise.resolve(i);
    });
    const results = await limitConcurrency(tasks, 1);
    expect(started).toEqual([0, 1, 2, 3]);
    expect(results.map((r) => r.status)).toEqual(["fulfilled", "rejected", "fulfilled", "fulfilled"]);
  });

  it("uses a model concurrency of 4", () => {
    expect(MODEL_CONCURRENCY).toBe(4);
  });
});

describe("textureNote", () => {
  it("is null without issues", () => {
    expect(textureNote([])).toBeNull();
  });

  it("names the tileset once however many maps share it", () => {
    const issue = { tileset: 12, reason: "bad tileset" };
    expect(textureNote([issue, issue, issue])).toBe("Tileset 0012 nicht konvertierbar – Karten untexturiert");
  });

  it("lists distinct tilesets and unreadable area data", () => {
    expect(
      textureNote([
        { tileset: 3, reason: "a" },
        { tileset: null, reason: "b" },
        { tileset: 12, reason: "c" },
      ]),
    ).toBe("Tileset 0003 nicht konvertierbar, Area-Data nicht lesbar, Tileset 0012 nicht konvertierbar – Karten untexturiert");
  });
});
