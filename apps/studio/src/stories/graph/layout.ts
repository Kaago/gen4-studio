import dagre from "@dagrejs/dagre";
import type { Edge, Node } from "@xyflow/react";
import type { Layout } from "../../api.stories";

/** The size dagre reserves for a node: the width of `StoryNode`, the height of its three lines. */
export const NODE_WIDTH = 192;
export const NODE_HEIGHT = 64;
/** The room of an edge's label (12 px text, „Zeile 12 · SetVar“): an estimate per character, the label's padding and height. */
const LABEL_CHAR_WIDTH = 7;
const LABEL_PADDING = 12;
const LABEL_HEIGHT = 18;

/** The key of a node in the side file: the step's key on a value with a step (`data.layoutKey`), the node's id otherwise. */
function layoutKey(node: Node): string {
  const key = (node.data as { layoutKey?: unknown }).layoutKey;
  return typeof key === "string" ? key : node.id;
}

/**
 * The nodes placed: left to right (dagre, rank direction LR, 24 between nodes of a rank, 64 between ranks), so the places that set a
 * value stand left of it and the places that check it right, with room for the edges' labels between the ranks; a position of the side file (`fixed`) wins over dagre's. Positions are
 * the top left corner, as React Flow takes them. The input is not changed.
 */
export function layoutGraph(nodes: Node[], edges: Edge[], fixed: Layout | null): Node[] {
  const g = new dagre.graphlib.Graph();
  g.setGraph({ rankdir: "LR", nodesep: 24, ranksep: 64 });
  g.setDefaultEdgeLabel(() => ({}));
  for (const node of nodes) g.setNode(node.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  for (const edge of edges) {
    if (!g.hasNode(edge.source) || !g.hasNode(edge.target)) continue;
    // A label takes room between the ranks (dagre places it in the middle of the edge), so it never lies on a node.
    const label = typeof edge.label === "string" ? edge.label : "";
    g.setEdge(edge.source, edge.target, label ? { width: label.length * LABEL_CHAR_WIDTH + LABEL_PADDING, height: LABEL_HEIGHT, labelpos: "c" } : {});
  }
  dagre.layout(g);
  return nodes.map((node) => {
    const kept = fixed?.nodes[layoutKey(node)];
    if (kept) return { ...node, position: { x: kept.x, y: kept.y } };
    const at = g.node(node.id);
    return { ...node, position: { x: at.x - NODE_WIDTH / 2, y: at.y - NODE_HEIGHT / 2 } };
  });
}
