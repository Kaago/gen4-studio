import { useId, useMemo, useRef } from "react";
import type { DataRowDto, EventsDto, HeaderSummary } from "../api";
import { EmptyState, ListRow, Notice, PanelHead, useRovingList } from "../ui/primitives";
import type { IconName } from "../ui/primitives";
import { lockedText } from "../world/events";
import type { EventKind, EventRef } from "../world/events";
import type { BuildingRef } from "../world/buildingLayer";
import type { MapBuildings } from "../world/scene/types";
import { contextCounts, groupEvents } from "./context";
import { id4 } from "./terms";
import { scriptTrainerLabel } from "./trainerScript";
import styles from "./ContextPanel.module.css";

export interface ContextPanelProps {
  /** Whether a place is shown at all (else the panel says to choose one). */
  hasPlace: boolean;
  /** Its events (null while they load, or when reading them failed). */
  events: EventsDto | null;
  /** For the names of the places the doors lead to. */
  headers: readonly HeaderSummary[];
  /** The buildings of the shown maps as the scene reports them. */
  buildings: MapBuildings[];
  selectedEvent: EventRef | null;
  selectedBuilding: BuildingRef | null;
  onSelectEvent: (ref: EventRef) => void;
  onSelectBuilding: (ref: BuildingRef) => void;
  /** Follows a door to the header it leads to. */
  onOpenHeader: (header: number) => void;
  /** The events of the place are being read (`events` is null until they come): an empty list says "Lade …", not "nichts". */
  loading?: boolean;
  /** The trainer list: a figure whose script starts a battle is named by the trainer's class and name (by number while it is not read). */
  trainers?: readonly DataRowDto[] | null;
}

const KIND_ICON: Record<EventKind, IconName> = { object: "figure", warp: "door", coord: "trigger", bg: "sign" };

/** "An diesem Ort": what the shown place holds, as a list: its events by class and the buildings of its maps; a click selects like a click in the scene. */
export function ContextPanel({ hasPlace, events, headers, buildings, selectedEvent, selectedBuilding, onSelectEvent, onSelectBuilding, onOpenHeader, loading = false, trainers = null }: ContextPanelProps) {
  const list = useRef<HTMLDivElement>(null);
  useRovingList(list);
  const uid = useId();
  const groups = useMemo(
    () => (events ? groupEvents(events, (id) => headers.find((h) => h.id === id)?.internal_name, (script) => scriptTrainerLabel(trainers, script)) : []),
    [events, headers, trainers],
  );
  const buildingTotal = buildings.reduce((sum, m) => sum + m.models.length, 0);
  if (!hasPlace) {
    return (
      <section className={styles.panel} aria-label="An diesem Ort">
        <PanelHead title="An diesem Ort" />
        <EmptyState what="Noch kein Ort gewählt." why="Wähle links einen Ort, dann stehen hier seine Events und Gebäude." />
      </section>
    );
  }
  const empty = groups.length === 0 && buildingTotal === 0;
  const waiting = loading && events === null;
  return (
    <section className={styles.panel} aria-label="An diesem Ort">
      <PanelHead title="An diesem Ort" subtitle={waiting && empty ? undefined : contextCounts(events, buildingTotal)} />
      <div className={styles.body}>
        {events?.locked && (
          <Notice kind="warning" title="Eventdatei gesperrt">
            {lockedText(events.locked)}
          </Notice>
        )}
        {empty && waiting && <p className={styles.empty}>Lade …</p>}
        {empty && !waiting && (
          <p className={styles.empty}>Hier ist noch nichts. Mit den Werkzeugen setzt du eine Figur, eine Tür, einen Auslöser, ein Schild oder ein Gebäude.</p>
        )}
        <div ref={list} role="listbox" aria-label="Events und Gebäude an diesem Ort" className={styles.list} hidden={empty}>
          {groups.map((group) => (
            <div key={group.kind} role="group" aria-labelledby={`${uid}-${group.kind}`}>
              <div id={`${uid}-${group.kind}`} className={styles.group}>
                {group.label}
                <span className={styles.count}>{group.rows.length}</span>
              </div>
              {group.rows.map((row) => {
                const ref: EventRef = { file: events?.event_file_id ?? -1, kind: group.kind, index: row.index };
                const known = row.dest !== undefined && headers.some((h) => h.id === row.dest);
                return (
                  <div
                    key={row.index}
                    role="presentation"
                    className={styles.entry}
                    onDoubleClick={known ? () => onOpenHeader(row.dest as number) : undefined}
                    // Enter on a door follows it too (after the row selected it), as the double click does.
                    onKeyDown={known ? (e) => e.key === "Enter" && onOpenHeader(row.dest as number) : undefined}
                  >
                    <ListRow
                      label={row.title}
                      tech={row.tech}
                      icon={KIND_ICON[group.kind]}
                      selected={selectedEvent?.file === ref.file && selectedEvent.kind === ref.kind && selectedEvent.index === ref.index}
                      onSelect={() => onSelectEvent(ref)}
                      dataAttrs={{ "data-event": `${group.kind}:${row.index}` }}
                      title={known ? "Doppelklick oder Enter folgt der Tür zum Ziel" : undefined}
                    />
                  </div>
                );
              })}
            </div>
          ))}
          {buildingTotal > 0 && (
            <div role="group" aria-labelledby={`${uid}-buildings`}>
              <div id={`${uid}-buildings`} className={styles.group}>
                Gebäude
                <span className={styles.count}>{buildingTotal}</span>
              </div>
              {buildings.map((m) => (
                <div key={m.map} role="group" aria-label={`Karte ${id4(m.map)}`}>
                  {buildings.length > 1 && <div className={styles.map}>{`Karte ${id4(m.map)}`}</div>}
                  {m.models.map((model, index) => (
                    <ListRow
                      key={index}
                      label={`Gebäude ${index + 1}`}
                      tech={`Modell ${model}`}
                      icon="building"
                      selected={selectedBuilding?.map === m.map && selectedBuilding.index === index}
                      onSelect={() => onSelectBuilding({ map: m.map, index })}
                      dataAttrs={{ "data-building": `${m.map}:${index}` }}
                    />
                  ))}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
