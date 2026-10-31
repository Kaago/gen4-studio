import { useCallback, useState } from "react";
import type { KeyboardEvent, SyntheticEvent } from "react";
import type { WorldMeta } from "../api";
import { Dialog, Icon, Menu, Picker, TextField } from "../ui/primitives";
import type { MenuEntry, PickerItem } from "../ui/primitives";
import type { Room, WorldMap } from "./worldModel";
import styles from "./WorldExplorer.module.css";

/** What a rename names: a room by its header, a world map by its matrix and half. */
export type RenameTarget = { kind: "room"; id: number } | { kind: "worldMap"; matrix: number; filter: WorldMap["filter"] };

/** An area a room can be given to: its key, its name and where it lies (the world map, else "Weitere Gebiete"). */
export interface AreaChoice {
  key: string;
  name: string;
  where: string;
}

/** The project file with a new name; an empty name drops the entry, so the suggestion shows again. */
export function renamedMeta(meta: WorldMeta, target: RenameTarget, name: string): WorldMeta {
  const text = name.trim();
  if (target.kind === "room") {
    const roomNames = { ...meta.roomNames };
    if (text === "") delete roomNames[String(target.id)];
    else roomNames[String(target.id)] = text;
    return { ...meta, roomNames };
  }
  const same = (w: WorldMeta["worldMaps"][number]) => w.matrix === target.matrix && (w.filter ?? null) === target.filter;
  const worldMaps = meta.worldMaps.filter((w) => !same(w));
  if (text !== "") worldMaps.push(target.filter === null ? { matrix: target.matrix, name: text } : { matrix: target.matrix, filter: target.filter, name: text });
  return { ...meta, worldMaps };
}

/**
 * The project file with the name a wizard gave what it made ("Raum anlegen": the room's name; "Gebiet anlegen": the area's name for the header
 * it assigned) as the header's clear name, as if it was renamed in the tree; null when nothing was assigned or no name given.
 */
export function namedByWizard(meta: WorldMeta, made: { header: number | null; name?: string }): WorldMeta | null {
  const name = made.name?.trim() ?? "";
  if (made.header === null || name === "") return null;
  return renamedMeta(meta, { kind: "room", id: made.header }, name);
}

/** The project file with a room given to an area by hand; null takes the assignment back (the entrance decides again). */
export function ownedMeta(meta: WorldMeta, room: number, areaKey: string | null): WorldMeta {
  const roomOwner = { ...meta.roomOwner };
  if (areaKey === null) delete roomOwner[String(room)];
  else roomOwner[String(room)] = areaKey;
  return { ...meta, roomOwner };
}

/** The tree's two edits of the project file; each one writes it at once (no undo: the file holds editor knowledge only). */
export function useWorldActions(meta: WorldMeta, setMeta: (next: WorldMeta) => Promise<void>) {
  const onRename = useCallback((target: RenameTarget, name: string) => void setMeta(renamedMeta(meta, target, name)), [meta, setMeta]);
  const onOwner = useCallback((room: number, areaKey: string | null) => void setMeta(ownedMeta(meta, room, areaKey)), [meta, setMeta]);
  return { onRename, onOwner };
}

/**
 * The actions sit inside a tree row, and React lets the events of their portalled menu and dialog bubble through it:
 * they stop here, so a click in the dialog does not select the row and a key does not move the tree.
 */
const stop = (e: SyntheticEvent) => e.stopPropagation();

/**
 * „Raum umbenennen“: one field „Name“ that starts with the current name; empty gives the suggestion back. Its owner keys it
 * on the opening, so the field starts anew, and keeps it mounted while closed, so the dialog closes like every other.
 * The inspector's row „Name“ in Erzählen opens it too.
 */
