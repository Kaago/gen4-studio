import { Fragment, useEffect, useRef, useState } from "react";
import { getEvents, updateEvent } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import type { AssetId, BgDto, DataRowDto, EventKind, EventsDto, HeaderSummary, ObjectDto, ProjectInfo, WarpDto } from "../../api";
import { FlagField } from "../../flags/FlagField";
import { Button, Checkbox, InspectorRow, Kbd, Notice, NumberField, Reference, SelectField } from "../../ui/primitives";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import type { EventData } from "../../world/events";
import { KIND_LABELS, eventTitle, lockedText } from "../../world/events";
import type { NavTarget } from "../navigation";
import { id4, termFor, termLabel } from "../terms";
import { AddEvent } from "./AddEvent";
import { DialogSection } from "./DialogSection";
import { dialogTitle } from "../dialog/dialogModel";
import type { DialogTarget } from "../dialog/dialogModel";
import type { WizardRequest } from "../WorkbenchContext";
import type { StoryRecipeRequest } from "../../stories/recipes/storyRecipeModel";
import { useStoryHint } from "../../stories/useStoryHint";
import styles from "./Editor.module.css";
import { EVENT_FIELDS, eventBackend, eventFieldsAt, movementOptions } from "./eventFields";
import type { EventFieldKey } from "./eventFields";
import { isDirty } from "./dirtyAssets";
import { EditSession } from "./headerCommit";
import { HeaderPicker } from "./HeaderPicker";
import { FigureLookField } from "./FigureLookField";
import { EVENT_INSPECTOR_KIND, InspectorHead } from "./InspectorHead";
import { breakpointLock, hasBreakpoint, toggleBreakpoint } from "./eventBreakpoint";
import type { EventLive } from "./eventBreakpoint";
import { destinationSpec, referenceFor } from "./referenceFor";
import { ADOPT_HINT, headerOfEventFile, openScriptNumber } from "./scriptNav";
import { TechnicalSection } from "./TechnicalSection";
import { TrainerField } from "./TrainerField";
import { WarpField } from "./WarpField";
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
  /** The open project: the flag and variable fields read their names from it (none: they show the numbers). */
  project?: ProjectInfo | null;
  /** Opens the dialog editor of a figure or a sign (the field „Dialog“ in Erzählen). */
  openDialogEditor: (target: DialogTarget) => void;
  /** Changes whenever texts or scripts may have changed (a text written, undo, redo, a script saved): the field „Dialog“ reads again. */
  dialogRevision?: number;
  /** Opens a wizard (a door's „Tür verbinden…“ in Erzählen opens „Tür verbinden“ with the door as door A); absent: no button. */
  openWizard?: (request: WizardRequest) => void;
  /** Opens a step recipe (a figure without a script: „Dialog anlegen…“ is „Gespräch schaltet weiter“ with it); absent: no button. */
  openStoryRecipe?: (request: StoryRecipeRequest) => void;
}

/** Sentences under fields in Bauen: what the flag of a figure and the value of a trigger do. */
const BAUEN_HINTS: Partial<Record<EventFieldKey, string>> = {
  hidden_flag: "Die Figur ist da, solange das Flag 0 ist.",
  value: "Der Auslöser feuert, wenn die Variable diesen Wert hat.",
};

/**
 * Editable form of one event: its fields by class, a warp's destination header as a reference with a picker,
 * add and delete. Erzählen shows only the fields from Erzählen, with pickers instead of numbers (a figure's look, movement,
 * facing and trainer; a door's destination and door), and for a trigger and a sign what comes later. Each committed field
 * is one undoable change. Render it with `key` per (file, class, index): all reads and writes of an instance belong to
 * that one event (see `EditSession`). A locked event file is shown with its reason and cannot be changed.
 */
