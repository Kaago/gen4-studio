import { Fragment, memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { errorText, listScriptFiles, searchScripts } from "../api";
import type { HeaderSummary, MatrixSummary, ProjectInfo, ScriptFileDto, ScriptHitDto } from "../api";
import { gameLabel } from "../app/gameLabel";
import { FindingMark } from "../checks/FindingMark";
import type { MarkSeverity } from "../checks/FindingMark";
import { ListPane } from "../ui/frame/ListPane";
import type { Depth } from "../ui/frame/shellState";
import { Icon, Menu, Notice, Segmented, TextField, TreeRow, useRovingList } from "../ui/primitives";
import { HeaderList } from "./HeaderList";
import { showsMatrix } from "./headers";
import { buildPlaces, filterPlaces, mapsText, placeColor, placeOf, scriptChildren } from "./places";
import type { PlaceHeader } from "./places";
import { ADOPT_HINT } from "./inspector/scriptNav";
import { ShownMark } from "./ShownMark";
import { TERMS, id4, techLabel } from "./terms";
import type { WizardRequest } from "./WorkbenchContext";
import styles from "./WorldExplorer.module.css";

export interface WorldExplorerProps {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  matrices: MatrixSummary[];
  /** False until the overview's first matrix listing arrived: the rooms' details are left out then (every header would count as a room). */
  matricesLoaded: boolean;
  /** The first matrix listing failed (and nothing is loaded): the header line says the overview is not available instead of loading. */
  matricesFailed: boolean;
  selected: number | null;
  /** The matrix the scene shows: headers using it are marked. */
  shownMatrix: number | null;
  /** Bumped by every link to a header: the tree scrolls to the selection again, also when it was selected already. */
  revealToken: number;
  /** Tüfteln writes the identifiers (id, code) into every header row; Bauen leaves them to the tooltip. */
  depth: Depth;
  onSelect: (h: HeaderSummary) => void;
  onOpenWizard: (request: WizardRequest) => void;
  /** Whether rotom has taken the scripts over; without it the script entries only show a hint. */
  scriptsAdopted: boolean;
  /** Opens a script file in the script panel (at a 1-based line). Must keep its identity (the tree is memoised). */
  onOpenScript: (file: number, line?: number) => void;
  /** Whether the list column shows or is folded to its grip (the shell remembers it per area). */
  listOpen: boolean;
  onToggleList: () => void;
  /** The worst error or warning the checks found per header (`headerMarks`): a mark at the end of its row. None: no marks. */
  marks?: ReadonlyMap<number, MarkSeverity>;
}

type Mode = "tree" | "list";
const MODE_KEY = "g4.explorer.mode";

function readMode(): Mode {
  try {
    return localStorage.getItem(MODE_KEY) === "list" ? "list" : "tree";
  } catch {
    return "tree";
  }
}

function writeMode(mode: Mode): void {
  try {
    localStorage.setItem(MODE_KEY, mode);
  } catch {
    /* the mode is simply not remembered */
  }
}

/**
 * One header of a place. The place already carries the location name, so a room is named by its
 * internal name (as long as the overview is not known, every row is named by the location name:
 * outside and room are not told apart yet); the grey part has the room's maps and, in Tüfteln, the id and the code (when it is not the name).
 * Memoised: a selection change renders the rows whose props changed, not all of them.
 */
const HeaderRow = memo(function HeaderRow({
  entry,
  selected,
  marked,
  showRooms,
  tinker,
  expanded,
  adopted,
  mark,
  onSelect,
  onToggle,
  onOpenScript,
}: {
  entry: PlaceHeader;
  selected: boolean;
  marked: boolean;
  /** What the checks found at the header or its events. */
  mark: MarkSeverity | undefined;
  showRooms: boolean;
  /** Depth Tüfteln: the identifiers show. */
  tinker: boolean;
  /** The header's script entries are shown below it. */
  expanded: boolean;
  adopted: boolean;
  onSelect: (h: HeaderSummary) => void;
  onToggle: (id: number) => void;
  onOpenScript: (file: number, line?: number) => void;
}) {
  const h = entry.header;
  const name = entry.kind === "outside" || !showRooms ? h.location_name || h.internal_name : h.internal_name;
  const grey = [tinker ? id4(h.id) : null, tinker && h.internal_name !== name ? h.internal_name : null, showRooms && entry.kind === "inside" ? mapsText(entry) : null].filter((part) => part !== null);
  return (
    <>
      <TreeRow
        depth={1}
        label={name}
        tech={grey.join(" · ")}
        // Outside and room cannot be told apart before the overview is known.
        icon={showRooms ? (entry.kind === "outside" ? "outside" : "room") : undefined}
        selected={selected}
        onSelect={() => onSelect(h)}
        expanded={expanded}
        onToggle={() => onToggle(h.id)}
        title={`${techLabel("header", h.id)} ${h.internal_name}, ${techLabel("matrix", h.matrix_id)}${marked ? ` (${TERMS.matrix.label} wird angezeigt)` : ""}`}
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
      {expanded &&
        scriptChildren(h).map((child) => (
          <TreeRow
            key={child.kind}
            depth={2}
            icon="script"
            label={child.label}
            selected={false}
            disabled={!adopted}
            title={adopted ? undefined : ADOPT_HINT}
            onSelect={() => onOpenScript(child.file)}
            dataAttrs={{ "data-script-file": child.file }}
          />
        ))}
    </>
  );
});

const SEARCH_DELAY_MS = 300;
const SEARCH_LIMIT = 100;

/** "412 Byte" for a script source; the explorer's grey part says what a file is and which headers use it. */
function fileDetails(f: ScriptFileDto): string {
  const users = f.headers.length === 0 ? "ohne Header" : `${TERMS.header.plural} ${f.headers.map(id4).join(", ")}`;
  return [f.kind === "binary" ? "binär" : null, `${f.size} Byte`, users].filter((part) => part !== null).join(" · ");
}

const ScriptFileRow = memo(function ScriptFileRow({ file, onOpenScript }: { file: ScriptFileDto; onOpenScript: (file: number, line?: number) => void }) {
  const name = `${file.kind === "levelScript" ? TERMS.levelScript.label : TERMS.scriptFile.label} ${id4(file.file)}`;
  return (
    <TreeRow
      depth={1}
      icon="script"
      label={name}
      tech={fileDetails(file)}
      selected={false}
      onSelect={() => onOpenScript(file.file)}
      title={`${name}, ${fileDetails(file)}`}
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
function ScriptsNode({ info, adopted, onOpenScript }: { info: ProjectInfo | null; adopted: boolean; onOpenScript: (file: number, line?: number) => void }) {
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
              list.list.map((file) => <ScriptFileRow key={file.file} file={file} onOpenScript={onOpenScript} />)
            )}
          </>
        ))}
    </>
  );
}

