import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties } from "react";
import type { HeaderSummary, MatrixSummary, ProjectInfo } from "../api";
import { HeaderList } from "./HeaderList";
import { showsMatrix } from "./headers";
import { buildPlaces, filterPlaces, mapsText, placeColor, placeOf } from "./places";
import type { PlaceHeader } from "./places";
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

/**
 * One header of a place. The place already carries the location name, so a room is named by its
 * internal name (as long as the overview is not known, every row is named by the location name:
 * outside and room are not told apart yet); the grey part has the id, the code (when it is not the name) and the room's maps.
 * Memoised: a selection change renders the rows whose props changed, not all of them.
 */
const HeaderRow = memo(function HeaderRow({ entry, selected, marked, showRooms, onSelect }: { entry: PlaceHeader; selected: boolean; marked: boolean; showRooms: boolean; onSelect: (h: HeaderSummary) => void }) {
  const h = entry.header;
  const name = entry.kind === "outside" || !showRooms ? h.location_name || h.internal_name : h.internal_name;
  const grey = [id4(h.id), h.internal_name !== name ? h.internal_name : null, showRooms && entry.kind === "inside" ? mapsText(entry) : null].filter((part) => part !== null);
  return (
    <li>
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
          width: "100%",
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
    </li>
  );
});

const barButton: CSSProperties = { fontFamily: "inherit", fontSize: 12 };

/** "+ Neu": a menu of what the wizards can create. */
function NewMenu({ onOpenWizard }: { onOpenWizard: (request: WizardRequest) => void }) {
  const [open, setOpen] = useState(false);
  const wrapper = useRef<HTMLSpanElement>(null);
  const close = useCallback(() => setOpen(false), []);
  useDismiss(open, wrapper, close);
  const choose = (request: WizardRequest) => {
    setOpen(false);
    onOpenWizard(request);
  };
  const entry: CSSProperties = { all: "unset", display: "block", padding: "4px 12px", cursor: "pointer", whiteSpace: "nowrap" };
  return (
    <span ref={wrapper} style={{ position: "relative" }}>
      <button type="button" aria-haspopup="menu" aria-expanded={open} onClick={() => setOpen((o) => !o)} style={barButton}>
        + Neu
      </button>
      {open && (
        <div role="menu" style={{ ...popoverStyle, left: 0, padding: "4px 0" }}>
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

type TreeProps = Omit<WorldExplorerProps, "onOpenWizard">;

/** The places as one `details` each. Its state (search, opened places) lasts only while the tree mode shows. */
function WorldTree({ info, headers, matrices, matricesLoaded, matricesFailed, selected, shownMatrix, revealToken, onSelect }: TreeProps) {
  const [query, setQuery] = useState("");
  // Without the overview every header would count as a room: the places then keep plain id order. The warps are not needed here, so a new warp list renders nothing.
  const places = useMemo(() => buildPlaces(headers, matricesLoaded ? matrices : []), [headers, matrices, matricesLoaded]);
  const shown = useMemo(() => filterPlaces(places, query), [places, query]);
  // The places the user (or a selection) opened; a search opens every place it leaves.
  const [openKeys, setOpenKeys] = useState<ReadonlySet<string>>(() => {
    const place = selected === null ? undefined : placeOf(places, selected);
    return new Set(place ? [place.key] : []);
  });
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
        {shown.map((place) => (
          <details key={place.key} data-place-key={place.key} open={query !== "" || openKeys.has(place.key)} onToggle={(e) => onToggle(place.key, e.currentTarget.open)}>
            <summary style={{ cursor: "pointer", lineHeight: 1.6 }}>
              <span data-place-chip="" style={{ display: "inline-block", width: 10, height: 10, borderRadius: 2, marginRight: 6, background: placeColor(place.key) }} />
              {place.name}
              <span style={{ float: "right", color: GREY, fontSize: 12 }}>{place.headers.length}</span>
            </summary>
            <ul style={{ listStyle: "none", margin: 0, padding: "0 0 4px 16px" }}>
              {place.headers.map((entry) => (
                <HeaderRow key={entry.header.id} entry={entry} selected={entry.header.id === selected} marked={showsMatrix(entry.header, shownMatrix)} showRooms={matricesLoaded} onSelect={onSelect} />
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