export function EventEditor({ file, kind, index, context, depth, live }: { file: number; kind: EventKind; index: number; context: EventEditorContext; depth: Depth; live?: EventLive }) {
  const { headers, identify, dirty, family, revision, onChanged, onAdd, onDelete, goTo, selected, scriptsAdopted, onOpenScript, onAddScript, trainers = null, needTrainers, onError, project = null, openDialogEditor, dialogRevision = 0, openWizard, openStoryRecipe } = context;
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
    if (!(await session.current?.commit("script", termLabel("script"), n))) return;
    onOpenScript(owner.script_file_id);
  };

  // The event in the words of the ROM: its class and index, and the event file it is in.
  const ident = `${KIND_LABELS[kind]} ${index} · ${termLabel("eventFile")} ${id4(file)}`;
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
  const narrating = !depthAtLeast(depth, "bauen");
  const specs = eventFieldsAt(kind, family, depth);
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
          {specs.map((spec) => {
            const value = EVENT_FIELDS.value(event, spec.key);
            // An object's script may start a trainer battle: the field „Trainer“ follows the script.
            const trainerField =
              kind === "object" && spec.key === "script" && family !== "Dp" ? (
                <TrainerField script={value} family={family} trainers={trainers} needTrainers={needTrainers} goTo={goTo} onPick={pickTrainer} disabled={readOnly} />
              ) : null;
            if (narrating) {
              // Erzählen: the script only as its trainer, the look as a sprite, the door below the destination.
              if (spec.key === "script") return <Fragment key={spec.key}>{trainerField}</Fragment>;
              if (spec.key === "dest_warp_id") return null;
              if (spec.key === "graphics_id") {
                return <FigureLookField key={spec.key} value={value} dir={EVENT_FIELDS.value(event, "dir")} disabled={readOnly} onCommit={(n) => commit(spec.key, spec.label, n)} />;
              }
            }
            const hint = !depthAtLeast(depth, "tuefteln") ? BAUEN_HINTS[spec.key] : undefined;
            const pick = spec.pick;
            if (pick !== undefined) {
              // A flag or a variable: its name with the picker (and the number in Tüfteln); 0 is none and is written like any number.
              return (
                <Fragment key={spec.key}>
                  <FlagField
                    kind={pick}
                    label={spec.label}
                    value={value}
                    onCommit={(n) => commit(spec.key, spec.label, n)}
                    disabled={readOnly}
                    depth={depth}
                    project={project}
                    revision={revision}
                    source="events"
                    onShowInDirectory={(number) => goTo({ kind: "flag", flagKind: pick, number })}
                    field={spec.key}
                    hint={hint}
                  />
                  <StoryHintLine kind={pick} number={value} project={project} revision={revision} value={kind === "coord" ? EVENT_FIELDS.value(event, "value") : undefined} />
                </Fragment>
              );
            }
            const options = narrating && spec.key === "movement_type" ? movementOptions(value) : spec.options;
            return (
              <Fragment key={spec.key}>
                <InspectorRow label={spec.label} hint={hint}>
                  {options ? (
                    <SelectField label={spec.label} value={value} options={options} disabled={readOnly} onChange={(next) => commit(spec.key, spec.label, next)} />
                  ) : (
                    <div className={styles.inline}>
                      <div className={styles.grow}>
                        <NumberField label={spec.label} value={value} min={spec.min} max={spec.max} disabled={readOnly} onCommit={(n) => commit(spec.key, spec.label, n)} showBase={depthAtLeast(depth, "tuefteln")} />
                      </div>
                      {spec.key === "script" && (
                        <>
                          <Button kind="quiet" size="s" disabled={!scriptsAdopted} onClick={openScript} title={`Öffnet das ${termLabel("script")} in der Scriptdatei`}>
                            Öffnen
                          </Button>
                          <Button
                            kind="quiet"
                            size="s"
                            disabled={!scriptsAdopted || readOnly}
                            onClick={() => void newScript()}
                            title={`Legt ein neues ${termLabel("script")} in der ${termLabel("scriptFile")} des ${depthAtLeast(depth, "tuefteln") ? "Headers" : "Ortes"} an und trägt die Nummer ein`}
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
          {!scriptsAdopted && !narrating && specs.some((s) => s.key === "script") && (
            <p role="note" className={styles.note}>
              {ADOPT_HINT}
            </p>
          )}
          {/* The raw height is a Bauen row: Erzählen does not show numbers it cannot change. */}
          {!narrating && (kind === "object" || kind === "bg") && (
            <InspectorRow label="y" hint="Nur lesbar: Die Höhe lässt sich hier nicht ändern; sie bleibt unverändert erhalten.">
              <span className={styles.mono}>{(event as ObjectDto | BgDto).y}</span>
            </InspectorRow>
          )}
        </div>
        {/* Erzählen shows no flag field: the story of the figure's flag or the trigger's variable stands in its place. */}
        {narrating && kind === "object" && <StoryHintLine kind="flag" number={EVENT_FIELDS.value(event, "hidden_flag")} project={project} revision={revision} />}
        {narrating && kind === "coord" && <StoryHintLine kind="var" number={EVENT_FIELDS.value(event, "var")} project={project} revision={revision} value={EVENT_FIELDS.value(event, "value")} none="Keine Geschichte führt die Variable dieses Auslösers." />}
        {narrating && (kind === "object" || kind === "bg") && owner && (
          <DialogSection
            target={{ header: owner.id, eventFile: file, kind, index, title: dialogTitle(kind, index) }}
            revision={revision + dialogRevision}
            onEdit={openDialogEditor}
            onError={onError}
            figure={kind === "object" ? { header: owner.id, local_id: (event as ObjectDto).local_id } : undefined}
            onStoryRecipe={openStoryRecipe}
          />
        )}
        {/* Play mode: a breakpoint halts the game when the figure's flag or the trigger's variable changes; not an edit, no undo step. */}
        {live?.active && (
          <div data-live-breakpoint={hasBreakpoint(live, { file, kind, index }) ? "on" : ""}>
            <InspectorRow label="Haltepunkt" hint={breakpointLock(kind, event) ?? undefined}>
              <Checkbox
                label={kind === "coord" ? "Anhalten, wenn sich die Variable ändert" : "Anhalten, wenn sich das Flag ändert"}
                checked={hasBreakpoint(live, { file, kind, index })}
                disabled={breakpointLock(kind, event) !== null}
                onChange={() => toggleBreakpoint(live, { file, kind, index }, event)}
              />
            </InspectorRow>
          </div>
        )}
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
        {warp && narrating && (
          <WarpField
            destHeader={destHeader}
            value={warp.dest_warp_id}
            disabled={readOnly}
            onCommit={(n) => commit("dest_warp_id", "Tür", n)}
            onError={onError}
            self={owner ? { header: owner.id, warp: index } : undefined}
            onConnect={owner && openWizard ? () => openWizard({ kind: "connect_doors", prefill: { a: { header: owner.id, warp: index } } }) : undefined}
          />
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

/**
 * „Geschichte „X“: {Schrittname}“ under a figure's visibility flag or a trigger's variable when a story holds it (`useStoryHint`); `none`
 * is said when it is known that none does.
 */
function StoryHintLine({ kind, number, project, revision, value, none }: { kind: "flag" | "var"; number: number; project: ProjectInfo | null; revision: number; value?: number; none?: string }) {
  const hint = useStoryHint(kind, number, { project, revision, value });
  if (hint === undefined || (hint === null && none === undefined)) return null;
  return hint === null ? (
    <p className={styles.note}>{none}</p>
  ) : (
    <p className={styles.note} data-story-hint="">
      {hint}
    </p>
  );
}
