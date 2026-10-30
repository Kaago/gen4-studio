import { Fragment, useEffect, useRef, useState } from "react";
import { getEvents, updateEvent } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import type { AssetId, BgDto, DataRowDto, EventKind, EventsDto, HeaderSummary, ObjectDto, WarpDto } from "../../api";
import { Button, InspectorRow, Kbd, Notice, NumberField, Reference, SelectField } from "../../ui/primitives";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import type { EventData } from "../../world/events";
import { KIND_LABELS, eventTitle, lockedText } from "../../world/events";
import type { NavTarget } from "../navigation";
import { TERMS, id4, termFor } from "../terms";
import { AddEvent } from "./AddEvent";
import styles from "./Editor.module.css";
import { EVENT_FIELDS, eventBackend, eventFieldSpecs } from "./eventFields";
import { isDirty } from "./dirtyAssets";
import { EditSession } from "./headerCommit";
import { HeaderPicker } from "./HeaderPicker";
import { EVENT_INSPECTOR_KIND, InspectorHead } from "./InspectorHead";
import { destinationSpec, referenceFor } from "./referenceFor";
import { ADOPT_HINT, headerOfEventFile, openScriptNumber } from "./scriptNav";
import { TechnicalSection } from "./TechnicalSection";
import { TrainerField } from "./TrainerField";
import { scriptTrainerLabel, trainerOfScript, trainerPatch } from "../trainerScript";
import type { HeaderIdentity } from "../headerIdentity";

/** What the event editor needs from App besides the event's file, class and index. */
export interface EventEditorContext {
  headers: HeaderSummary[];
  /** What the world tree calls a header (`WorkbenchWorld.identify`): Bauen names a door's destination with it in the head. */
  identify?: (header: HeaderSummary) => HeaderIdentity;
  /** The assets with unsaved changes: the head says "Ungespeichert" while the event file is one. */
  dirty: readonly AssetId[];
  /** Game family of the project ("Dp", "Pt", "Hgss"): decides which fields exist. */
  family: string | null;
  /** Changes whenever events may have changed behind the editor's back (a drag, undo, redo, add, delete). */
  revision: number;
  /** A field was written: the scene shows the file's events again and the dirty count follows. */
  onChanged: (eventFile: number) => void | Promise<void>;
  /** Adds an event of a class on the tile last hovered and selects it. */
  onAdd: (kind: EventKind) => void | Promise<void>;
  /** Asks for confirmation, then deletes the event. */
  onDelete: (eventFile: number, kind: EventKind, index: number) => void | Promise<void>;
  /** Follows a link (a warp's destination header). */
  goTo: (target: NavTarget) => void;
  /** The selected header: when several headers share the event file, its script file is the one the event's scripts are looked up in. */
  selected: number | null;
  /** Whether rotom has taken the scripts over; without it the script buttons only show a hint. */
  scriptsAdopted: boolean;
  /** Opens a script file in the script panel, at a 1-based line. */
  onOpenScript: (file: number, line?: number) => void;
  /** Appends an empty script to the script file and resolves to its number (null: it failed and was reported). */
  onAddScript: (file: number) => Promise<number | null>;
  /** The trainer rows of the project (null while not read): the object field „Trainer“ names and offers them. */
  trainers?: readonly DataRowDto[] | null;
  /** Asks for the trainer rows (the field shows a trainer, or its picker opens). */
  needTrainers?: () => void;
  onError: (e: unknown) => void;
}


/**
 * Editable form of one event: its fields by class, a warp's destination header as a reference with a picker,
 * add and delete. Each committed field is one undoable change. Render it with `key` per (file, class, index): all
 * reads and writes of an instance belong to that one event (see `EditSession`). A locked event file is shown with
 * its reason and cannot be changed.
 */
