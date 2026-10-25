import { memo, useEffect, useMemo, useRef, useState } from "react";
import type { HeaderSummary, ProjectInfo } from "../api";
import { FindingMark } from "../checks/FindingMark";
import type { MarkSeverity } from "../checks/FindingMark";
import type { Depth, ListSort } from "../ui/frame/shellState";
import { ListRow, TextField, useRovingList } from "../ui/primitives";
import { filterHeaders, showsMatrix } from "./headers";
import { ShownMark } from "./ShownMark";
import { TERMS, id4, techLabel } from "./terms";
import { sortHeaders } from "./worldSort";
import styles from "./WorldExplorer.module.css";

interface Props {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  selected: number | null;
  /** The matrix the scene shows: headers using it are marked. */
  shownMatrix: number | null;
  /** Bumped by every link to a header: the list scrolls to the selection again, also when it was selected already. */
  revealToken: number;
  /** Both depths write the number and the code into every row; Tüfteln adds the matrix and speaks of headers. */
  depth: Depth;
  sort: ListSort;
  onSelect: (h: HeaderSummary) => void;
  /** The worst error or warning the checks found per header (`headerMarks`); none: no marks. */
  marks?: ReadonlyMap<number, MarkSeverity>;
}

/** One row of the list. Memoised: a selection or search change renders the rows whose props changed, not all of them. */
const HeaderRow = memo(function HeaderRow({
  header: h,
  selected,
  marked,
  tinker,
  mark,
  onSelect,
}: {
  header: HeaderSummary;
  selected: boolean;
  marked: boolean;
  tinker: boolean;
  mark: MarkSeverity | undefined;
  onSelect: (h: HeaderSummary) => void;
}) {
  const name = h.location_name || h.internal_name;
  return (
    <ListRow
      label={name}
      // The number and the code tell the rows of one place apart (eight rooms of Zweiblattdorf all read "Zweiblattdorf").
      tech={`${id4(h.id)} · ${h.internal_name}`}
      selected={selected}
      onSelect={() => onSelect(h)}
      // Building speaks of the game: the number and the code, no technical words.
      title={
        tinker
          ? `${techLabel("header", h.id)} ${h.location_name} ${h.internal_name}, ${techLabel("matrix", h.matrix_id)}${marked ? ` (${TERMS.matrix.label} wird angezeigt)` : ""}`
          : `${name} · Nummer ${id4(h.id)} · ${h.internal_name}`
      }
      dataAttrs={{ "data-header-id": h.id }}
      trailing={
        mark || marked || tinker ? (
          <>
            {mark && <FindingMark severity={mark} />}
            {marked && <ShownMark />}
            {tinker && <span className={styles.matrixTag}>M{h.matrix_id}</span>}
          </>
        ) : undefined
      }
    />
  );
});

/**
 * Memoised, and its rows too: it renders when the list, the selection, the matrix shown, the
 * reveal token or the callback change, not when the scene is hovered or a message arrives.
 * `onSelect` must keep its identity for that.
 */
export const HeaderList = memo(function HeaderList({ info, headers, selected, shownMatrix, revealToken, depth, sort, onSelect, marks }: Props) {
  const [query, setQuery] = useState("");
  const sorted = useMemo(() => sortHeaders(headers, sort), [headers, sort]);
  const shown = useMemo(() => filterHeaders(sorted, query), [sorted, query]);
  // The host is always rendered, so the arrow keys work after a search and after switching modes.
  const list = useRef<HTMLDivElement>(null);
  useRovingList(list);
  // The selection and token last scrolled to, so a search that hides the selection later does not pull it back.
  const revealed = useRef<string | null>(null);
  // A newly selected header (also one chosen by a link) is scrolled into view; a search that hides
  // it is cleared first.
  useEffect(() => {
    if (selected === null) {
      revealed.current = null;
      return;
    }
    const key = `${selected}:${revealToken}`;
    if (revealed.current === key) return;
    // Looked up by id when needed, instead of a ref callback per row.
    const row = list.current?.querySelector(`[data-header-id="${selected}"]`);
    if (row) {
      revealed.current = key;
      row.scrollIntoView({ block: "nearest" });
    } else if (query !== "") {
      setQuery("");
    }
  });
  return (
    <div className={styles.column}>
      <div className={styles.top}>
        {info && (
          <p className={styles.caption}>
            {info.header_count} {depth === "tuefteln" ? TERMS.header.plural : info.header_count === 1 ? TERMS.place.label : TERMS.place.plural}
          </p>
        )}
        <TextField type="search" label={depth === "tuefteln" ? "Header suchen" : "Orte suchen"} placeholder="Ort, Raum oder Kürzel suchen" value={query} onChange={setQuery} onCommit={setQuery} />
      </div>
      <div ref={list} role="listbox" aria-label={depth === "tuefteln" ? TERMS.header.plural : TERMS.place.plural} className={styles.host}>
        {shown.map((h) => (
          <HeaderRow key={h.id} header={h} selected={h.id === selected} marked={showsMatrix(h, shownMatrix)} tinker={depth === "tuefteln"} mark={marks?.get(h.id)} onSelect={onSelect} />
        ))}
      </div>
    </div>
  );
});
