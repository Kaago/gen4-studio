import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties, KeyboardEvent as ReactKeyboardEvent } from "react";
import { errorText, listScriptFiles, searchScripts } from "../api";
import type { HeaderSummary, MatrixSummary, ProjectInfo, ScriptFileDto, ScriptHitDto } from "../api";
import { HeaderList } from "./HeaderList";
import { showsMatrix } from "./headers";
import { buildPlaces, filterPlaces, mapsText, placeColor, placeOf, scriptChildren } from "./places";
import type { PlaceHeader } from "./places";
import { ADOPT_HINT } from "./inspector/scriptNav";
import { TERMS, id4, techLabel } from "./terms";
import { TermsPopover } from "./TermsPopover";
import { popoverStyle, useDismiss } from "./useDismiss";
import type { WizardRequest } from "./WorkbenchContext";

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
  onSelect: (h: HeaderSummary) => void;
  onOpenWizard: (request: WizardRequest) => void;
  /** Whether rotom has taken the scripts over; without it the script entries only show a hint. */
  scriptsAdopted: boolean;
  /** Opens a script file in the script panel (at a 1-based line). Must keep its identity (the tree is memoised). */
  onOpenScript: (file: number, line?: number) => void;
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

const ellipsis: CSSProperties = { whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" };
const GREY = "#6b7280";
/** A row that only opens something (a script entry): looks like text, takes the cursor. */
const linkButton: CSSProperties = { all: "unset", boxSizing: "border-box", display: "block", width: "100%", cursor: "pointer", lineHeight: 1.6, ...ellipsis };

/**
 * One header of a place. The place already carries the location name, so a room is named by its
 * internal name (as long as the overview is not known, every row is named by the location name:
 * outside and room are not told apart yet); the grey part has the id, the code (when it is not the name) and the room's maps.
 * Memoised: a selection change renders the rows whose props changed, not all of them.
 */
const HeaderRow = memo(function HeaderRow({
  entry,
  selected,
  marked,
  showRooms,
  expanded,
  adopted,
  onSelect,
  onToggle,
  onOpenScript,
}: {
  entry: PlaceHeader;
  selected: boolean;
  marked: boolean;
  showRooms: boolean;
  /** The header's script entries are shown below it. */
  expanded: boolean;
  adopted: boolean;
  onSelect: (h: HeaderSummary) => void;
  onToggle: (id: number) => void;
  onOpenScript: (file: number, line?: number) => void;
}) {
  const h = entry.header;
  const name = entry.kind === "outside" || !showRooms ? h.location_name || h.internal_name : h.internal_name;
  const grey = [id4(h.id), h.internal_name !== name ? h.internal_name : null, showRooms && entry.kind === "inside" ? mapsText(entry) : null].filter((part) => part !== null);
  return (
    <li>
      <div style={{ display: "flex", alignItems: "baseline" }}>
        <button
          type="button"
          aria-expanded={expanded}
          aria-label={`${TERMS.script.plural} von ${name} ${expanded ? "ausblenden" : "anzeigen"}`}
          onClick={() => onToggle(h.id)}
          style={{ all: "unset", cursor: "pointer", width: 14, flex: "0 0 auto", textAlign: "center", color: GREY, fontSize: 10 }}
        >
          {expanded ? "▾" : "▸"}
        </button>
        <button
          data-header-id={h.id}
          onClick={() => onSelect(h)}
          aria-current={selected ? "true" : undefined}
          title={`${techLabel("header", h.id)} ${h.internal_name}, ${techLabel("matrix", h.matrix_id)}${marked ? ` (${TERMS.matrix.label} wird angezeigt)` : ""}`}
          style={{
            all: "unset",
            boxSizing: "border-box",
            display: "flex",
            alignItems: "baseline",
            columnGap: 8,
            flex: "1 1 auto",
            minWidth: 0,
            cursor: "pointer",
            lineHeight: 1.6,
            fontWeight: selected ? 700 : 400,
            background: marked ? "#eff6ff" : "transparent",
            boxShadow: marked ? "inset 3px 0 0 #2563eb" : "none",
          }}
        >
          <span style={{ ...ellipsis, flex: "1 1 auto", minWidth: 0, paddingLeft: 4 }}>{name}</span>
          <span style={{ ...ellipsis, flex: "0 1 auto", color: GREY, fontSize: 12, fontWeight: 400, minWidth: 0 }}>{grey.join(" · ")}</span>
        </button>
      </div>
      {expanded && (
        <ul style={{ listStyle: "none", margin: 0, padding: "0 0 2px 22px" }}>
          {scriptChildren(h).map((child) => (
            <li key={child.kind}>
              <button type="button" data-script-file={child.file} disabled={!adopted} title={adopted ? undefined : ADOPT_HINT} onClick={() => onOpenScript(child.file)} style={adopted ? linkButton : { ...linkButton, opacity: 0.5, cursor: "default" }}>
                {child.label}
              </button>
            </li>
          ))}
        </ul>
      )}
    </li>
  );
});

const barButton: CSSProperties = { fontFamily: "inherit", fontSize: 12 };

/** "+ Neu": a menu of what the wizards can create. */
function NewMenu({ onOpenWizard }: { onOpenWizard: (request: WizardRequest) => void }) {
  const [open, setOpen] = useState(false);
  const wrapper = useRef<HTMLSpanElement>(null);
  const toggle = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  // Closing while the focus is in the menu gives it back to "+ Neu" (Escape, a choice).
  const close = useCallback(() => {
    if (menu.current?.contains(document.activeElement)) toggle.current?.focus();
    setOpen(false);
  }, []);
  useDismiss(open, wrapper, close);
  // An opened menu takes the focus on its first entry, so the arrow keys work at once.
  useEffect(() => {
    if (open) menu.current?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
  }, [open]);
  const choose = (request: WizardRequest) => {
    close();
    onOpenWizard(request);
  };
  /** Up and down move between the entries (wrapping), Home and End jump to the ends. */
  const onMenuKey = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    const items = [...(menu.current?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])];
    const at = items.indexOf(document.activeElement as HTMLElement);
    const to = e.key === "ArrowDown" ? (at + 1) % items.length : e.key === "ArrowUp" ? (at - 1 + items.length) % items.length : e.key === "Home" ? 0 : e.key === "End" ? items.length - 1 : null;
    if (to === null || items.length === 0) return;
    e.preventDefault();
    items[to].focus();
  };
  const entry: CSSProperties = { all: "unset", display: "block", padding: "4px 12px", cursor: "pointer", whiteSpace: "nowrap" };
  return (
    <span ref={wrapper} style={{ position: "relative" }}>
      <button ref={toggle} type="button" aria-haspopup="menu" aria-expanded={open} onClick={() => setOpen((o) => !o)} style={barButton}>
        + Neu
      </button>
      {open && (
        <div ref={menu} role="menu" onKeyDown={onMenuKey} style={{ ...popoverStyle, left: 0, padding: "4px 0" }}>
          <button role="menuitem" onClick={() => choose({ kind: "area" })} style={entry}>
            Gebiet anlegen…
          </button>
          <button role="menuitem" onClick={() => choose({ kind: "room" })} style={entry}>
            Raum anlegen…
          </button>
        </div>
      )}
    </span>
  );
}

