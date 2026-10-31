import { useId, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { errorText, freeFlag } from "../api";
import type { FlagArea, FlagKind, ProjectInfo } from "../api";
import { Button, Picker, Tooltip } from "../ui/primitives";
import { NONE_ITEM, areaWord, flagRows, poolEmptyText, scriptsNotIndexedText } from "./flagWords";
import { useFlagList } from "./useFlagList";
import styles from "./FlagPicker.module.css";

/** The areas in the order the filter offers them (only those the list has). */
const AREA_ORDER: readonly FlagArea[] = ["map_local", "general", "hidden_items", "trainer", "system", "daily", "temp", "object_gfx", "script_local", "outside"];

/** At most this many rows are listed; a search or an area reaches every number (a full list has 4 097 flags). */
const ROW_LIMIT = 1000;

export type FlagPickerProps = {
  open: boolean;
  kind: FlagKind;
  /** The number the field holds: „Im Verzeichnis zeigen“ shows it. */
  value: number;
  onOpenChange: (open: boolean) => void;
  /** The chosen number; 0 („Keins (0)“) is a choice like any other. */
  onPick: (number: number) => void;
  /** Opens the list of flags and variables on a number; without it the picker has no such button. */
  onShowInDirectory?: (number: number) => void;
  project: ProjectInfo | null;
  revision: number;
};

/**
 * Picks a flag or a variable: „Keins (0)“ first, then every number the game names or the project uses, found by own name, pret name, an
 * alias or the number; the areas filter. The foot takes the smallest free number of the pool („Freies Flag nehmen“, locked with the reason
 * while the scripts are not adopted or the pool is empty) and leads to the list of flags and variables. Reads the list only while open.
 */
export function FlagPicker({ open, kind, value, onOpenChange, onPick, onShowInDirectory, project, revision }: FlagPickerProps) {
  // Read anew on every opening: the lock of „Freies Flag nehmen“ and the pool follow an adoption of the scripts or a save at once.
  const { list, error } = useFlagList(kind, { project: open ? project : null, revision, fresh: true });
  const [area, setArea] = useState<FlagArea | "all">("all");
  // Why the last „Freies Flag nehmen“ was refused (a pool emptied since the list was read).
  const [refusal, setRefusal] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // Every opening starts with all areas and without an old refusal.
  const [wasOpen, setWasOpen] = useState(open);
  if (open !== wasOpen) {
    setWasOpen(open);
    if (open) {
      setArea("all");
      setRefusal(null);
    }
  }

  // „Keins (0)“ first in every area, then the numbers; the picker's search filters both (the first hit is what Enter picks).
  const items = useMemo(() => {
    if (list === null) return [NONE_ITEM];
    return [NONE_ITEM, ...flagRows(area === "all" ? list : { ...list, entries: list.entries.filter((e) => e.area === area) })];
  }, [list, area]);
  const filters = useMemo(
    () => [{ value: "all", label: "Alle" }, ...AREA_ORDER.filter((a) => list?.entries.some((e) => e.area === a)).map((a) => ({ value: a, label: areaWord(a) }))],
    [list],
  );

  const locked = list === null ? null : !list.scripts_indexed ? scriptsNotIndexedText(kind) : list.pool_free === 0 ? poolEmptyText(kind) : null;
  const why = refusal ?? locked;
  const take = async () => {
    setBusy(true);
    try {
      const free = await freeFlag(kind);
      onPick(free.number);
      onOpenChange(false);
    } catch (e) {
      setRefusal(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const showInDirectory = (show: (number: number) => void) => {
    onOpenChange(false);
    // On the tick after the close request, so the dialog's focus return does not run into the list it opens.
    setTimeout(() => show(value), 0);
  };
  const status = refusal ?? error ?? (list === null && project !== null ? "Lädt …" : null);

  return (
    <Picker
      open={open}
      onOpenChange={onOpenChange}
      title={kind === "flag" ? "Flag wählen" : "Variable wählen"}
      placeholder="Name oder Nummer"
      items={items}
      onPick={(item) => onPick(Number(item.id))}
      filters={filters.length > 1 ? filters : undefined}
      filter={area}
      onFilter={(next) => setArea(next as FlagArea | "all")}
      limitPerGroup={ROW_LIMIT}
      footer={
        <div className={styles.footer} data-flag-picker={kind}>
          {status !== null && (
            <span role="status" className={refusal !== null || error !== null ? styles.refusal : styles.status}>
              {status}
            </span>
          )}
          <div className={styles.buttons}>
            <LockableButton why={why} waiting={list === null || busy} onClick={() => void take()}>
              {kind === "flag" ? "Freies Flag nehmen" : "Freie Variable nehmen"}
            </LockableButton>
            {onShowInDirectory && (
              <Button kind="quiet" size="s" onClick={() => showInDirectory(onShowInDirectory)}>
                Im Verzeichnis zeigen
              </Button>
            )}
          </div>
        </div>
      }
    />
  );
}

/** A quiet button that may be locked: focusable all the same, its reason the description and the tooltip. `waiting` locks it without one. */
function LockableButton({ why, waiting, onClick, children }: { why: string | null; waiting: boolean; onClick: () => void; children: ReactNode }) {
  const noteId = useId();
  const off = waiting || why !== null;
  const button = (
    <Button kind="quiet" size="s" aria-disabled={off || undefined} aria-describedby={why !== null ? noteId : undefined} onClick={() => !off && onClick()}>
      {children}
    </Button>
  );
  if (why === null) return button;
  return (
    <>
      <Tooltip content={why}>{button}</Tooltip>
      <span id={noteId} hidden>
        {why}
      </span>
    </>
  );
}