type TreeProps = Omit<WorldExplorerProps, "onOpenWizard" | "listOpen" | "onToggleList">;

/** The places as tree rows with their headers below. Its state (search, opened places) lasts only while the tree mode shows. */
function WorldTree({ info, headers, matrices, matricesLoaded, matricesFailed, selected, shownMatrix, revealToken, depth, onSelect, scriptsAdopted, onOpenScript, marks }: TreeProps) {
  const [query, setQuery] = useState("");
  // Without the overview every header would count as a room: the places then keep plain id order. The warps are not needed here, so a new warp list renders nothing.
  const places = useMemo(() => buildPlaces(headers, matricesLoaded ? matrices : []), [headers, matrices, matricesLoaded]);
  const shown = useMemo(() => filterPlaces(places, query), [places, query]);
  // The places the user (or a selection) opened; a search opens every place it leaves.
  const [openKeys, setOpenKeys] = useState<ReadonlySet<string>>(() => {
    const place = selected === null ? undefined : placeOf(places, selected);
    return new Set(place ? [place.key] : []);
  });
  // The headers whose script entries are shown below them.
  const [expandedHeaders, setExpandedHeaders] = useState<ReadonlySet<number>>(new Set());
  const toggleHeader = useCallback(
    (id: number) =>
      setExpandedHeaders((prev) => {
        const next = new Set(prev);
        if (!next.delete(id)) next.add(id);
        return next;
      }),
    [],
  );
  // The host is always rendered, so the arrow keys work after a search and after switching modes.
  const tree = useRef<HTMLDivElement>(null);
  useRovingList(tree);
  // The selection and token last scrolled to, so a search that hides the selection later does not pull it back.
  const revealed = useRef<string | null>(null);
  // A newly selected header (also one chosen by a link) is scrolled into view, its place opened
  // first; a search that hides it is cleared first.
  useEffect(() => {
    if (selected === null) {
      revealed.current = null;
      return;
    }
    const key = `${selected}:${revealToken}`;
    if (revealed.current === key) return;
    const place = placeOf(shown, selected);
    if (place && query === "" && !openKeys.has(place.key)) {
      // The row is not rendered while its place is closed: open the place, the render after that scrolls.
      setOpenKeys((prev) => new Set(prev).add(place.key));
      return;
    }
    // Looked up by id when needed, instead of a ref callback per row.
    const row = tree.current?.querySelector(`[data-header-id="${selected}"]`);
    if (row) {
      revealed.current = key;
      row.scrollIntoView({ block: "nearest" });
    } else if (query !== "") {
      setQuery("");
    }
  });

  const togglePlace = (placeKey: string) => {
    if (query !== "") return;
    setOpenKeys((prev) => {
      const next = new Set(prev);
      if (!next.delete(placeKey)) next.add(placeKey);
      return next;
    });
  };

  return (
    <div className={styles.column}>
      <div className={styles.top}>
        <p className={styles.caption}>
          {matricesLoaded ? `${places.length} ${TERMS.place.plural}, ${headers.length} ${TERMS.header.plural}` : matricesFailed ? "Übersicht nicht verfügbar" : "Lade Übersicht …"}
        </p>
        <TextField type="search" label="Welt durchsuchen" placeholder="Ort, Raum oder Kürzel suchen" value={query} onChange={setQuery} onCommit={setQuery} />
      </div>
      <div ref={tree} role="tree" aria-label="Orte und Header" className={styles.host}>
        <ScriptsNode info={info} adopted={scriptsAdopted} onOpenScript={onOpenScript} />
        {shown.length === 0 && query !== "" && <p className={styles.note}>Kein Ort passt zur Suche.</p>}
        {shown.map((place) => {
          const open = query !== "" || openKeys.has(place.key);
          const count = place.headers.length;
          return (
            <Fragment key={place.key}>
              <TreeRow
                depth={0}
                icon="place"
                label={place.name}
                ariaLabel={`${place.name}, ${count} ${count === 1 ? TERMS.header.label : TERMS.header.plural}`}
                selected={false}
                expanded={open}
                onToggle={() => togglePlace(place.key)}
                onSelect={() => togglePlace(place.key)}
                dataAttrs={{ "data-place-key": place.key }}
                trailing={
                  <>
                    <span data-place-chip="" className={styles.chip} style={{ background: placeColor(place.key) }} /* dyn */ />
                    <span className={styles.count}>{count}</span>
                  </>
                }
              />
              {open &&
                place.headers.map((entry) => (
                  <HeaderRow
                    key={entry.header.id}
                    entry={entry}
                    selected={entry.header.id === selected}
                    marked={showsMatrix(entry.header, shownMatrix)}
                    showRooms={matricesLoaded}
                    tinker={depth === "tuefteln"}
                    expanded={expandedHeaders.has(entry.header.id)}
                    adopted={scriptsAdopted}
                    mark={marks?.get(entry.header.id)}
                    onSelect={onSelect}
                    onToggle={toggleHeader}
                    onOpenScript={onOpenScript}
                  />
                ))}
            </Fragment>
          );
        })}
      </div>
    </div>
  );
}

