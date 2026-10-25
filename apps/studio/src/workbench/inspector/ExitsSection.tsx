import type { HeaderSummary } from "../../api";
import type { Depth } from "../../ui/frame/shellState";
import { Button, Reference, Section } from "../../ui/primitives";
import { KIND_LABELS, lockedText } from "../../world/events";
import type { NavTarget } from "../navigation";
import { TERMS } from "../terms";
import type { WizardRequest } from "../WorkbenchContext";
import { useEvents } from "../useEvents";
import styles from "./Editor.module.css";
import exits from "./ExitsSection.module.css";
import { useSectionOpen } from "./foldState";
import { destinationSpec, referenceFor } from "./referenceFor";

interface Props {
  header: HeaderSummary;
  headers: HeaderSummary[];
  /** Changes whenever events were changed behind the section's back (a drag, add, delete, undo, redo); it reads them again. */
  revision: number;
  goTo: (target: NavTarget) => void;
  onOpenWizard: (request: WizardRequest) => void;
  /** Adds a warp to the header's event file (the app decides where). */
  onAddWarp: () => void;
  onError: (e: unknown) => void;
  /** Names an unknown destination's field in the words of the depth. */
  depth: Depth;
}


/** The "Eingänge / Ausgänge" of the header editor: the warps of the header's event file, where they lead, and the ways to add one. */
export function ExitsSection({ header, headers, revision, goTo, onOpenWizard, onAddWarp, onError, depth }: Props) {
  const events = useEvents(header.event_file_id, onError, revision);
  const locked = events?.locked ?? null;
  const [open, toggle] = useSectionOpen("header:exits", true);
  return (
    <Section title={TERMS.warp.plural} count={events?.warps.length} open={open} onToggle={toggle}>
      {events === null ? (
        <p className={styles.note}>Lade …</p>
      ) : events.warps.length === 0 ? (
        <p className={styles.note}>Keine Ausgänge.</p>
      ) : (
        <ul className={exits.list}>
          {events.warps.map((w) => {
            const dest = referenceFor(destinationSpec(depth), w.dest_header_id, headers);
            const target = dest.target;
            return (
              <li key={w.index} className={exits.row}>
                <span className={exits.where}>
                  {KIND_LABELS.warp} {w.index} (x {w.x}, z {w.z})
                </span>
                {target ? (
                  <Reference area={dest.area} label={dest.label} tech={dest.tech} onOpen={() => goTo(target)} />
                ) : (
                  <span className={styles.error}>{`${dest.label} (${dest.tech})`}</span>
                )}
                <Button kind="quiet" size="s" onClick={() => goTo({ kind: "event", eventFile: header.event_file_id, eventKind: "warp", index: w.index, header: header.id })}>
                  Bearbeiten
                </Button>
              </li>
            );
          })}
        </ul>
      )}
      <div className={styles.actions}>
        <Button kind="secondary" size="s" onClick={() => onOpenWizard({ kind: "room", prefill: { outsideHeader: header.id } })}>
          {TERMS.room.label} hier anbinden…
        </Button>
        <Button kind="secondary" size="s" disabled={locked !== null} title={locked === null ? "Legt einen Warp auf der Kachel an, über der die Maus zuletzt war" : lockedText(locked)} onClick={onAddWarp}>
          Warp hinzufügen
        </Button>
      </div>
    </Section>
  );
}
