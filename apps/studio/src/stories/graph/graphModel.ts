import type { Edge, Node } from "@xyflow/react";
import type { NavTargetDto } from "../../api.data";
import type { GraphEdgeDto, GraphNodeDto, ThreadGraphDto } from "../../api.stories";
import { t } from "../../i18n";
import type { StoryNodeKind } from "../../ui/primitives";

/** The field of a level script's condition (`stories::threads::LEVEL_SCRIPT_FIELD`). */
const LEVEL_SCRIPT_FIELD = "var_conditions";

/**
 * What a node of the graph shows (`StoryNode`) and where it sits: `layoutKey` is the key of the side file (the step's key on a value with a
 * step, the node's id otherwise), `nav` where „Öffnen“ leads from a place.
 */
export type StoryNodeData = {
  kind: StoryNodeKind;
  title: string;
  subtitle?: string;
  notedOnly: boolean;
  loose: boolean;
  layoutKey: string;
  nav: NavTargetDto | null;
};

/** The React Flow node type of every node of a thread's graph. */
export const STORY_NODE_TYPE = "story";

function nodeData(node: GraphNodeDto): StoryNodeData {
  if (node.kind === "place") {
    const named = node.header_name !== null;
    return {
      kind: "place",
      title: named ? node.header_name! : node.label,
      ...(named && { subtitle: node.label }),
      notedOnly: false,
      loose: node.loose,
      layoutKey: node.id,
      nav: node.nav,
    };
  }
  if (node.step) {
    return {
      kind: node.step.kind,
      title: node.step.name,
      subtitle: t("stories:graphModel.value", { value: node.value }),
      notedOnly: node.noted_only,
      loose: false,
      layoutKey: node.step.key,
      nav: null,
    };
  }
  return { kind: "value", title: String(node.value), notedOnly: node.noted_only, loose: false, layoutKey: node.id, nav: null };
}

/**
 * Where an edge's reference is, as the flag detail says it: „Zeile 12 · SetVar“ in a script, „Bedingung 1“ in a level script,
 * „Auslöser 0“ and „Objekt 3“ in an event file; a script reference without a line is its command.
 */
export function edgeLabel(edge: GraphEdgeDto, graph: ThreadGraphDto): string {
  const placeId = edge.from.startsWith("p:") ? edge.from : edge.to;
  const place = graph.nodes.find((n) => n.id === placeId);
  const slot = edge.slot ?? 0;
  if (place?.kind === "place" && place.source.kind === "Event") {
    return edge.field === "var" ? t("stories:graphModel.trigger", { slot }) : t("stories:graphModel.object", { slot });
  }
  if (edge.field === LEVEL_SCRIPT_FIELD) return t("stories:graphModel.condition", { number: slot + 1 });
  return edge.slot === null ? edge.field : t("stories:graphModel.line", { line: edge.slot, field: edge.field });
}

/**
 * What a screen reader says for an edge (React Flow would say „Edge from p:script:0101 to v:1“): the place by its header (else its source)
 * sets or checks the value, and where („Zweiblattdorf setzt Wert 1: Zeile 12 · SetVar“).
 */
export function edgeAria(edge: GraphEdgeDto, graph: ThreadGraphDto, where: string): string {
  const sets = edge.from.startsWith("p:");
  const place = graph.nodes.find((n) => n.id === (sets ? edge.from : edge.to));
  const name = place?.kind === "place" ? (place.header_name ?? place.label) : (sets ? edge.from : edge.to);
  return t(sets ? "stories:graphModel.edgeSets" : "stories:graphModel.edgeChecks", { place: name, value: edge.value, where });
}

/**
 * A thread's graph as React Flow nodes and edges: one `story` node per graph node (all at the origin; `layoutGraph` places them) and one
 * edge per reference, labelled where it is; an edge at a value only noted is dashed (class `noted`).
 */
export function toFlow(graph: ThreadGraphDto): { nodes: Node<StoryNodeData>[]; edges: Edge[] } {
  const nodes = graph.nodes.map(
    (node): Node<StoryNodeData> => ({ id: node.id, type: STORY_NODE_TYPE, position: { x: 0, y: 0 }, data: nodeData(node) }),
  );
  const noted = new Set(nodes.filter((n) => n.data.notedOnly).map((n) => n.id));
  const edges = graph.edges.map((edge, i): Edge => {
    const label = edgeLabel(edge, graph);
    return {
      id: `e${i}:${edge.from}>${edge.to}`,
      source: edge.from,
      target: edge.to,
      label,
      ariaLabel: edgeAria(edge, graph, label),
      className: [edge.from.startsWith("p:") ? "sets" : "checks", noted.has(edge.from) || noted.has(edge.to) ? "noted" : ""].filter(Boolean).join(" "),
      data: { access: edge.access, value: edge.value },
    };
  });
  return { nodes, edges };
}
