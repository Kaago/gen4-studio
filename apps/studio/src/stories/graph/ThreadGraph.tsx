import "@xyflow/react/dist/base.css";
import { applyNodeChanges, Handle, MarkerType, Position, ReactFlow } from "@xyflow/react";
import type { AriaLabelConfig, Edge, Node, NodeChange, NodeProps } from "@xyflow/react";
import { createContext, useCallback, useContext, useMemo, useRef, useState } from "react";
import type { NavTargetDto } from "../../api.data";
import type { Layout, ThreadGraphDto } from "../../api.stories";
import { Button, StoryNode, storyKindWord } from "../../ui/primitives";
import { STORY_NODE_TYPE, toFlow } from "./graphModel";
import type { StoryNodeData } from "./graphModel";
import { layoutGraph, layoutOf } from "./layout";
import styles from "./ThreadGraph.module.css";

export type ThreadGraphProps = {
  graph: ThreadGraphDto;
  /** The id of the selected node (`v:3`, `p:script:0101`), or none. */
  selected: string | null;
  onSelect: (id: string | null) => void;
  /** „Öffnen“ in a selected place: where its reference is (the script line, the event, the header of a level script). */
  onOpenPlace: (nav: NavTargetDto) => void;
  /** Given (a story's graph, 3d.7): the nodes move, and the end of each drag (and each arrow key's move) hands over the side file with every
   * node where it lies now.
   * Without it nothing moves (a thread without a story). */
  onLayout?: (layout: Layout) => void;
};

type StoryFlowNode = Node<StoryNodeData>;

/**
 * The smallest zoom the graph is fitted at: the node's text (12 and 13 px) still reads at 7 to 8 px (twice that on a Retina screen), and a
 * story's graph of six ranks (gate 3d.6, „Heimkehr“, fitted at 0.6) still fits the detail whole. A larger graph (HeartGold's
 * `VAR_UNOWN_REPORT_LEVEL`, 21 nodes, was fitted at about 0.26) shows its middle at this zoom; the rest is panned to (drag) or zoomed out
 * to (pinch, down to 0.2).
 */
export const FIT_MIN_ZOOM = 0.6;

/** React Flow's own words for screen readers, in German; nothing removes a node, only a story's graph moves them (drag or arrow keys). */
const ariaLabels: Partial<AriaLabelConfig> = {
  "node.a11yDescription.default": "Enter oder Leertaste wählt einen Knoten.",
  "node.a11yDescription.keyboardDisabled": "Enter oder Leertaste wählt einen Knoten.",
  "edge.a11yDescription.default": "Enter oder Leertaste wählt eine Kante.",
  "handle.ariaLabel": "Anschluss",
};

const DIRECTIONS: Readonly<Record<string, string>> = { up: "oben", down: "unten", left: "links", right: "rechts" };

/** What a node says to a screen reader in a story's graph: its nodes move, by drag or by the arrow keys on the selected one. */
const MOVABLE = "Enter oder Leertaste wählt einen Knoten; die Pfeiltasten verschieben den gewählten, mit Umschalt weiter.";

/** A story's graph (React Flow shows `keyboardDisabled` while the keys work, so both say it); each key's move is announced. */
const movableLabels: Partial<AriaLabelConfig> = {
  ...ariaLabels,
  "node.a11yDescription.default": MOVABLE,
  "node.a11yDescription.keyboardDisabled": MOVABLE,
  "node.a11yDescription.ariaLiveMessage": ({ direction, x, y }) => `Knoten nach ${DIRECTIONS[direction] ?? direction} verschoben, jetzt bei x ${x}, y ${y}`,
};

/** `nodes` with `selected` as the only selected one; a node whose selection stays is the same object (it keeps its measured size). */
function withSelection(nodes: StoryFlowNode[], selected: string | null): StoryFlowNode[] {
  return nodes.map((node) => ((node.selected ?? false) === (node.id === selected) ? node : { ...node, selected: node.id === selected }));
}

const OpenPlace = createContext<(nav: NavTargetDto) => void>(() => {});

/** A node of the graph: the `StoryNode` between its two anchors (in left, out right), „Öffnen“ in the head of a selected place. */
function StoryFlowNodeView({ id, data, selected }: NodeProps<StoryFlowNode>) {
  const open = useContext(OpenPlace);
  return (
    <div className={styles.flowNode}>
      <Handle type="target" position={Position.Left} isConnectable={false} className={styles.handle} />
      <StoryNode
        kind={data.kind}
        title={data.title}
        subtitle={data.subtitle}
        selected={selected}
        notedOnly={data.notedOnly}
        loose={data.loose}
        action={
          selected && data.nav !== null ? (
            <span data-open-place="" className="nodrag nopan">
              <Button kind="secondary" size="s" icon="link" onClick={() => open(data.nav!)}>
                Öffnen
              </Button>
            </span>
          ) : undefined
        }
        dataAttrs={{ "data-node-id": id }}
      />
      <Handle type="source" position={Position.Right} isConnectable={false} className={styles.handle} />
    </div>
  );
}

const nodeTypes = { [STORY_NODE_TYPE]: StoryFlowNodeView };

