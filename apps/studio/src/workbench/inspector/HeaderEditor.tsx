import { useEffect, useRef, useState } from "react";
import { getHeader, headerFamily, newScriptFile, setHeader } from "../../api";
import type { HeaderSummary, LevelScriptDoc, MapHeader, MatrixSummary, MemberCounts, ProjectInfo, SaveScriptDto } from "../../api";
import type { Created } from "../../app/useWizards";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { renameEntry } from "../../textproject/renameEntry";
import { useTextProject } from "../../textproject/TextProjectContext";
import { useEntityIds } from "../../textproject/useEntityIds";
import { Badge, Button, InspectorRow, Section, Tooltip } from "../../ui/primitives";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import type { HeaderIdentity } from "../headerIdentity";
import type { NavTarget } from "../navigation";
import { RenameDialog } from "../RoomActions";
import type { RenameTarget } from "../RoomActions";
import { TERMS, id4, termFor } from "../terms";
import { useEvents } from "../useEvents";
import type { WizardRequest } from "../WorkbenchContext";
import { roomOf } from "../worldModel";
import type { Room, World } from "../worldModel";
import styles from "./Editor.module.css";
import { EncounterFileField } from "./EncounterFileField";
import { Field } from "./Field";
import { useSectionFolds } from "./foldState";
import { fieldValue, pickMax } from "./fields";
import type { FieldSpec } from "./fields";
import { HeaderSession } from "./headerCommit";
import { headerSections } from "./headerSections";
import type { HeaderSectionId } from "./headerSections";
import { InspectorHead } from "./InspectorHead";
import { LookField } from "./LookField";
import { PlacePicker } from "../places/PlacePicker";
import { RenamePlaceDialog } from "../places/RenamePlaceDialog";
import { LevelScriptSection } from "./LevelScriptSection";
import { SplitRoomDialog } from "./SplitRoomDialog";
import { SHARED_HINT, sharedText } from "../sharedRooms";
import { ADOPT_HINT } from "./scriptNav";
import { TechnicalSection } from "./TechnicalSection";

/** What the header editor needs from App besides the header id. */
export interface HeaderEditorContext {
  headers: HeaderSummary[];
  /** What a header is and how it is called, as the world tree says (`WorkbenchWorld.identify`): the head's kind and title. */
  identify: (header: HeaderSummary) => HeaderIdentity;
  counts: MemberCounts;
  /** Changes whenever the header table was changed behind the editor's back (undo, redo, reload). */
  revision: number;
  /** Called after a field was written, so the list, the scene and the dirty count follow. */
  onChanged: () => void | Promise<void>;
  /** Called after a place command (assign, create, rename): the header is read again and the list follows; `bank` is the location bank when it changed. */
  onPlaceChanged: (bank: number | null) => void | Promise<void>;
  /** Follows a link (area data, matrix panel, the doors of the scene). */
  goTo: (target: NavTarget) => void;
  /** Changes whenever events were changed behind the editor's back (a drag, add, delete, undo, redo): the doors are counted again. */
  eventRevision: number;
  /** Opens a wizard ("Raum hier anbinden…"). */
  onOpenWizard: (request: WizardRequest) => void;
  /** The open project (the level-script form starts over when another one is opened). */
  info: ProjectInfo | null;
  /** Whether rotom has taken the scripts over; without it the script buttons and the level-script form only show a hint. */
  scriptsAdopted: boolean;
  /** Changes when the script panel saved or closed a document (the level-script form reads its file again). */
  scriptsRevision: number;
  /** Opens a script file in the script panel (a level script as JSON). */
  onOpenScript: (file: number, line?: number) => void;
  /** The level-script form saved a document in `project`: its problems and an open document follow. */
  onLevelScriptSaved: (project: ProjectInfo | null, file: number, doc: LevelScriptDoc, dto: SaveScriptDto) => void;
  /** Whether `project` is still the open one (the form drops saves that wait while another project is opened). */
  isCurrentProject: (project: ProjectInfo | null) => boolean;
  /** A script file was added: the member counts (the pickers' limits) follow. */
  onCountsChanged: () => void | Promise<void>;
  /** The world model (null until it is read): whether a room shares its maps with other rooms (the badge „Geteilt“, „Für diesen Raum abspalten…“). */
  world: World | null;
  /** The matrices of the project: how many maps a room to split has. */
  matrices: readonly MatrixSummary[];
  /** A room was split off: the project is read again as after a wizard, and `made.header` is shown with the announcement. */
  onRoomSplit: (made: Created) => void | Promise<void>;
  /** Renames a room as the world tree does (Erzählen's row „Name“); without it the row only shows the name. */
  onRename?: (target: RenameTarget, name: string) => void;
  onError: (e: unknown) => void;
}

