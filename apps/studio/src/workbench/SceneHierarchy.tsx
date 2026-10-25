import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DataRowDto, EventsDto, HeaderSummary, MatrixDto } from "../api";
import { FindingMark } from "../checks/FindingMark";
import type { MarkSeverity } from "../checks/FindingMark";
import type { Depth, Tool } from "../ui/frame/shellState";
import { Badge, EmptyState, Icon, IconButton, Menu, Notice, PanelHead, TextField, TreeRow, useRovingList } from "../ui/primitives";
import type { MenuEntry } from "../ui/primitives";
import { lockedText } from "../world/events";
import type { EventRef } from "../world/events";
import type { MapBuildings } from "../world/scene/types";
import { ROOT_KEY, SCENE_OPEN, groupKey, sameRef, sceneRows } from "./sceneModel";
import type { SceneGroup, SceneInput, SceneRef, SceneRow } from "./sceneModel";
import { TOOLS, isEventTool } from "./tools";
import styles from "./SceneHierarchy.module.css";

export interface SceneHierarchyProps {
  /** The header the scene shows; null before one is shown. */
  header: HeaderSummary | null;
  headers: readonly HeaderSummary[];
  matrix: MatrixDto | null;
  events: EventsDto | null;
  buildings: readonly MapBuildings[];
  trainers?: readonly DataRowDto[] | null;
  depth: Depth;
  /** What the world model says a header is and how it is called (`WorkbenchWorld.identify`); without it the places tell. */
  identify?: SceneInput["identify"];
  selected: SceneRef | null;
  /** The worst error or warning the checks found per header (`headerMarks`): a mark at the root. */
  marks?: ReadonlyMap<number, MarkSeverity>;
  /** The events an error of the checks is about: a mark at their rows. */
  faultyEvents?: readonly EventRef[];
  /** Why the event file cannot be changed; null when it can. */
  locked: string | null;
  /** The events of the scene are being read. */
  loading: boolean;
  /** A row was chosen: the same selection as a click in the scene. */
  onSelect: (ref: SceneRef) => void;
  /** Enter or a double click on a door with a known target: show the target's scene. */
  onFollow: (header: number) => void;
  onOpenScript: (file: number) => void;
  /** An entry of „+“ picked a tool. */
  onTool: (tool: Tool) => void;
  /** Why the event tools are locked (the toolbar's reason); null when they are not. */
  toolsBlocked: string | null;
  /** „Raum hier anbinden…“: the wizard „Raum anlegen“ with an exit here. */
  onConnectRoom: () => void;
  onBack: () => void;
  onForward: () => void;
  canBack: boolean;
  canForward: boolean;
  /** Bumping `token` opens the group (and the root), focuses its row and scrolls to it; while the events load it waits for them. */
  focusGroup?: { group: SceneGroup; token: number };
}

/** The tools „+“ offers, in the toolbar's order. */
const ADDS: readonly Tool[] = ["figure", "door", "trigger", "sign", "building"];
const EVENT_GROUPS: ReadonlySet<SceneGroup> = new Set(["object", "warp", "coord", "bg"]);
const FOLLOW_HINT = "Doppelklick oder Enter folgt der Tür zum Ziel";

/**
 * „In dieser Szene“: what the shown area or room holds, as a tree (the rows come from `sceneRows`). A click selects like a click
 * in the scene, a script opens in its tab, a door with a known target is followed on Enter or a double click. The head goes back
 * and forward through the shown scenes and adds with „+“; a search filters the rows.
 */