function ModeSwitch({ mode, onChange }: { mode: Mode; onChange: (mode: Mode) => void }) {
  return (
    <span role="group" aria-label="Ansicht" style={{ display: "inline-flex" }}>
      {(["tree", "list"] as const).map((m) => (
        <button key={m} type="button" aria-pressed={mode === m} onClick={() => onChange(m)} style={{ ...barButton, fontWeight: mode === m ? 700 : 400 }}>
          {m === "tree" ? "Baum" : "Liste"}
        </button>
      ))}
    </span>
  );
}


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
    <li>
      <button type="button" data-script-file={file.file} onClick={() => onOpenScript(file.file)} title={`${name}, ${fileDetails(file)}`} style={{ ...linkButton, display: "flex", columnGap: 8 }}>
        <span style={{ ...ellipsis, flex: "0 0 auto" }}>{name}</span>
        <span style={{ ...ellipsis, flex: "1 1 auto", minWidth: 0, color: GREY, fontSize: 12 }}>{fileDetails(file)}</span>
      </button>
    </li>
  );
});

const HitRow = memo(function HitRow({ hit, onOpenScript }: { hit: ScriptHitDto; onOpenScript: (file: number, line?: number) => void }) {
  return (
    <li>
      <button type="button" onClick={() => onOpenScript(hit.file, hit.line)} style={linkButton}>
        {TERMS.scriptFile.label} {id4(hit.file)} · Zeile {hit.line} · {hit.excerpt}
      </button>
    </li>
  );
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

  const list = files !== null && files.info === info ? files : null;
  const answer = found !== null && found.info === info && found.query === text ? found : null;
  const count = list !== null && "list" in list ? list.list.length : null;
  return (
    <details
      data-scripts-root=""
      open={open}
      onToggle={(e) => {
        const now = e.currentTarget.open;
        if (now && !open) setOpened((n) => n + 1);
        setOpen(now);
      }}
    >
      <summary style={{ cursor: "pointer", lineHeight: 1.6 }}>
        {TERMS.script.plural}
        {count !== null && (
          <span aria-hidden="true" style={{ float: "right", color: GREY, fontSize: 12 }}>
            {count}
          </span>
        )}
      </summary>
      {open &&
        (!adopted ? (
          <p role="note" style={{ margin: "2px 0 4px 16px", color: GREY }}>
            {ADOPT_HINT}
          </p>
        ) : (
          <div style={{ padding: "0 0 4px 16px" }}>
            <input
              type="search"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Nummer oder Text suchen"
              aria-label="Scripts durchsuchen"
              style={{ width: "100%", boxSizing: "border-box", padding: "2px 6px", marginBottom: 4 }}
            />
            {text !== "" ? (
              answer === null ? (
                <p style={{ margin: 0, color: GREY }}>Suche …</p>
              ) : "error" in answer ? (
                <p role="alert" style={{ margin: 0, color: "#b91c1c" }}>
                  {answer.error}
                </p>
              ) : answer.hits.length === 0 ? (
                <p style={{ margin: 0, color: GREY }}>Keine Treffer.</p>
              ) : (
                <ul style={{ listStyle: "none", margin: 0, padding: 0 }}>
                  {answer.hits.map((hit) => (
                    <HitRow key={`${hit.file}:${hit.line}`} hit={hit} onOpenScript={onOpenScript} />
                  ))}
                </ul>
              )
            ) : list === null ? (
              <p style={{ margin: 0, color: GREY }}>Lade …</p>
            ) : "error" in list ? (
              <p role="alert" style={{ margin: 0, color: "#b91c1c" }}>
                {list.error}
              </p>
            ) : (
              <ul style={{ listStyle: "none", margin: 0, padding: 0 }}>
                {list.list.map((file) => (
                  <ScriptFileRow key={file.file} file={file} onOpenScript={onOpenScript} />
                ))}
              </ul>
            )}
          </div>
        ))}
    </details>
  );
}

type TreeProps = Omit<WorldExplorerProps, "onOpenWizard">;

/** The places as one `details` each. Its state (search, opened places) lasts only while the tree mode shows. */
function WorldTree({ info, headers, matrices, matricesLoaded, matricesFailed, selected, shownMatrix, revealToken, onSelect, scriptsAdopted, onOpenScript }: TreeProps) {
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
  const tree = useRef<HTMLDivElement>(null);
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
    // Looked up by id when needed, instead of a ref callback per row.
    const row = tree.current?.querySelector(`[data-header-id="${selected}"]`);
    if (row) {
      const details = row.closest("details");
      const placeKey = details?.dataset.placeKey;
      if (details && !details.open && placeKey !== undefined && !openKeys.has(placeKey)) {
        // The row is in the DOM but hidden: open its place, the render after that scrolls.
        setOpenKeys((prev) => new Set(prev).add(placeKey));
        return;
      }
      revealed.current = key;
      row.scrollIntoView({ block: "nearest" });
    } else if (query !== "") {
      setQuery("");
    }
  });

  const onToggle = (placeKey: string, open: boolean) => {
    if (query !== "") return;
    setOpenKeys((prev) => {
      if (prev.has(placeKey) === open) return prev;
      const next = new Set(prev);
      if (open) next.add(placeKey);
      else next.delete(placeKey);
      return next;
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", boxSizing: "border-box" }}>
      <div style={{ padding: "4px 16px 8px" }}>
        <p style={{ margin: "0 0 8px", ...ellipsis }}>
          {matricesLoaded
            ? `${info ? `${info.game_code} (${info.family}, ${info.language}): ` : ""}${places.length} ${TERMS.place.plural}, ${headers.length} ${TERMS.header.plural}`
            : matricesFailed
              ? "Übersicht nicht verfügbar"
              : "Lade Übersicht …"}
        </p>
        <input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Suchen nach Ort oder Kürzel"
          aria-label="Welt durchsuchen"
          style={{ width: "100%", boxSizing: "border-box", padding: "4px 8px" }}
        />
      </div>
      <div ref={tree} style={{ flex: 1, minHeight: 0, overflowY: "auto", padding: "0 16px 16px" }}>
        <ScriptsNode info={info} adopted={scriptsAdopted} onOpenScript={onOpenScript} />
        {shown.map((place) => (
          <details key={place.key} data-place-key={place.key} open={query !== "" || openKeys.has(place.key)} onToggle={(e) => onToggle(place.key, e.currentTarget.open)}>
            <summary style={{ cursor: "pointer", lineHeight: 1.6 }} aria-label={`${place.name}, ${place.headers.length} ${place.headers.length === 1 ? TERMS.header.label : TERMS.header.plural}`}>
              <span data-place-chip="" style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, marginRight: 6, background: placeColor(place.key) }} />
              {place.name}
              <span aria-hidden="true" style={{ float: "right", color: GREY, fontSize: 12 }}>
                {place.headers.length}
              </span>
            </summary>
            <ul style={{ listStyle: "none", margin: 0, padding: "0 0 4px 16px" }}>
              {place.headers.map((entry) => (
                <HeaderRow
                  key={entry.header.id}
                  entry={entry}
                  selected={entry.header.id === selected}
                  marked={showsMatrix(entry.header, shownMatrix)}
                  showRooms={matricesLoaded}
                  expanded={expandedHeaders.has(entry.header.id)}
                  adopted={scriptsAdopted}
                  onSelect={onSelect}
                  onToggle={toggleHeader}
                  onOpenScript={onOpenScript}
                />
              ))}
            </ul>
          </details>
        ))}
      </div>
    </div>
  );
}