/**
 * The list pane of the world: the headers as places (tree) or as the flat list, with the menu for the wizards in its head.
 * Memoised like the list: it renders when the headers, the overview, the selection, the matrix shown, the reveal token or
 * a callback change, not when the scene is hovered or a message arrives. `onSelect`, `onOpenWizard` and `onToggleList`
 * must keep their identity for that.
 */
export const WorldExplorer = memo(function WorldExplorer({ onOpenWizard, listOpen, onToggleList, ...tree }: WorldExplorerProps) {
  const [mode, setMode] = useState<Mode>(readMode);
  const changeMode = (next: Mode) => {
    setMode(next);
    writeMode(next);
  };
  const actions = (
    <>
      <Segmented<Mode>
        size="s"
        iconOnly
        ariaLabel="Ansicht"
        value={mode}
        onChange={changeMode}
        options={[
          { value: "tree", label: "Baum", icon: "tree" },
          { value: "list", label: "Liste", icon: "list" },
        ]}
      />
      <Menu
        label="Neu anlegen"
        trigger={<Icon name="plus" />}
        items={[
          { label: "Gebiet anlegen…", onSelect: () => onOpenWizard({ kind: "area" }) },
          { label: "Raum anlegen…", onSelect: () => onOpenWizard({ kind: "room" }) },
        ]}
      />
    </>
  );
  return (
    <ListPane title="Orte" subtitle={tree.info ? gameLabel(tree.info) : undefined} actions={actions} open={listOpen} onToggle={onToggleList}>
      {mode === "list" ? (
        <HeaderList info={tree.info} headers={tree.headers} selected={tree.selected} shownMatrix={tree.shownMatrix} revealToken={tree.revealToken} depth={tree.depth} onSelect={tree.onSelect} marks={tree.marks} />
      ) : (
        <WorldTree {...tree} />
      )}
    </ListPane>
  );
});