export function RenameDialog({
  open,
  title,
  name,
  suggestion,
  onClose,
  onRename,
}: {
  open: boolean;
  title: string;
  name: string;
  suggestion: string | null;
  onClose: () => void;
  onRename: (name: string) => void;
}) {
  const [text, setText] = useState(name);
  const submit = () => {
    const trimmed = text.trim();
    // The suggestion itself is not stored: the room follows it when it changes.
    onRename(suggestion !== null && trimmed === suggestion ? "" : trimmed);
    onClose();
  };
  return (
    <Dialog
      open={open}
      onOpenChange={(open) => !open && onClose()}
      title={title}
      actions={[
        { label: "Abbrechen", onClick: onClose },
        { label: "Umbenennen", kind: "primary", onClick: submit },
      ]}
    >
      <div
        onKeyDown={(e: KeyboardEvent) => {
          if (e.key !== "Enter") return;
          e.preventDefault();
          submit();
        }}
      >
        <TextField
          label="Name"
          showLabel
          value={text}
          onChange={setText}
          onCommit={setText}
          placeholder={suggestion ?? undefined}
          hint={suggestion !== null ? `Leer lassen für „${suggestion}“.` : "Leer lassen für den vorgeschlagenen Namen."}
        />
      </div>
    </Dialog>
  );
}

/**
 * The menu „Weitere Aktionen für <Name>“ at the selected room: „Umbenennen…“, „Gehört zu Gebiet…“ (a picker of the other
 * areas) and, for a room given to its area by hand, „Zuordnung zurücksetzen“.
 */
export function RoomActions({
  room,
  areaKey,
  areas,
  onRename,
  onOwner,
}: {
  room: Room;
  /** The area the room lies in; null under „Nicht verbunden“. */
  areaKey: string | null;
  areas: readonly AreaChoice[];
  onRename: (target: RenameTarget, name: string) => void;
  onOwner: (room: number, areaKey: string | null) => void;
}) {
  const [open, setOpen] = useState<"rename" | "owner" | null>(null);
  // Counts the openings of the rename dialog, which is keyed on it.
  const [renames, setRenames] = useState(0);
  const id = room.header.id;
  const items: MenuEntry[] = [
    {
      label: "Umbenennen…",
      onSelect: () => {
        setRenames((n) => n + 1);
        setOpen("rename");
      },
    },
    { label: "Gehört zu Gebiet…", onSelect: () => setOpen("owner") },
    ...(room.manual ? [{ label: "Zuordnung zurücksetzen", onSelect: () => onOwner(id, null) }] : []),
  ];
  const choices: PickerItem[] = areas.filter((a) => a.key !== areaKey).map((a) => ({ id: a.key, label: a.name, tech: a.where }));
  return (
    <span className={styles.actions} onClick={stop} onKeyDown={stop}>
      <Menu label={`Weitere Aktionen für ${room.name}`} trigger={<Icon name="more" />} items={items} />
      <RenameDialog
        key={renames}
        open={open === "rename"}
        title="Raum umbenennen"
        name={room.name}
        suggestion={room.suggested}
        onClose={() => setOpen(null)}
        onRename={(name) => onRename({ kind: "room", id }, name)}
      />
      <Picker
        open={open === "owner"}
        onOpenChange={(next) => !next && setOpen(null)}
        title="Gehört zu Gebiet"
        items={choices}
        onPick={(item) => onOwner(id, String(item.id))}
        placeholder="Gebiet suchen …"
        emptyText="Kein Gebiet passt zur Suche."
      />
    </span>
  );
}

/** The menu „Weitere Aktionen für <Name>“ at a world map: „Umbenennen…“. */
export function WorldMapActions({ map, onRename }: { map: WorldMap; onRename: (target: RenameTarget, name: string) => void }) {
  const [renaming, setRenaming] = useState(false);
  const [renames, setRenames] = useState(0);
  const start = () => {
    setRenames((n) => n + 1);
    setRenaming(true);
  };
  return (
    <span className={styles.actions} onClick={stop} onKeyDown={stop}>
      <Menu label={`Weitere Aktionen für ${map.name}`} trigger={<Icon name="more" />} items={[{ label: "Umbenennen…", onSelect: start }]} />
      <RenameDialog
        key={renames}
        open={renaming}
        title="Weltkarte umbenennen"
        name={map.name}
        suggestion={null}
        onClose={() => setRenaming(false)}
        onRename={(name) => onRename({ kind: "worldMap", matrix: map.matrix, filter: map.filter }, name)}
      />
    </span>
  );
}