/**
 * The left panel: the headers as places (tree) or as the flat list, with "+ Neu" for the wizards
 * and the glossary. Memoised like the list: it renders when the headers, the overview, the
 * selection, the matrix shown, the reveal token or a callback change, not when the scene is
 * hovered or a message arrives. `onSelect` and `onOpenWizard` must keep their identity for that.
 */
export const WorldExplorer = memo(function WorldExplorer({ onOpenWizard, ...tree }: WorldExplorerProps) {
  const [mode, setMode] = useState<Mode>(readMode);
  const changeMode = (next: Mode) => {
    setMode(next);
    writeMode(next);
  };
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", boxSizing: "border-box", fontFamily: "system-ui, sans-serif", fontSize: 13 }}>
      <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", gap: 8, padding: "12px 16px 8px" }}>
        <NewMenu onOpenWizard={onOpenWizard} />
        <ModeSwitch mode={mode} onChange={changeMode} />
        <span style={{ marginLeft: "auto" }}>
          <TermsPopover />
        </span>
      </div>
      <div style={{ flex: 1, minHeight: 0 }}>
        {mode === "list" ? (
          <HeaderList info={tree.info} headers={tree.headers} selected={tree.selected} shownMatrix={tree.shownMatrix} revealToken={tree.revealToken} onSelect={tree.onSelect} />
        ) : (
          <WorldTree {...tree} />
        )}
      </div>
    </div>
  );
});