/**
 * The graph of a thread or a story (3d.6): the places that set a value left of it, the places that check it right (dagre, or the story's
 * side file), one `StoryNode` per node, the edges labelled where the reference is. A click selects a node; a selected place opens with
 * „Öffnen“. With `onLayout` (a story, 3d.7) the nodes are dragged or moved with the arrow keys and each drag's end and each key's move hands the
 * side file over; without it nothing moves. Fits itself into view, never below `FIT_MIN_ZOOM` (a larger graph is panned); no minimap; React Flow's attribution stays (bottom right). The default export is for `React.lazy`: React
 * Flow and dagre stay in this chunk.
 */
export default function ThreadGraph({ graph, selected, onSelect, onOpenPlace, onLayout }: ThreadGraphProps) {
  const flow = useMemo(() => toFlow(graph), [graph]);
  const placed = useMemo(
    () =>
      (layoutGraph(flow.nodes, flow.edges, graph.layout) as StoryFlowNode[]).map((node) => ({
        ...node,
        ariaLabel: `${storyKindWord(node.data.kind)}: ${node.data.title}${node.data.subtitle ? `, ${node.data.subtitle}` : ""}`,
      })),
    [flow, graph.layout],
  );
  // React Flow keeps the measured size on the node objects it is given: they live in state and take every change it reports (sizes
  // included), so a selection replaces only the nodes whose selection changes and no node has to be measured (and hidden) again.
  const [nodes, setNodes] = useState(() => withSelection(placed, selected));
  const [shown, setShown] = useState({ placed, selected });
  if (shown.placed !== placed || shown.selected !== selected) {
    setShown({ placed, selected });
    setNodes(shown.placed !== placed ? withSelection(placed, selected) : (current) => withSelection(current, selected));
  }
  const edges = useMemo(
    (): Edge[] =>
      flow.edges.map((edge) => ({
        ...edge,
        selectable: false,
        focusable: false,
        markerEnd: { type: MarkerType.ArrowClosed, color: "var(--g4-linie-stark)", width: 16, height: 16 },
      })),
    [flow],
  );
  // A drag runs from its start to its end: the moves React Flow reports meanwhile are the drag's, and so is the last one (its end, or the
  // way back of a drag Escape broke off), which also ends it; `onNodeDragStop` saves a drag.
  const dragging = useRef(false);
  // Selection goes through the owner: a click or Enter on a node selects it, a click on the pane or Escape clears it. Every other
  // change (the sizes React Flow measures, the positions) is kept on the nodes. A finished move outside a drag is an arrow key's on the
  // selected node: it is saved at once, like a drag's end.
  const onNodesChange = useCallback(
    (changes: NodeChange<StoryFlowNode>[]) => {
      const kept = changes.filter((c) => c.type !== "select");
      if (kept.length > 0) setNodes((current) => applyNodeChanges(kept, current));
      const moved: StoryFlowNode[] = [];
      for (const c of kept) {
        if (c.type !== "position" || c.dragging !== false || c.position === undefined) continue;
        const node = nodes.find((n) => n.id === c.id);
        if (node) moved.push({ ...node, position: c.position });
      }
      if (moved.length > 0) {
        if (dragging.current) dragging.current = false;
        else onLayout?.(layoutOf(nodes, moved, graph.layout));
      }
      const picked = changes.find((c) => c.type === "select" && c.selected);
      if (picked && picked.type === "select") onSelect(picked.id);
      else if (changes.some((c) => c.type === "select" && !c.selected && c.id === selected)) onSelect(null);
    },
    [onSelect, selected, onLayout, nodes, graph.layout],
  );
  const onNodeDragStart = useCallback(() => {
    dragging.current = true;
  }, []);
  // The end of a drag: the nodes as they lie (the dragged ones from React Flow, the moves of this drag already in `nodes`).
  const onNodeDragStop = useCallback(
    (_event: unknown, _node: StoryFlowNode, moved: StoryFlowNode[]) => {
      dragging.current = false;
      onLayout?.(layoutOf(nodes, moved, graph.layout));
    },
    [onLayout, nodes, graph.layout],
  );
  return (
    <div className={styles.graph} data-thread-graph="" data-node-count={graph.nodes.length} data-edge-count={graph.edges.length}>
      <OpenPlace.Provider value={onOpenPlace}>
        <ReactFlow
          // A new graph lays out and fits anew.
          key={`${graph.var}:${graph.nodes.map((n) => n.id).join(",")}`}
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          onNodesChange={onNodesChange}
          onNodeDragStart={onLayout ? onNodeDragStart : undefined}
          onNodeDragStop={onLayout ? onNodeDragStop : undefined}
          nodesDraggable={onLayout !== undefined}
          nodesConnectable={false}
          edgesFocusable={false}
          deleteKeyCode={null}
          selectionKeyCode={null}
          multiSelectionKeyCode={null}
          zoomOnScroll={false}
          zoomOnDoubleClick={false}
          preventScrolling={false}
          minZoom={0.2}
          maxZoom={2}
          fitView
          fitViewOptions={{ padding: 0.12, minZoom: FIT_MIN_ZOOM, maxZoom: 1 }}
          ariaLabelConfig={onLayout ? movableLabels : ariaLabels}
        />
      </OpenPlace.Provider>
    </div>
  );
}