/** The header fields that name a file of the script archive, with what a new file of that field is. */
const SCRIPT_FIELDS: Record<string, "script" | "levelScript"> = { script_file_id: "script", level_script_id: "levelScript" };

/** „2 Türen, 1 ohne Ziel“: the doors of the header's event file and how many lead to a header the list does not know; null for none. */
function doorsText(dests: readonly number[], headers: readonly HeaderSummary[]): string | null {
  if (dests.length === 0) return null;
  const lost = dests.filter((dest) => !headers.some((h) => h.id === dest)).length;
  const doors = `${dests.length} ${dests.length === 1 ? "Tür" : "Türen"}`;
  return lost === 0 ? doors : `${doors}, ${lost} ohne Ziel`;
}

/**
 * The line of the doors where the exits list was: how many the header's event file has and how many lead nowhere, as a link that shows
 * the scene, opens its „Türen“ and focuses them (the scene's tools and „+“ add a door, the event editor sets its target).
 */
function DoorsLine({ header, headers, revision, goTo, onError }: { header: HeaderSummary; headers: readonly HeaderSummary[]; revision: number; goTo: (target: NavTarget) => void; onError: (e: unknown) => void }) {
  const events = useEvents(header.event_file_id, onError, revision);
  const text = events === null ? null : doorsText(events.warps.map((w) => w.dest_header_id), headers);
  return (
    <p className={styles.actions} data-doors="">
      {events === null ? (
        <span className={styles.note}>Lade Türen …</span>
      ) : text === null ? (
        <span className={styles.note}>Keine Türen</span>
      ) : (
        <Button kind="quiet" size="s" icon="door" title="Zeigt die Türen in „In dieser Szene“" onClick={() => goTo({ kind: "sceneGroup", group: "warp", header: header.id })}>
          {text}
        </Button>
      )}
    </p>
  );
}

/**
 * Erzählen's row „Name“: the name the head shows, and for a room „Umbenennen…“ with the world tree's dialog (an area has no rename,
 * its name stands alone).
 */
function NameRow({ name, room, onRename }: { name: string; room: Room | undefined; onRename?: (target: RenameTarget, name: string) => void }) {
  const [open, setOpen] = useState(false);
  // Counts the openings: the dialog is keyed on it, so its field starts with the current name each time.
  const [opens, setOpens] = useState(0);
  return (
    <>
      <InspectorRow label="Name">
        <div className={styles.inline} data-name-row="">
          <span>{name}</span>
          {room && onRename && (
            <Button
              kind="quiet"
              size="s"
              onClick={() => {
                setOpens((n) => n + 1);
                setOpen(true);
              }}
            >
              Umbenennen…
            </Button>
          )}
        </div>
      </InspectorRow>
      {room && onRename && (
        <RenameDialog
          key={opens}
          open={open}
          title="Raum umbenennen"
          name={room.name}
          suggestion={room.suggested}
          onClose={() => setOpen(false)}
          onRename={(next) => onRename({ kind: "room", id: room.header.id }, next)}
        />
      )}
    </>
  );
}

/**
 * Editable form of one map header; every committed field is one undoable change. Erzählen shows the one section „Ort“: the name, the
 * place, the look (`LookField`) and the music. Render it with
 * `key={id}`: all reads and writes of an instance belong to its one header (see `HeaderSession`).
 */