export function EventEditor({ file, kind, index, context, depth }: { file: number; kind: EventKind; index: number; context: EventEditorContext; depth: Depth }) {
  const { headers, identify, dirty, family, revision, onChanged, onAdd, onDelete, goTo, selected, scriptsAdopted, onOpenScript, onAddScript, trainers = null, needTrainers, onError } = context;
  const reset = useReset();
  const [event, setEvent] = useState<EventData | null>(null);
  const [locked, setLocked] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);
  // Bumped when an edit is refused, so the fields drop the rejected text they still show.
  const [refused, setRefused] = useState(0);
  const callbacks = useRef({ onChanged, onError });
  callbacks.current = { onChanged, onError };
  const session = useRef<EditSession<EventData> | null>(null);

  useEffect(() => {
    const onEvents = (events: EventsDto) => setLocked(events.locked);
    const backend = eventBackend(file, kind, { getEvents, update: updateEvent, onEvents });
    const s = new EditSession<EventData>(index, backend, EVENT_FIELDS, {
      onState: setEvent,
      onRefused: () => setRefused((n) => n + 1),
      onChanged: () => callbacks.current.onChanged(file),
      onError: (e) => callbacks.current.onError(e),
    });
    session.current = s;
    return () => s.dispose();
  }, [file, kind, index]);

  useEffect(() => {
    void session.current?.load();
  }, [file, kind, index, revision]);

  const commit = (key: string, label: string, value: number) => void session.current?.commit(key, label, value);
  // A picked trainer: its script (a partner stays a partner), and the sightline of a figure that had none, in one undoable change.
  const pickTrainer = (id: number) => {
    if (event === null) return;
    const object = event as ObjectDto;
    const patch = trainerPatch(id, object, trainerOfScript(object.script)?.partner ?? false);
    void session.current?.commitAll(patch as Record<string, number>, "Trainer");
  };

  // The header whose script file the event's scripts live in (headers may share an event file).
  const owner = headerOfEventFile(headers, file, selected);
  const openScript = () => {
    if (event === null) return;
    if (!owner) return onError(`Kein ${termFor("header", depth)} nutzt Eventdatei ${id4(file)}`);
    void openScriptNumber(owner.id, EVENT_FIELDS.value(event, "script"), onOpenScript, onError);
  };
  const newScript = async () => {
    if (!owner) return onError(`Kein ${termFor("header", depth)} nutzt Eventdatei ${id4(file)}`);
    const n = await onAddScript(owner.script_file_id);
    if (n === null) return;
    // The number goes through the edit path like a typed one: undoable, and the event is dirty.
    if (!(await session.current?.commit("script", TERMS.script.label, n))) return;
    onOpenScript(owner.script_file_id);
  };

  // The event in the words of the ROM: its class and index, and the event file it is in.
  const ident = `${KIND_LABELS[kind]} ${index} · ${TERMS.eventFile.label} ${id4(file)}`;
  if (event === null) {
    return (
      <div className={styles.panel}>
        <InspectorHead kind={EVENT_INSPECTOR_KIND[kind]} title={`${KIND_LABELS[kind]} ${index}`} ident={ident} unsaved={isDirty(dirty, "Event", file)} depth={depth} />
      </div>
    );
  }
  const readOnly = locked !== null;
  const warp = kind === "warp" ? (event as WarpDto) : null;
  const destHeader = warp ? headers.find((h) => h.id === warp.dest_header_id) : undefined;
  const destinationField = destinationSpec(depth);
  const destination = warp ? referenceFor(destinationField, warp.dest_header_id, headers) : null;
  const destinationTarget = destination?.target ?? null;
  // The head says what the scene hierarchy says: a door's destination by the world tree's name (Tüfteln: number and code), a trainer
  // figure by its trainer ("Objekt 0 · Wanderer Jens").
  const destName = destHeader === undefined ? undefined : depthAtLeast(depth, "tuefteln") ? destHeader.internal_name : identify?.(destHeader).name;
  const trainer = kind === "object" ? scriptTrainerLabel(trainers, (event as ObjectDto).script) : null;
  const headTitle = trainer !== null ? `Objekt ${index} · ${trainer}` : eventTitle(kind, event, depth, destName);

  return (
    <div className={styles.panel}>
      <InspectorHead
        kind={EVENT_INSPECTOR_KIND[kind]}
        title={headTitle}
        ident={ident}
        unsaved={isDirty(dirty, "Event", file)}
        locked={readOnly}
        actions={[
          { label: "Löschen", detail: <Kbd keys="Entf" />, disabled: readOnly, onSelect: () => void onDelete(file, kind, index) },
          "separator",
          // Not the event alone: the event file it is in, as one undo step (a locked file is refused with its reason in the dialog).
          resetEntry({ kind: "event_file", index: file }, reset.status, reset.open, "Eventdatei auf Original zurücksetzen…"),
        ]}
        depth={depth}
      />
      <div className={styles.body}>
        {locked !== null && (
          <div className={styles.pad}>
            <Notice kind="warning" title="Eventdatei gesperrt">
              {lockedText(locked)}
            </Notice>
          </div>
        )}
        <div key={refused}>
          {eventFieldSpecs(kind, family).map((spec) => {
            const value = EVENT_FIELDS.value(event, spec.key);
            // An object's script may start a trainer battle: the field „Trainer“ follows the script.
            const trainerField =
              kind === "object" && spec.key === "script" && family !== "Dp" ? (
                <TrainerField script={value} family={family} trainers={trainers} needTrainers={needTrainers} goTo={goTo} onPick={pickTrainer} disabled={readOnly} />
              ) : null;
            return (
              <Fragment key={spec.key}>
                <InspectorRow label={spec.label}>
                  {spec.options ? (
                    <SelectField label={spec.label} value={value} options={spec.options} disabled={readOnly} onChange={(next) => commit(spec.key, spec.label, next)} />
                  ) : (
                    <div className={styles.inline}>
                      <div className={styles.grow}>
                        <NumberField label={spec.label} value={value} min={spec.min} max={spec.max} disabled={readOnly} onCommit={(n) => commit(spec.key, spec.label, n)} showBase={depthAtLeast(depth, "tuefteln")} />
                      </div>
                      {spec.key === "script" && (
                        <>
                          <Button kind="quiet" size="s" disabled={!scriptsAdopted} onClick={openScript} title={`Öffnet das ${TERMS.script.label} in der Scriptdatei`}>
                            Öffnen
                          </Button>
                          <Button
                            kind="quiet"
                            size="s"
                            disabled={!scriptsAdopted || readOnly}
                            onClick={() => void newScript()}
                            title={`Legt ein neues ${TERMS.script.label} in der ${TERMS.scriptFile.label} des ${depthAtLeast(depth, "tuefteln") ? "Headers" : "Ortes"} an und trägt die Nummer ein`}
                          >
                            Neues Script
                          </Button>
                        </>
                      )}
                    </div>
                  )}
                </InspectorRow>
                {trainerField}
              </Fragment>
            );
          })}
          {!scriptsAdopted && eventFieldSpecs(kind, family).some((s) => s.key === "script") && (
            <p role="note" className={styles.note}>
              {ADOPT_HINT}
            </p>
          )}
          {(kind === "object" || kind === "bg") && (
            <InspectorRow label="y" hint="Nur lesbar: Die Höhe lässt sich hier nicht ändern; sie bleibt unverändert erhalten.">
              <span className={styles.mono}>{(event as ObjectDto | BgDto).y}</span>
            </InspectorRow>
          )}
        </div>
        {warp && destination && (
          <InspectorRow label="Ziel" hint={destinationTarget === null ? "Die Tür führt ins Leere. Wähle ein Ziel oder lösche die Tür." : undefined}>
            <div className={styles.inline}>
              {destinationTarget !== null ? (
                <Reference area={destination.area} label={destination.label} tech={destination.tech} onOpen={() => goTo(destinationTarget)} />
              ) : (
                <span className={styles.error}>{`${destination.label} (${destination.tech})`}</span>
              )}
              <Button kind="quiet" size="s" aria-label={`${destinationField.label} ändern…`} disabled={readOnly} onClick={() => setPicking(true)}>
                Ändern…
              </Button>
            </div>
          </InspectorRow>
        )}
        {warp && (
          <HeaderPicker
            open={picking}
            onOpenChange={setPicking}
            headers={headers}
            title={`${destinationField.label} wählen`}
            onPick={(id) => commit("dest_header_id", destinationField.label, id)}
          />
        )}
        {depthAtLeast(depth, "tuefteln") && <TechnicalSection model={{ kind: "event", eventFile: file, eventKind: kind, index }} source={{ kind: "event", eventKind: kind, event, family }} />}
        <div className={styles.actions}>
          <AddEvent disabled={readOnly} onAdd={(k) => void onAdd(k)} />
        </div>
        <p className={styles.note}>
          Ziehen in der 3D-Ansicht verschiebt das Event um ganze Kacheln, Esc bricht das Ziehen ab. Neue Events entstehen auf der zuletzt berührten Kachel; Entf löscht das gewählte Event nach Rückfrage.
        </p>
      </div>
    </div>
  );
}
