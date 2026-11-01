import "@xyflow/react/dist/base.css";
import { applyNodeChanges, Handle, MarkerType, Position, ReactFlow } from "@xyflow/react";
import type { AriaLabelConfig, Edge, Node, NodeChange, NodeProps } from "@xyflow/react";
import { createContext, useCallback, useContext, useMemo, useState } from "react";
import type { NavTargetDto } from "../../api.data";
import type { ThreadGraphDto } from "../../api.stories";
import { Button, StoryNode, storyKindWord } from "../../ui/primitives";
import { STORY_NODE_TYPE, toFlow } from "./graphModel";
import type { StoryNodeData } from "./graphModel";
import { layoutGraph } from "./layout";
import styles from "./ThreadGraph.module.css";

export type ThreadGraphProps = {
  graph: ThreadGraphDto;
  /** The id of the selected node (`v:3`, `p:script:0101`), or none. */
  selected: string | null;
  onSelect: (id: string | null) => void;
  /** „Öffnen“ in a selected place: where its reference is (the script line, the event, the header of a level script). */
  onOpenPlace: (nav: NavTargetDto) => void;
};

type StoryFlowNode = Node<StoryNodeData>;

/** React Flow's own words for screen readers, in German; nothing in 3d.6 moves or removes a node. */
const ariaLabels: Partial<AriaLabelConfig> = {
  "node.a11yDescription.default": "Enter oder Leertaste wählt einen Knoten.",
  "node.a11yDescription.keyboardDisabled": "Enter oder Leertaste wählt einen Knoten.",
  "edge.a11yDescription.default": "Enter oder Leertaste wählt eine Kante.",
  "handle.ariaLabel": "Anschluss",
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
 * The graph of a thread or a story (3d.6, read only): the places that set a value left of it, the places that check it right (dagre,
 * or the story's side file), one `StoryNode` per node, the edges labelled where the reference is. A click selects a node; a selected
 * place opens with „Öffnen“. Fits itself into view; no minimap, nothing to drag (3d.7 writes the layout); React Flow's attribution
 * stays (bottom right). The default export is for `React.lazy`: React Flow and dagre stay in this chunk.
 */
export default function ThreadGraph({ graph, selected, onSelect, onOpenPlace }: ThreadGraphProps) {
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
  // Selection goes through the owner: a click or Enter on a node selects it, a click on the pane or Escape clears it. Every other
  // change (the sizes React Flow measures) is kept on the nodes.
  const onNodesChange = useCallback(
    (changes: NodeChange<StoryFlowNode>[]) => {
      const kept = changes.filter((c) => c.type !== "select");
      if (kept.length > 0) setNodes((current) => applyNodeChanges(kept, current));
      const picked = changes.find((c) => c.type === "select" && c.selected);
      if (picked && picked.type === "select") onSelect(picked.id);
      else if (changes.some((c) => c.type === "select" && !c.selected && c.id === selected)) onSelect(null);
    },
    [onSelect, selected],
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
          nodesDraggable={false}
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
          fitViewOptions={{ padding: 0.12, maxZoom: 1 }}
          ariaLabelConfig={ariaLabels}
        />
      </OpenPlace.Provider>
    </div>
  );
}
