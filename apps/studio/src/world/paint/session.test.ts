import { describe, expect, it } from "vitest";
import type { WorldEditRequest, WorldReport } from "../../api.world";
import { createStrokeQueue } from "./session";

const req = (map: number): WorldEditRequest => ({ map, changes: [{ cells: [[0, 0]], tile: "0001" }] });
const report = (map: number): WorldReport => ({ map, tileset: 6, cells: 1, change: map, bake: null });

/** A send whose answers the test releases by hand. */
function manual() {
  const sent: WorldEditRequest[] = [];
  const answers: { ok: (r: WorldReport) => void; fail: (e: unknown) => void }[] = [];
  const send = (r: WorldEditRequest) =>
    new Promise<WorldReport>((ok, fail) => {
      sent.push(r);
      answers.push({ ok, fail });
    });
  return { sent, answers, send };
}

const tick = () => new Promise((r) => setTimeout(r, 0));

describe("stroke queue", () => {
  it("the_queue_sends_strokes_one_after_another", async () => {
    const m = manual();
    const counts: number[] = [];
    const q = createStrokeQueue(m.send, (n) => counts.push(n));
    const first = q.push(req(1));
    const second = q.push(req(2));
    expect(q.pending()).toBe(2);
    await tick();
    expect(m.sent.map((r) => r.map)).toEqual([1]);
    m.answers[0].ok(report(1));
    await expect(first).resolves.toEqual(report(1));
    expect(q.pending()).toBe(1);
    await tick();
    expect(m.sent.map((r) => r.map)).toEqual([1, 2]);
    m.answers[1].ok(report(2));
    await expect(second).resolves.toEqual(report(2));
    expect(q.pending()).toBe(0);
    expect(counts).toEqual([1, 2, 1, 0]);
  });

  it("a_refused_stroke_does_not_block_the_next", async () => {
    const m = manual();
    const q = createStrokeQueue(m.send, () => {});
    const first = q.push(req(1));
    const second = q.push(req(2));
    await tick();
    m.answers[0].fail(new Error("Zelle (32, 4) liegt außerhalb der Karte (0 bis 31)."));
    await expect(first).rejects.toThrow("außerhalb");
    await tick();
    expect(m.sent).toHaveLength(2);
    m.answers[1].ok(report(2));
    await expect(second).resolves.toEqual(report(2));
    expect(q.pending()).toBe(0);
    // A send that throws synchronously is refused like an answer.
    const q2 = createStrokeQueue(() => {
      throw new Error("kaputt");
    }, () => {});
    await expect(q2.push(req(3))).rejects.toThrow("kaputt");
    expect(q2.pending()).toBe(0);
  });
});
