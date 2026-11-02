import { memo, useEffect, useMemo, useRef, useState } from "react";
import type { HeaderSummary } from "../../api";
import { ListPane } from "../../ui/frame/ListPane";
import { EmptyState, TextField, TreeRow, useRovingList } from "../../ui/primitives";
import { speakersText } from "../dialog/dialogModel";
import { headerIdentity } from "../headerIdentity";
import { filterPlaces } from "../places";
import type { Place } from "../places";
import { useTextArea } from "./TextAreaContext";
import { useHeaderDialogs } from "./useHeaderDialogs";
import styles from "./TextList.module.css";

/**
 * A header of a place with how many figures speak there („2 Figuren sprechen“). Its figures are read when the row first shows in a place
 * the user opened (`counted`) and again when the data moved; the cache answers every later showing. A row a filter shows reads nothing:
 * each read waits for the session, and a filter can show nearly every header.
 */
const HeaderItem = memo(function HeaderItem({
  header,
  label,
  counted,
  selected,
  revision,
  onSelect,
}: {
  header: HeaderSummary;
  label: string;
  counted: boolean;
  selected: boolean;
  revision: number;
  onSelect: (id: number) => void;
}) {
  const { dialogs, error } = useHeaderDialogs(counted ? header.id : null, revision);
  const line = !counted ? undefined : dialogs !== null ? speakersText(dialogs.figures) : error !== null ? "Nicht lesbar" : "Lese …";
  return <TreeRow depth={1} label={label} subtitle={line} title={error ?? undefined} selected={selected} onSelect={() => onSelect(header.id)} dataAttrs={{ "data-dialog-header": header.id }} />;
});

/**
 * The list of the texts in Erzählen (`[data-dialog-list]`): the places of the world, folded, and under an opened place its headers with how
 * many figures speak there; a filter shows the matching places open, their headers without the count. A click on a header shows its
 * figures in the view. No archives and no numbers: those are Bauen's.
 */
export function DialogList({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  const { project, places, dialogHeader, dialogPlace, dialogRevision, selectDialogHeader } = useTextArea();
  const [filter, setFilter] = useState("");
  const [opened, setOpened] = useState<ReadonlySet<string>>(() => new Set());
  const tree = useRef<HTMLDivElement>(null);
  useRovingList(tree);
  const shown = useMemo(() => filterPlaces(places, filter), [places, filter]);
  const filtering = filter.trim() !== "";
  const toggle = (key: string) =>
    setOpened((before) => {
      const next = new Set(before);
      if (!next.delete(key)) next.add(key);
      return next;
    });

  // The header that is chosen (a click, a link) comes into view: a filter that hides its place is cleared and the place opened first.
  const revealed = useRef<number | null>(null);
  useEffect(() => {
    if (dialogHeader === null || dialogPlace === null) {
      revealed.current = null;
      return;
    }
    if (revealed.current === dialogHeader) return;
    if (filtering && !filterPlaces([dialogPlace], filter).some((p) => p.headers.some((h) => h.header.id === dialogHeader))) {
      setFilter("");
      return;
    }
    if (!filtering && !opened.has(dialogPlace.key)) {
      toggle(dialogPlace.key);
      return;
    }
    const element = tree.current?.querySelector(`[data-dialog-header="${dialogHeader}"]`);
    if (element) {
      revealed.current = dialogHeader;
      element.scrollIntoView({ block: "nearest" });
    }
  });

  return (
    <ListPane title="Dialoge" subtitle={project ? `${places.length} Orte` : undefined} open={open} onToggle={onToggle}>
      {project === null ? (
        <EmptyState what="Kein Projekt geöffnet." why="Öffne ein Projekt, dann stehen hier seine Orte mit ihren Figuren." />
      ) : (
        <div className={styles.column} data-dialog-list="">
          <div className={styles.top}>
            <TextField type="search" label="Orte filtern" placeholder="Ort" value={filter} onChange={setFilter} onCommit={setFilter} />
          </div>
          <div ref={tree} role="tree" aria-label="Orte" className={styles.host}>
            {shown.length === 0 && filtering && <p className={styles.note}>Kein Ort passt zum Filter.</p>}
            {shown.map((place) => (
              <PlaceGroup key={place.key} place={place} places={places} expanded={filtering || opened.has(place.key)} counted={opened.has(place.key)} onToggle={toggle} selected={dialogHeader} revision={dialogRevision} onSelect={selectDialogHeader} />
            ))}
          </div>
        </div>
      )}
    </ListPane>
  );
}

/** A place of the list: its row (a click folds or opens it) and, opened, its headers. */
function PlaceGroup({
  place,
  places,
  expanded,
  counted,
  onToggle,
  selected,
  revision,
  onSelect,
}: {
  place: Place;
  places: Place[];
  expanded: boolean;
  /** The user opened the place: its headers read how many figures speak there (a filter only shows them). */
  counted: boolean;
  onToggle: (key: string) => void;
  selected: number | null;
  revision: number;
  onSelect: (id: number) => void;
}) {
  return (
    <div role="group" aria-label={place.name}>
      <TreeRow
        depth={0}
        label={place.name}
        selected={false}
        expanded={expanded}
        onToggle={() => onToggle(place.key)}
        onSelect={() => onToggle(place.key)}
        dataAttrs={{ "data-dialog-place": place.name }}
      />
      {expanded &&
        place.headers.map(({ header }) => (
          <HeaderItem key={header.id} header={header} label={headerIdentity(places, header).name} counted={counted} selected={header.id === selected} revision={revision} onSelect={onSelect} />
        ))}
    </div>
  );
}
