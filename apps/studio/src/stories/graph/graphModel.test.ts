import { describe, expect, it } from "vitest";
import type { ThreadGraphDto } from "../../api.stories";
import { edgeAria, edgeLabel, toFlow } from "./graphModel";
import { sampleThreadGraph } from "./sampleGraph";

const nodeOf = (graph: ThreadGraphDto, id: string) => toFlow(graph).nodes.find((n) => n.id === id)!;

describe("graphModel", () => {
  it("value_nodes_become_story_nodes_with_their_step", () => {
    const { nodes } = toFlow(sampleThreadGraph);
    expect(nodes).toHaveLength(sampleThreadGraph.nodes.length);
    expect(nodes.every((n) => n.type === "story")).toBe(true);

    // A step lies on its value: the node takes the step's kind and name, the value is the second line, the step's key places it.
    const dialog = nodeOf(sampleThreadGraph, "v:1");
    expect(dialog.data).toMatchObject({ kind: "dialog", title: "Rivalin trifft", subtitle: "Wert 1", notedOnly: false, loose: false, layoutKey: "s1", nav: null });
    // A value only the story names is noted only.
    expect(nodeOf(sampleThreadGraph, "v:3").data).toMatchObject({ kind: "reward", title: "Pokédex bekommen", notedOnly: true });
    // A value without a step is a value node with the number.
    expect(nodeOf(sampleThreadGraph, "v:4").data).toMatchObject({ kind: "value", title: "4", layoutKey: "v:4" });
    expect(nodeOf(sampleThreadGraph, "v:4").data.subtitle).toBeUndefined();

    // A place is named by its header where it has one, the source below; it carries where a click leads.
    const place = nodeOf(sampleThreadGraph, "p:script:0101");
    expect(place.data).toMatchObject({ kind: "place", title: "Zweiblattdorf", subtitle: "Scriptdatei 0101", layoutKey: "p:script:0101" });
    expect(place.data.nav).toEqual({ kind: "script", file: 101, line: 12 });
    expect(nodeOf(sampleThreadGraph, "p:script:0005").data).toMatchObject({ kind: "place", title: "Level-Script 0005" });
    expect(nodeOf(sampleThreadGraph, "p:script:0005").data.subtitle).toBeUndefined();
  });

  it("edges_carry_field_and_slot_labels", () => {
    const { edges } = toFlow(sampleThreadGraph);
    expect(edges).toHaveLength(sampleThreadGraph.edges.length);
    expect(new Set(edges.map((e) => e.id)).size).toBe(edges.length);
    const label = (from: string, to: string) => edges.find((e) => e.source === from && e.target === to)?.label;
    // Set in a script line, checked by a trigger of an event file and by a condition of a level script.
    expect(label("p:script:0101", "v:1")).toBe("Zeile 12 · SetVar");
    expect(label("v:1", "p:event:0394")).toBe("Auslöser 0");
    expect(label("v:2", "p:script:0005")).toBe("Bedingung 1");
    // An object of an event file, and a script field without a line.
    const objectEdge = { from: "v:1", to: "p:event:0394", value: 1, access: "read" as const, field: "script", slot: 3 };
    expect(edgeLabel(objectEdge, sampleThreadGraph)).toBe("Objekt 3");
    expect(edgeLabel({ ...objectEdge, to: "p:script:0101", field: "SetVar", slot: null }, sampleThreadGraph)).toBe("SetVar");
    // An edge at a value only noted is dashed; the others are not.
    const noted: ThreadGraphDto = { ...sampleThreadGraph, edges: [{ ...objectEdge, from: "v:3", to: "p:script:0101", field: "SetVar", slot: 1 }] };
    expect(toFlow(noted).edges[0].className).toContain("noted");
    expect(edges.every((e) => !(e.className ?? "").includes("noted"))).toBe(true);
  });

  it("edges_say_who_sets_or_checks_the_value_for_a_screen_reader", () => {
    const { edges } = toFlow(sampleThreadGraph);
    const aria = (from: string, to: string) => edges.find((e) => e.source === from && e.target === to)?.ariaLabel;
    // Not React Flow's „Edge from p:script:0101 to v:1“: the place by its header, the value, and where.
    expect(aria("p:script:0101", "v:1")).toBe("Zweiblattdorf setzt Wert 1: Zeile 12 · SetVar");
    expect(aria("v:1", "p:event:0394")).toBe("Route 201 prüft Wert 1: Auslöser 0");
    // A place without a header is named by its source.
    expect(aria("v:2", "p:script:0005")).toBe("Level-Script 0005 prüft Wert 2: Bedingung 1");
    expect(edges.every((e) => typeof e.ariaLabel === "string" && !/[pv]:/.test(e.ariaLabel))).toBe(true);
    const objectEdge = { from: "v:1", to: "p:event:0394", value: 1, access: "read" as const, field: "script", slot: 3 };
    expect(edgeAria(objectEdge, sampleThreadGraph, "Objekt 3")).toBe("Route 201 prüft Wert 1: Objekt 3");
  });

  it("a_loose_place_has_no_edges", () => {
    const { nodes, edges } = toFlow(sampleThreadGraph);
    const loose = nodes.find((n) => n.id === "p:script:0200")!;
    expect(loose.data.loose).toBe(true);
    expect(edges.some((e) => e.source === loose.id || e.target === loose.id)).toBe(false);
  });
});
