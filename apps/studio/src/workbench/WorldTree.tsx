import { Fragment, memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { errorText, listScriptFiles, searchScripts } from "../api";
import type { HeaderSummary, ProjectInfo, ScriptFileDto, ScriptHitDto } from "../api";
import { FindingMark } from "../checks/FindingMark";
import type { MarkSeverity } from "../checks/FindingMark";
import type { Depth } from "../ui/frame/shellState";
import { Notice, TextField, TreeRow, useRovingList } from "../ui/primitives";
import type { IconName } from "../ui/primitives";
import { showsMatrix } from "./headers";
import { placeColor } from "./places";
import { ADOPT_HINT } from "./inspector/scriptNav";
import { RoomActions, WorldMapActions } from "./RoomActions";
import type { AreaChoice, RenameTarget } from "./RoomActions";
import { ShownMark } from "./ShownMark";
import { TERMS, id4, techLabel } from "./terms";
import { sortWorld } from "./worldSort";
import type { TreeSort } from "./worldSort";
import type { Area, Room, World, WorldMap } from "./worldModel";
import styles from "./WorldExplorer.module.css";

export interface WorldTreeProps {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  /** The world model; null while it is read (the tree then lists the headers plainly, by id). */
  world: World | null;
  /** False until the overview's first matrix listing arrived. */
  matricesLoaded: boolean;
  /** The first matrix listing failed (and nothing is loaded): the caption says the overview is not available instead of loading. */
  matricesFailed: boolean;
  selected: number | null;
  /** The matrix the scene shows: rows using it are marked. */
  shownMatrix: number | null;
  /** Bumped by every link to a header: the tree scrolls to the selection again, also when it was selected already. */
  revealToken: number;
  /** Tüfteln writes the identifiers (id, code) into every row and lists „Sonstiges“; Bauen speaks of the game only. */
  depth: Depth;
  onSelect: (h: HeaderSummary) => void;
  /** Whether rotom has taken the scripts over; without it the root node „Scripts“ only shows a hint. */
  scriptsAdopted: boolean;
  /** Opens a script file in the script panel (at a 1-based line). Must keep its identity (the rows are memoised). */
  onOpenScript: (file: number, line?: number) => void;
  /** The worst error or warning the checks found per header (`headerMarks`). None: no marks. */
  marks?: ReadonlyMap<number, MarkSeverity>;
  sort: TreeSort;
  /** The area keys chosen last, newest first (for the sort „Zuletzt gewählt“). */
  recent: readonly string[];
  /** A click chose an area, or a room of it. Must keep its identity. */
  onTouch: (areaKey: string) => void;
  onRename: (target: RenameTarget, name: string) => void;
  onOwner: (room: number, areaKey: string | null) => void;
}

/** The roots after the world maps; their keys cannot be area or world map keys. */
const MORE = "group:more";
const UNCONNECTED = "group:unconnected";
const OTHER = "group:other";

/** World maps start open, every other branch closed. */
const openByDefault = (key: string) => /^w\d+k?$/.test(key);

const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
const roomsText = (n: number) => plural(n, TERMS.room.label, TERMS.room.plural);
const areasText = (n: number) => plural(n, "Gebiet", "Gebiete");

/** The worst mark among headers: an error over a warning. */
function worstMark(marks: ReadonlyMap<number, MarkSeverity> | undefined, headers: readonly HeaderSummary[]): MarkSeverity | undefined {
  if (!marks) return undefined;
  let worst: MarkSeverity | undefined;
  for (const h of headers) {
    const mark = marks.get(h.id);
    if (mark === "error") return mark;
    if (mark) worst = mark;
  }
  return worst;
}

const codeMatches = (h: HeaderSummary, q: string) => h.internal_name.toLowerCase().includes(q) || id4(h.id).includes(q);
const textMatches = (q: string, ...texts: string[]) => texts.some((t) => t.toLowerCase().includes(q));

function filterRooms(rooms: readonly Room[], q: string): Room[] {
  return rooms.filter((r) => textMatches(q, r.name, r.placeName) || codeMatches(r.header, q));
}

/** The area whole when its name or one of its headers matches, else with the rooms that match; null when none does. */
function filterArea(area: Area, q: string): Area | null {
  if (textMatches(q, area.name) || area.headers.some((h) => codeMatches(h, q))) return area;
  const rooms = filterRooms(area.rooms, q);
  return rooms.length > 0 ? { ...area, rooms } : null;
}

const filterAreas = (areas: readonly Area[], q: string) => areas.map((a) => filterArea(a, q)).filter((a): a is Area => a !== null);

/**
 * The world with what matches the search: a world map by its name whole, areas by name or by the code or four-digit
 * number of one of their headers whole, rooms by name, place name, code or number. An empty query returns `world`.
 */
export function filterWorld(world: World, query: string): World {
  const q = query.trim().toLowerCase();
  if (q === "") return world;
  const worldMaps: WorldMap[] = [];
  for (const map of world.worldMaps) {
    if (textMatches(q, map.name)) {
      worldMaps.push(map);
      continue;
    }
    const areas = filterAreas(map.areas, q);
    if (areas.length > 0) worldMaps.push({ ...map, areas });
  }
  return {
    ...world,
    worldMaps,
    moreAreas: filterAreas(world.moreAreas, q),
    unconnected: filterRooms(world.unconnected, q),
    other: world.other.filter((h) => textMatches(q, h.location_name) || codeMatches(h, q)),
  };
}

/** Per header the branches that must be open for its row to show (Tüfteln lists the sections of an area as its children). */
function branchesOf(world: World, tinker: boolean): Map<number, string[]> {
  const paths = new Map<number, string[]>();
  const visit = (areas: readonly Area[], parent: string) => {
    for (const area of areas) {
      const sections = tinker && area.headers.length > 1;
      for (const h of area.headers) paths.set(h.id, sections ? [parent, area.key] : [parent]);
      for (const r of area.rooms) paths.set(r.header.id, [parent, area.key]);
    }
  };
  for (const map of world.worldMaps) visit(map.areas, map.key);
  visit(world.moreAreas, MORE);
  for (const r of world.unconnected) paths.set(r.header.id, [UNCONNECTED]);
  if (tinker) for (const h of world.other) paths.set(h.id, [OTHER]);
  return paths;
}

/** The hover text of a header row: Tüfteln names id, code and matrix; Bauen leaves it to the row's name. */
function techTitle(h: HeaderSummary, marked: boolean): string {
  return `${techLabel("header", h.id)} ${h.internal_name}, ${techLabel("matrix", h.matrix_id)}${marked ? ` (${TERMS.matrix.label} wird angezeigt)` : ""}`;
}

/**
 * A header without an area of the world: a section of an area (Tüfteln), a header of „Sonstiges“, or any header while
 * the world is read. A leaf: the scripts of the shown scene are in „In dieser Szene“. Memoised like every row of the tree.
 */
const HeaderLeaf = memo(function HeaderLeaf({
  header: h,
  depth,
  label,
  icon,
  tinker,
  selected,
  marked,
  mark,
  onSelect,
}: {
  header: HeaderSummary;
  depth: number;
  label: string;
  icon?: IconName;
  tinker: boolean;
  selected: boolean;
  marked: boolean;
  mark: MarkSeverity | undefined;
  onSelect: (h: HeaderSummary) => void;
}) {
  return (
    <TreeRow
      depth={depth}
      label={label}
      icon={icon}
      tech={tinker ? `${id4(h.id)} · ${h.internal_name}` : undefined}
      selected={selected}
      onSelect={() => onSelect(h)}
      title={tinker ? techTitle(h, marked) : undefined}
      dataAttrs={{ "data-header-id": h.id }}
      trailing={
        marked || mark ? (
          <>
            {mark && <FindingMark severity={mark} />}
            {marked && <ShownMark />}
          </>
        ) : undefined
      }
    />
  );
});

/** An area: a click selects its first header, the caret opens its rooms. */
const AreaRow = memo(function AreaRow({
  area,
  depth,
  tinker,
  selected,
  marked,
  mark,
  open,
  onPick,
  onToggle,
}: {
  area: Area;
  depth: number;
  tinker: boolean;
  selected: boolean;
  marked: boolean;
  mark: MarkSeverity | undefined;
  open: boolean;
  onPick: (h: HeaderSummary, areaKey: string | null) => void;
  onToggle: (key: string) => void;
}) {
  const first = area.headers[0];
  const count = area.rooms.length;
  return (
    <TreeRow
      depth={depth}
      icon="outside"
      label={area.name}
      tech={tinker ? `${id4(first.id)} · ${first.internal_name}` : undefined}
      ariaLabel={`${area.name}, ${roomsText(count)}`}
      title={tinker ? techTitle(first, marked) : `${area.name}, ${roomsText(count)}`}
      selected={selected}
      onSelect={() => onPick(first, area.key)}
      expanded={open}
      onToggle={() => onToggle(area.key)}
      dataAttrs={{ "data-area-key": area.key, "data-place-key": area.placeKey, "data-header-id": first.id }}
      trailing={
        <>
          {mark && <FindingMark severity={mark} />}
          {marked && <ShownMark />}
          <span className={styles.count}>{count}</span>
        </>
      }
    />
  );
});

/**
 * A room, named by the project file or the suggestion. A room of another place than its area's has the place label: a dot
 * in the place's colour and the place's name in grey (never colour alone). The selected room carries the menu of its actions.
 */
const RoomRow = memo(function RoomRow({
  room,
  areaKey,
  depth,
  labelled,
  tinker,
  selected,
  marked,
  mark,
  areas,
  onPick,
  onRename,
  onOwner,
}: {
  room: Room;
  areaKey: string | null;
  depth: number;
  /** The place label shows. */
  labelled: boolean;
  tinker: boolean;
  selected: boolean;
  marked: boolean;
  mark: MarkSeverity | undefined;
  areas: readonly AreaChoice[];
  onPick: (h: HeaderSummary, areaKey: string | null) => void;
  onRename: (target: RenameTarget, name: string) => void;
  onOwner: (room: number, areaKey: string | null) => void;
}) {
  const h = room.header;
  const tech = tinker ? `${id4(h.id)} · ${h.internal_name}` : undefined;
  const plain = [room.name, labelled ? room.placeName : null].filter((part) => part !== null).join(", ");
  return (
    <TreeRow
      depth={depth}
      icon="room"
      label={room.name}
      tech={tech}
      // The place label would run into the name: the row names itself.
      ariaLabel={labelled ? [room.name, tech, room.placeName].filter(Boolean).join(", ") : undefined}
      title={tinker ? techTitle(h, marked) : `${plain}${room.manual ? " (von Hand zugeordnet)" : ""}`}
      selected={selected}
      onSelect={() => onPick(h, areaKey)}
      dataAttrs={{ "data-header-id": h.id, "data-room": "" }}
      trailing={
        labelled || marked || mark || selected ? (
          <>
            {labelled && (
              <span className={styles.placeLabel} data-place-label="">
                <span data-place-dot="" className={styles.dot} style={{ background: placeColor(room.placeKey) }} /* dyn */ />
                {room.placeName}
              </span>
            )}
            {mark && <FindingMark severity={mark} />}
            {marked && <ShownMark />}
            {selected && <RoomActions room={room} areaKey={areaKey} areas={areas} onRename={onRename} onOwner={onOwner} />}
          </>
        ) : undefined
      }
    />
  );
});

const SEARCH_DELAY_MS = 300;
const SEARCH_LIMIT = 100;

/** "412 Byte" for a script source; the explorer's grey part says what a file is and which headers use it (Bauen: which places). */
function fileDetails(f: ScriptFileDto, depth: Depth): string {
  const tinker = depth === "tuefteln";
  const word = tinker ? TERMS.header.plural : f.headers.length === 1 ? TERMS.place.label : TERMS.place.plural;
  const users = f.headers.length === 0 ? `ohne ${tinker ? TERMS.header.label : TERMS.place.label}` : `${word} ${f.headers.map(id4).join(", ")}`;
  return [f.kind === "binary" ? "binär" : null, `${f.size} Byte`, users].filter((part) => part !== null).join(" · ");
}

const ScriptFileRow = memo(function ScriptFileRow({ file, depth, onOpenScript }: { file: ScriptFileDto; depth: Depth; onOpenScript: (file: number, line?: number) => void }) {
  const name = `${file.kind === "levelScript" ? TERMS.levelScript.label : TERMS.scriptFile.label} ${id4(file.file)}`;
  return (
    <TreeRow
      depth={1}
      icon="script"
      label={name}
      tech={fileDetails(file, depth)}
      selected={false}
      onSelect={() => onOpenScript(file.file)}
      title={`${name}, ${fileDetails(file, depth)}`}
      dataAttrs={{ "data-script-file": file.file }}
    />
  );
});

const HitRow = memo(function HitRow({ hit, onOpenScript }: { hit: ScriptHitDto; onOpenScript: (file: number, line?: number) => void }) {
  const text = `${TERMS.scriptFile.label} ${id4(hit.file)} · Zeile ${hit.line} · ${hit.excerpt}`;
  return <TreeRow depth={1} label={text} title={text} selected={false} onSelect={() => onOpenScript(hit.file, hit.line)} />;
});

/**
 * The root node "Scripts": every script file of the project and a search over their sources. The list is read when the
 * node is opened (not with the project), each time it is opened, so files made since then show. The search waits for a
 * pause in typing. Nothing is asked of the backend while rotom has not taken the scripts over.
 */
function ScriptsNode({ info, adopted, depth, onOpenScript }: { info: ProjectInfo | null; adopted: boolean; depth: Depth; onOpenScript: (file: number, line?: number) => void }) {
  const [open, setOpen] = useState(false);
  // Bumped with every opening: the list is read again.
  const [opened, setOpened] = useState(0);
  // Answers carry the project and the query they belong to, so a late one for another project or text is not shown.
  const [files, setFiles] = useState<{ info: ProjectInfo | null; list: ScriptFileDto[] } | { info: ProjectInfo | null; error: string } | null>(null);
  const [query, setQuery] = useState("");
  const [found, setFound] = useState<{ info: ProjectInfo | null; query: string; hits: ScriptHitDto[] } | { info: ProjectInfo | null; query: string; error: string } | null>(null);
  const text = query.trim();

  useEffect(() => {
    if (!open || !adopted) return;
    let current = true;
    listScriptFiles().then(
      (list) => current && setFiles({ info, list }),
      (e: unknown) => current && setFiles({ info, error: errorText(e) }),
    );
    return () => {
      current = false;
    };
  }, [open, opened, adopted, info]);

  useEffect(() => {
    if (!open || !adopted || text === "") return;
    let current = true;
    const timer = setTimeout(() => {
      searchScripts(text, SEARCH_LIMIT).then(
        (hits) => current && setFound({ info, query: text, hits }),
        (e: unknown) => current && setFound({ info, query: text, error: errorText(e) }),
      );
    }, SEARCH_DELAY_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [open, adopted, text, info]);

  const toggle = () => {
    if (!open) setOpened((n) => n + 1);
    setOpen(!open);
  };
  const list = files !== null && files.info === info ? files : null;
  const answer = found !== null && found.info === info && found.query === text ? found : null;
  const count = list !== null && "list" in list ? list.list.length : null;
  return (
    <>
      <TreeRow
        depth={0}
        icon="script"
        label={TERMS.script.plural}
        selected={false}
        expanded={open}
        onToggle={toggle}
        onSelect={toggle}
        dataAttrs={{ "data-scripts-root": "" }}
        trailing={count !== null ? <span className={styles.count}>{count}</span> : undefined}
      />
      {open &&
        (!adopted ? (
          <p role="note" className={`${styles.note} ${styles.indented}`}>
            {ADOPT_HINT}
          </p>
        ) : (
          <>
            <div className={styles.scriptSearch} role="none">
              <TextField type="search" label="Scripts durchsuchen" placeholder="Nummer oder Text suchen" value={query} onChange={setQuery} onCommit={setQuery} />
            </div>
            {text !== "" ? (
              answer === null ? (
                <p className={`${styles.note} ${styles.indented}`}>Suche …</p>
              ) : "error" in answer ? (
                <div className={styles.scriptSearch} role="none">
                  <Notice kind="error">{answer.error}</Notice>
                </div>
              ) : answer.hits.length === 0 ? (
                <p className={`${styles.note} ${styles.indented}`}>Keine Treffer.</p>
              ) : (
                answer.hits.map((hit) => <HitRow key={`${hit.file}:${hit.line}`} hit={hit} onOpenScript={onOpenScript} />)
              )
            ) : list === null ? (
              <p className={`${styles.note} ${styles.indented}`}>Lade …</p>
            ) : "error" in list ? (
              <div className={styles.scriptSearch} role="none">
                <Notice kind="error">{list.error}</Notice>
              </div>
            ) : (
              list.list.map((file) => <ScriptFileRow key={file.file} file={file} depth={depth} onOpenScript={onOpenScript} />)
            )}
          </>
        ))}
    </>
  );
}

/** "2 Weltkarten, 5 Gebiete, 40 Räume" (Tüfteln: ", 120 Header"). */
export function worldCaption(world: World, headerCount: number, tinker: boolean): string {
  const areas = world.worldMaps.reduce((n, m) => n + m.areas.length, world.moreAreas.length);
  const parts = [plural(world.worldMaps.length, "Weltkarte", "Weltkarten"), areasText(areas), roomsText(world.roomCount)];
  if (tinker) parts.push(`${headerCount} ${TERMS.header.plural}`);
  return parts.join(", ");
}

/**
 * The world as a tree: world maps with their areas and the rooms of each area, then „Weitere Gebiete“, „Nicht verbunden“
 * and (Tüfteln) „Sonstiges“, after the root node „Scripts“. Its state (search, open branches) lasts only while the tree
 * mode shows. A search opens every branch it leaves; a newly selected header opens its branch and is scrolled to.
 */
export function WorldTree({
  info,
  headers,
  world,
  matricesLoaded,
  matricesFailed,
  selected,
  shownMatrix,
  revealToken,
  depth,
  onSelect,
  scriptsAdopted,
  onOpenScript,
  marks,
  sort,
  recent,
  onTouch,
  onRename,
  onOwner,
}: WorldTreeProps) {
  const tinker = depth === "tuefteln";
  const [query, setQuery] = useState("");
  const searching = query.trim() !== "";
  const sorted = useMemo(() => (world === null ? null : sortWorld(world, sort, recent)), [world, sort, recent]);
  const shown = useMemo(() => (sorted === null ? null : filterWorld(sorted, query)), [sorted, query]);
  const branches = useMemo(() => (world === null ? new Map<number, string[]>() : branchesOf(world, tinker)), [world, tinker]);
  const byId = useMemo(() => [...headers].sort((a, b) => a.id - b.id), [headers]);
  // The areas a room can be given to, in the order of the world (not of the sort or the search).
  const choices = useMemo<AreaChoice[]>(
    () =>
      world === null
        ? []
        : [
            ...world.worldMaps.flatMap((m) => m.areas.map((a) => ({ key: a.key, name: a.name, where: m.name }))),
            ...world.moreAreas.map((a) => ({ key: a.key, name: a.name, where: "Weitere Gebiete" })),
          ],
    [world],
  );

  // The branches whose open state differs from the default (world maps open, the rest closed).
  const [toggled, setToggled] = useState<ReadonlySet<string>>(new Set());
  const isOpen = (key: string) => searching || openByDefault(key) !== toggled.has(key);
  const toggleBranch = useCallback(
    (key: string) => {
      if (searching) return;
      setToggled((prev) => {
        const next = new Set(prev);
        if (!next.delete(key)) next.add(key);
        return next;
      });
    },
    [searching],
  );
  const onPick = useCallback(
    (h: HeaderSummary, areaKey: string | null) => {
      onSelect(h);
      if (areaKey !== null) onTouch(areaKey);
    },
    [onSelect, onTouch],
  );

  // The host is always rendered, so the arrow keys work after a search and after switching modes.
  const tree = useRef<HTMLDivElement>(null);
  useRovingList(tree);
  // The selection, token and area last scrolled to, so a search that hides the selection later does not pull it back.
  const revealed = useRef<string | null>(null);
  // A newly selected header (also one chosen by a link, or a room given to another area) is scrolled into view, its
  // branch opened first; a search that hides it is cleared first.
  useEffect(() => {
    if (selected === null) {
      revealed.current = null;
      return;
    }
    const key = `${selected}:${revealToken}:${world?.areaOf.get(selected) ?? ""}`;
    if (revealed.current === key) return;
    const closed = searching ? [] : (branches.get(selected) ?? []).filter((k) => !isOpen(k));
    if (closed.length > 0) {
      // The row is not rendered while a branch above it is closed: open them, the render after that scrolls.
      setToggled((prev) => {
        const next = new Set(prev);
        for (const k of closed) if (!next.delete(k)) next.add(k);
        return next;
      });
      return;
    }
    // Looked up by id when needed, instead of a ref callback per row; an area row carries its first header's id too.
    const host = tree.current;
    const row = host?.querySelector(`[data-header-id="${selected}"][aria-selected="true"]`) ?? host?.querySelector(`[data-header-id="${selected}"]`);
    if (row) {
      revealed.current = key;
      row.scrollIntoView({ block: "nearest" });
    } else if (searching) {
      setQuery("");
    }
  });

  const rowsOfRooms = (rooms: readonly Room[], areaKey: string | null, areaPlace: string | null, level: number) =>
    rooms.map((room) => (
      <RoomRow
        key={room.header.id}
        room={room}
        areaKey={areaKey}
        depth={level}
        labelled={room.placeKey !== areaPlace && room.name !== room.placeName}
        tinker={tinker}
        selected={room.header.id === selected}
        marked={showsMatrix(room.header, shownMatrix)}
        mark={marks?.get(room.header.id)}
        areas={choices}
        onPick={onPick}
        onRename={onRename}
        onOwner={onOwner}
      />
    ));

  const leaf = (h: HeaderSummary, level: number, label: string, icon?: IconName) => (
    <HeaderLeaf
      key={h.id}
      header={h}
      depth={level}
      label={label}
      icon={icon}
      tinker={tinker}
      selected={h.id === selected}
      marked={showsMatrix(h, shownMatrix)}
      mark={marks?.get(h.id)}
      onSelect={onSelect}
    />
  );

  const rowsOfAreas = (areas: readonly Area[], level: number) =>
    areas.map((area) => {
      const open = isOpen(area.key);
      const sections = tinker && area.headers.length > 1;
      return (
        <Fragment key={area.key}>
          <AreaRow
            area={area}
            depth={level}
            tinker={tinker}
            selected={sections ? false : area.headers.some((h) => h.id === selected)}
            marked={area.headers.some((h) => showsMatrix(h, shownMatrix))}
            mark={worstMark(marks, area.headers)}
            open={open}
            onPick={onPick}
            onToggle={toggleBranch}
          />
          {open && sections && area.headers.map((h, i) => leaf(h, level + 1, `Abschnitt ${i + 1}`, "outside"))}
          {open && rowsOfRooms(area.rooms, area.key, area.placeKey, level + 1)}
        </Fragment>
      );
    });

  const groupRow = (key: string, label: string, icon: IconName, count: number, ariaCount: string) => (
    <TreeRow
      depth={0}
      icon={icon}
      label={label}
      ariaLabel={`${label}, ${ariaCount}`}
      selected={false}
      expanded={isOpen(key)}
      onToggle={() => toggleBranch(key)}
      onSelect={() => toggleBranch(key)}
      dataAttrs={{ "data-world-group": key.slice("group:".length) }}
      trailing={<span className={styles.count}>{count}</span>}
    />
  );

  const empty = shown !== null && shown.worldMaps.length === 0 && shown.moreAreas.length === 0 && shown.unconnected.length === 0 && (!tinker || shown.other.length === 0);

  return (
    <div className={styles.column}>
      <div className={styles.top}>
        <p className={styles.caption}>{world !== null ? worldCaption(world, headers.length, tinker) : matricesFailed && !matricesLoaded ? "Übersicht nicht verfügbar" : "Lade Übersicht …"}</p>
        <TextField type="search" label="Welt durchsuchen" placeholder="Gebiet, Raum oder Kürzel suchen" value={query} onChange={setQuery} onCommit={setQuery} />
      </div>
      <div ref={tree} role="tree" aria-label="Orte" className={styles.host}>
        <ScriptsNode info={info} adopted={scriptsAdopted} depth={depth} onOpenScript={onOpenScript} />
        {shown === null ? (
          // Until the world is read, the headers by id, so a selection and a link still find their row.
          byId.filter((h) => !searching || textMatches(query.trim().toLowerCase(), h.location_name) || codeMatches(h, query.trim().toLowerCase())).map((h) => leaf(h, 0, h.location_name || h.internal_name))
        ) : (
          <>
            {empty && searching && <p className={styles.note}>Kein Ort passt zur Suche.</p>}
            {shown.worldMaps.map((map) => {
              const open = isOpen(map.key);
              return (
                <Fragment key={map.key}>
                  <TreeRow
                    depth={0}
                    icon="world"
                    label={map.name}
                    tech={tinker ? techLabel("matrix", map.matrix) : undefined}
                    ariaLabel={`${map.name}, ${areasText(map.areas.length)}`}
                    title={tinker ? techLabel("matrix", map.matrix) : undefined}
                    selected={false}
                    expanded={open}
                    onToggle={() => toggleBranch(map.key)}
                    onSelect={() => toggleBranch(map.key)}
                    dataAttrs={{ "data-world-map": map.key }}
                    trailing={
                      <>
                        <span className={styles.count}>{map.areas.length}</span>
                        <WorldMapActions map={map} onRename={onRename} />
                      </>
                    }
                  />
                  {open && rowsOfAreas(map.areas, 1)}
                </Fragment>
              );
            })}
            {shown.moreAreas.length > 0 && (
              <>
                {groupRow(MORE, "Weitere Gebiete", "place", shown.moreAreas.length, areasText(shown.moreAreas.length))}
                {isOpen(MORE) && rowsOfAreas(shown.moreAreas, 1)}
              </>
            )}
            {shown.unconnected.length > 0 && (
              <>
                {groupRow(UNCONNECTED, "Nicht verbunden", "room", shown.unconnected.length, roomsText(shown.unconnected.length))}
                {isOpen(UNCONNECTED) && rowsOfRooms(shown.unconnected, null, null, 1)}
              </>
            )}
            {tinker && shown.other.length > 0 && (
              <>
                {groupRow(OTHER, "Sonstiges", "tinker", shown.other.length, `${shown.other.length} ${TERMS.header.plural}`)}
                {isOpen(OTHER) && shown.other.map((h) => leaf(h, 1, h.location_name || h.internal_name))}
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
}