export function HeaderEditor({ id, context, depth }: { id: number; context: HeaderEditorContext; depth: Depth }) {
  const { headers, identify, counts, revision, onChanged, onPlaceChanged, goTo, eventRevision, onOpenWizard, info, scriptsAdopted, scriptsRevision, onOpenScript, onLevelScriptSaved, isCurrentProject, onCountsChanged, world, matrices, onRoomSplit, onRename, onError } = context;
  const reset = useReset();
  // In a text project the head names the header's id (`MAP_…`) and offers to rename it.
  const text = useTextProject();
  const entityId = useEntityIds("Map", text.active)?.[id] ?? null;
  const [header, setHeaderState] = useState<MapHeader | null>(null);
  // Bumped when an edit is refused, so the fields drop the rejected text they still show.
  const [refused, setRefused] = useState(0);
  const [renaming, setRenaming] = useState(false);
  const [splitting, setSplitting] = useState(false);
  // The form sections the user folded away (a closed section does not render its fields). Kept outside the editor: selecting another header mounts a new one.
  const folds = useSectionFolds();
  const foldKey = (section: HeaderSectionId) => `header:${section}`;
  const callbacks = useRef({ onChanged, onError });
  callbacks.current = { onChanged, onError };
  const session = useRef<HeaderSession | null>(null);

  // One session per mount. Disposing only cuts it off from this editor's state: a commit that
  // is already queued (also an uncommitted field flushed on unmount) still writes its own header.
  useEffect(() => {
    const s = new HeaderSession(
      id,
      { getHeader, setHeader },
      {
        onState: setHeaderState,
        onRefused: () => setRefused((n) => n + 1),
        onChanged: () => callbacks.current.onChanged(),
        onError: (e) => callbacks.current.onError(e),
      },
    );
    session.current = s;
    return () => s.dispose();
  }, [id]);

  // Load, and reload after undo/redo (queued behind pending commits). Keeps showing the old
  // values meanwhile.
  useEffect(() => {
    void session.current?.load();
  }, [id, revision]);

  const commit = (spec: FieldSpec, value: number) => void session.current?.commit(spec.key, spec.label, value);

  // A new script file: its number goes through the edit path like a typed one (undoable, the header is dirty).
  const createFile = async (key: string, kind: "script" | "levelScript") => {
    try {
      const n = await newScriptFile(kind);
      const label = kind === "script" ? TERMS.scriptFile.label : TERMS.levelScript.label;
      // Only a number that is in the header is worth opening (a refused write was reported; the new file stays unused).
      if (!(await session.current?.commit(key, label, n))) return;
      await onCountsChanged();
      if (kind === "script") onOpenScript(n);
    } catch (e) {
      onError(e);
    }
  };

  const summary = headers.find((h) => h.id === id);
  // The head: an area by its name, a room by its name; Bauen gives the place and the code as the ident, Tüfteln the header's number.
  const identity = summary ? identify(summary) : null;
  const headerId = `${termFor("header", depth)} ${id4(id)}`;
  const code = summary ? ` · ${summary.internal_name}` : "";
  const ident = depthAtLeast(depth, "tuefteln") ? `${headerId}${code}` : identity ? `${identity.place}${code}` : undefined;
  // A room whose maps other rooms use too: the head says so, and the room can get copies of its own.
  const room = summary && world ? roomOf(world, id) : undefined;
  const sharedRoom = room !== undefined && room.shared > 0 ? room : undefined;
  // Only a room on a matrix without a header section can be split (the kernel refuses a world map); an unknown matrix offers nothing.
  const roomMatrix = room ? matrices.find((m) => m.index === room.header.matrix_id) : undefined;
  const canSplit = sharedRoom !== undefined && roomMatrix?.has_headers === false;
  const narrating = !depthAtLeast(depth, "bauen");
  const fieldRow = (spec: FieldSpec, header: MapHeader) => {
    const script = SCRIPT_FIELDS[spec.key];
    const value = fieldValue(header, spec.key);
    const family = headerFamily(header);
    // Erzählen chooses the look by its places, with this header's map drawn in each.
    if (narrating && spec.key === "area_data_id" && summary) {
      return <LookField key={spec.key} value={value} headers={headers} template={summary} onCommit={(n) => commit(spec, n)} />;
    }
    // „Begegnungen“ names an encounter file of the Pokémon area (Diamond and Pearl have no data editors: there it stays a number).
    if (spec.key === "wild_pokemon" && family !== "DP") {
      return <EncounterFileField key={spec.key} spec={spec} value={value} family={family} headers={headers} goTo={goTo} onCommit={commit} />;
    }
    // „Ort“ is chosen by name from the location bank; Diamond and Pearl write the field itself (the kernel has no place commands there).
    if (spec.kind === "place" && summary) {
      return (
        <PlacePicker
          key={spec.key}
          header={summary}
          depth={depth}
          headers={headers}
          family={family}
          onAssigned={onPlaceChanged}
          onError={onError}
          commit={family === "DP" ? (index) => commit(spec, index) : undefined}
        />
      );
    }
    return (
      <Field
        key={spec.key}
        spec={spec}
        value={value}
        max={spec.kind === "pick" && spec.pick ? pickMax(spec.pick, counts) : undefined}
        onCommit={commit}
        showBase={depthAtLeast(depth, "tuefteln")}
        headers={headers}
        goTo={goTo}
        open={script === undefined ? undefined : { run: () => onOpenScript(value), disabled: !scriptsAdopted || (script === "levelScript" && value === 0) }}
        extra={
          script === undefined ? undefined : (
            <Button
              kind="quiet"
              size="s"
              aria-label={script === "script" ? `Neue ${TERMS.scriptFile.label} anlegen` : `Neues ${TERMS.levelScript.label} anlegen`}
              disabled={!scriptsAdopted}
              onClick={() => void createFile(spec.key, script)}
            >
              Neu
            </Button>
          )
        }
      />
    );
  };

  return (
    <div className={styles.panel}>
      <InspectorHead
        kind={identity === null ? "room" : identity.kind === "other" ? "place" : identity.kind}
        title={identity?.name ?? headerId}
        ident={ident}
        entityId={entityId}
        actions={[
          ...(summary
            ? [
                { label: `${TERMS.room.label} hier anbinden…`, onSelect: () => onOpenWizard({ kind: "room", prefill: { outsideHeader: id } }) },
                // No place (index 0) has no name to change; Diamond and Pearl have no place commands.
                { label: "Ort umbenennen…", disabled: summary.location_index === 0 || header === null || headerFamily(header) === "DP", onSelect: () => setRenaming(true) },
                ...(canSplit ? [{ label: "Für diesen Raum abspalten…", onSelect: () => setSplitting(true) }] : []),
                "separator" as const,
              ]
            : []),
          // The header as in the base ROM, field by field (the dialog lists the fields that differ).
          resetEntry({ kind: "header", id }, reset.status, reset.open),
          ...(entityId !== null ? [renameEntry(entityId, text.renameId)] : []),
        ]}
        badges={
          sharedRoom && (
            <Tooltip content={SHARED_HINT}>
              <span className={styles.shared} tabIndex={0} data-shared="">
                <Badge kind="info" icon="shared">
                  {sharedText(sharedRoom.shared)}
                </Badge>
              </span>
            </Tooltip>
          )
        }
        depth={depth}
      />
      <div className={styles.body}>
        {header === null ? (
          <p className={styles.note}>Lade {termFor("header", depth)} …</p>
        ) : (
          <div key={refused}>
            {headerSections(headerFamily(header), depth).map((section) => (
              <Section key={section.id} title={section.title} open={folds.isOpen(foldKey(section.id))} onToggle={() => folds.toggle(foldKey(section.id))}>
                {section.id === "ort" && <NameRow name={identity?.name ?? headerId} room={room} onRename={onRename} />}
                {section.specs.map((spec) => fieldRow(spec, header))}
                {section.id === "texts" && !scriptsAdopted && (
                  <p role="note" className={styles.note}>
                    {ADOPT_HINT}
                  </p>
                )}
              </Section>
            ))}
          </div>
        )}
        {summary && <DoorsLine header={summary} headers={headers} revision={eventRevision} goTo={goTo} onError={onError} />}
        {summary && header !== null && !narrating && (
          <LevelScriptSection
            // A header of another project or another level script file starts a fresh form.
            key={`${id}:${fieldValue(header, "level_script_id")}`}
            header={summary}
            levelScriptId={fieldValue(header, "level_script_id")}
            project={info}
            adopted={scriptsAdopted}
            revision={scriptsRevision}
            onOpenScript={onOpenScript}
            onCreate={() => createFile("level_script_id", "levelScript")}
            onSaved={onLevelScriptSaved}
            isCurrentProject={isCurrentProject}
            onError={onError}
            depth={depth}
            onShowFlag={(flagKind, number) => goTo({ kind: "flag", flagKind, number })}
          />
        )}
        {depthAtLeast(depth, "tuefteln") && header !== null && <TechnicalSection model={{ kind: "header", id }} source={{ kind: "header", header }} />}
      </div>
      {room && (
        <SplitRoomDialog
          open={splitting}
          header={id}
          name={room.name}
          // One filled cell is one map; with more cells the number of distinct maps is not known (the text names none).
          maps={roomMatrix?.filled === 1 ? 1 : null}
          onClose={() => setSplitting(false)}
          onSplit={(report) => onRoomSplit({ matrix: report.matrix, header: id, announcement: `Raum „${room.name}“ abgespalten` })}
          onError={onError}
        />
      )}
      {summary && <RenamePlaceDialog open={renaming} index={summary.location_index} headers={headers} onClose={() => setRenaming(false)} onRenamed={onPlaceChanged} onError={onError} />}
    </div>
  );
}
