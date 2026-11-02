import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import { layoutGraph } from "./layout";
import { toFlow } from "./graphModel";
import { sampleThreadGraph } from "./sampleGraph";

const chain = (): { nodes: Node[]; edges: Edge[] } => ({
  nodes: [
    { id: "p:script:0001", position: { x: 0, y: 0 }, data: { layoutKey: "p:script:0001" } },
    { id: "v:1", position: { x: 0, y: 0 }, data: { layoutKey: "s1" } },
    { id: "p:event:0002", position: { x: 0, y: 0 }, data: { layoutKey: "p:event:0002" } },
  ],
  edges: [
    { id: "a", source: "p:script:0001", target: "v:1" },
    { id: "b", source: "v:1", target: "p:event:0002" },
  ],
});

describe("layoutGraph", () => {
  it("fixed_positions_win_over_dagre", () => {
    const { nodes, edges } = chain();
    // The side file keeps a step's node by the step's key.
    const out = layoutGraph(nodes, edges, { nodes: { s1: { x: 500, y: -40.5 } } });
    const at = (id: string) => out.find((n) => n.id === id)!.position;
    expect(at("v:1")).toEqual({ x: 500, y: -40.5 });
    expect(Number.isFinite(at("p:script:0001").x)).toBe(true);
    expect(Number.isFinite(at("p:event:0002").y)).toBe(true);
    // The input is not changed.
    expect(nodes[1].position).toEqual({ x: 0, y: 0 });
  });

  it("dagre_puts_setters_left_of_values_and_readers_right", () => {
    const { nodes, edges } = chain();
    const out = layoutGraph(nodes, edges, null);
    const x = (id: string) => out.find((n) => n.id === id)!.position.x;
    expect(x("p:script:0001")).toBeLessThan(x("v:1"));
    expect(x("v:1")).toBeLessThan(x("p:event:0002"));
    // The ranks are 64 apart beyond the node width: no two ranks overlap.
    expect(x("v:1") - x("p:script:0001")).toBeGreaterThanOrEqual(64);

    // The sample: every value right of the script that sets it, the level script right of the value it checks; no two nodes on one spot.
    const flow = toFlow(sampleThreadGraph);
    const laid = layoutGraph(flow.nodes, flow.edges, null);
    const pos = (id: string) => laid.find((n) => n.id === id)!.position;
    expect(pos("p:script:0101").x).toBeLessThan(pos("v:1").x);
    expect(pos("v:2").x).toBeLessThan(pos("p:script:0005").x);
    expect(new Set(laid.map((n) => `${n.position.x},${n.position.y}`)).size).toBe(laid.length);
    // A labelled edge gets room for its label between the ranks: „Zeile 12 · SetVar“ does not lie on a node.
    const gap = pos("v:1").x - (pos("p:script:0101").x + 192);
    expect(gap).toBeGreaterThanOrEqual("Zeile 12 · SetVar".length * 7);
  });
});