export function SceneHierarchy(props: SceneHierarchyProps) {
  const { header, headers, matrix, events, buildings, trainers, depth, identify, selected, marks, faultyEvents, locked, loading } = props;
  const { onSelect, onFollow, onOpenScript, onTool, toolsBlocked, onConnectRoom, onBack, onForward, canBack, canForward, focusGroup } = props;
  const [open, setOpen] = useState<ReadonlySet<string>>(SCENE_OPEN);
  const [query, setQuery] = useState("");
  const rows = useMemo(
    () => (header === null ? [] : sceneRows({ header, headers, matrix, events, buildings, trainers, depth, identify }, open, query)),
    [header, headers, matrix, events, buildings, trainers, depth, identify, open, query],
  );
  const toggle = useCallback(
    (key: string) =>
      setOpen((prev) => {
        const next = new Set(prev);
        if (!next.delete(key)) next.add(key);
        return next;
      }),
    [],
  );

  const tree = useRef<HTMLDivElement>(null);
  useRovingList(tree);

  // A request to focus a group opens it and the root; the render after that focuses its row (a search that hides it is cleared first, events
  // that are still read are waited for).
  const pending = useRef<SceneGroup | null>(null);
  // The request opened a branch: its rows come with the next render, not this one. The open set as rendered, for the request's effect.
  const opening = useRef(false);
  const shownOpen = useRef(open);
  shownOpen.current = open;
  const token = focusGroup?.token;
  const wanted = focusGroup?.group;
  useEffect(() => {
    if (token === undefined || wanted === undefined) return;
    pending.current = wanted;
    const prev = shownOpen.current;
    if (prev.has(groupKey(wanted)) && prev.has(ROOT_KEY)) return;
    opening.current = true;
    setOpen((current) => new Set([...current, ROOT_KEY, groupKey(wanted)]));
  }, [token, wanted]);
  useEffect(() => {
    const group = pending.current;
    if (group === null) return;
    const row = tree.current?.querySelector<HTMLElement>(`[data-scene-group="${group}"]`);
    if (row) {
      pending.current = null;
      opening.current = false;
      // A row that just came has no tab index yet (the roving list gives it one a moment later): focusing makes it the tab stop.
      if (!row.hasAttribute("tabindex")) row.tabIndex = -1;
      row.focus();
      row.scrollIntoView?.({ block: "nearest" });
    } else if (query !== "") setQuery("");
    else if (opening.current) opening.current = false;
    // The scene has no such group (once its events are read): nothing to focus.
    else if (!(events === null && loading)) pending.current = null;
  });

  const faulty = (ref: SceneRef | undefined) =>
    ref?.kind === "event" && (faultyEvents ?? []).some((e) => e.file === ref.file && e.kind === ref.eventKind && e.index === ref.index);

  const adds: MenuEntry[] = [
    ...ADDS.map((id) => {
      const tool = TOOLS.find((t) => t.id === id)!;
      const blocked = isEventTool(id) ? toolsBlocked : null;
      return { label: blocked ? `${tool.label}, ${blocked}` : tool.label, icon: tool.icon, disabled: blocked !== null, onSelect: () => onTool(id) };
    }),
    "separator",
    { label: "Raum hier anbinden…", icon: "room", onSelect: onConnectRoom },
  ];

  const renderRow = (row: SceneRow) => {
    const isSelected = row.ref !== undefined && sameRef(row.ref, selected);
    const common = { depth: row.depth, label: row.label, tech: row.tech, subtitle: row.second, icon: row.icon, expanded: row.expanded, onToggle: row.expanded !== undefined ? () => toggle(row.key) : undefined };
    switch (row.kind) {
      case "header": {
        const mark = header === null ? undefined : marks?.get(header.id);
        return (
          <TreeRow
            key={row.key}
            {...common}
            selected={isSelected}
            onSelect={() => row.ref && onSelect(row.ref)}
            dataAttrs={{ "data-scene-group": "header", "data-header-id": header?.id }}
            trailing={mark ? <FindingMark severity={mark} /> : undefined}
          />
        );
      }
      case "group":
        return (
          <TreeRow
            key={row.key}
            {...common}
            ariaLabel={`${row.label}, ${row.count}`}
            selected={false}
            onSelect={() => toggle(row.key)}
            dataAttrs={{ "data-scene-group": row.group }}
            trailing={
              <>
                {locked !== null && row.group !== undefined && EVENT_GROUPS.has(row.group) && <Badge kind="locked" />}
                <span className={styles.count}>{row.count}</span>
              </>
            }
          />
        );
      case "script": {
        const file = row.ref?.kind === "script" ? row.ref.file : -1;
        return <TreeRow key={row.key} {...common} selected={false} onSelect={() => onOpenScript(file)} dataAttrs={{ "data-script-file": file }} />;
      }
      default: {
        const ref = row.ref!;
        const dataAttrs =
          ref.kind === "event" ? { "data-event": `${ref.eventKind}:${ref.index}` } : ref.kind === "building" ? { "data-building": `${ref.map}:${ref.index}` } : undefined;
        const item = (
          <TreeRow
            key={row.key}
            {...common}
            selected={isSelected}
            onSelect={() => onSelect(ref)}
            dataAttrs={dataAttrs}
            title={typeof row.dest === "number" ? FOLLOW_HINT : undefined}
            trailing={faulty(ref) ? <FindingMark severity="error" /> : undefined}
          />
        );
        if (typeof row.dest !== "number") return item;
        const dest = row.dest;
        return (
          <div
            key={row.key}
            role="presentation"
            className={styles.entry}
            onDoubleClick={() => onFollow(dest)}
            // Enter on a door follows it too (after the row selected it), as the double click does.
            onKeyDown={(e) => e.key === "Enter" && onFollow(dest)}
          >
            {item}
          </div>
        );
      }
    }
  };

  const eventCount = events === null ? 0 : events.objects.length + events.warps.length + events.coords.length + events.bg.length;
  const nothing = events !== null && eventCount === 0 && buildings.every((b) => b.models.length === 0);
  const waiting = events === null && loading;

  return (
    <div className={styles.panel}>
      <PanelHead
        title="In dieser Szene"
        actions={
          <>
            <IconButton label="Zurück" icon="back" shortcut="⌥⌘←" disabled={!canBack} onClick={onBack} />
            <IconButton label="Vor" icon="forward" shortcut="⌥⌘→" disabled={!canForward} onClick={onForward} />
            {header !== null && <Menu label="Hinzufügen" trigger={<Icon name="plus" />} items={adds} />}
          </>
        }
      />
      {header === null ? (
        <EmptyState what="Noch keine Szene gezeigt." why="Wähle oben ein Gebiet oder einen Raum, dann steht hier, was die Szene enthält." />
      ) : (
        <>
          <div className={styles.top}>
            <TextField type="search" label="In dieser Szene suchen" placeholder="Name oder Nummer suchen" value={query} onChange={setQuery} onCommit={setQuery} />
            {locked !== null && (
              <Notice kind="warning" title="Eventdatei gesperrt">
                {lockedText(locked)}
              </Notice>
            )}
          </div>
          <div ref={tree} role="tree" aria-label="In dieser Szene" className={styles.host}>
            {rows.map(renderRow)}
            {query.trim() !== "" && rows.length <= 1 && <p className={styles.note}>Nichts in dieser Szene passt zur Suche.</p>}
            {waiting && <p className={styles.note}>Lade …</p>}
            {nothing && query.trim() === "" && (
              <p className={styles.note}>Hier ist noch nichts. Mit „+“ oder den Werkzeugen setzt du eine Figur, eine Tür, einen Auslöser, ein Schild oder ein Gebäude.</p>
            )}
          </div>
        </>
      )}
    </div>
  );
}
