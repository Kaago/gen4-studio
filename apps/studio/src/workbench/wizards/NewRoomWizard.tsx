import { useEffect, useMemo, useState } from "react";
import type { CSSProperties } from "react";
import { createRoom, errorText } from "../../api";
import type { HeaderSummary, MatrixSummary, MemberCounts, RoomReport, RoomSpec, WarpLink } from "../../api";
import { lockedText } from "../../world/events";
import { thumbnails } from "../../world/thumbnailStore";
import { useThumbnail } from "../../world/useThumbnail";
import { DraftInput } from "../DraftInput";
import { headerLabel } from "../headers";
import { HeaderSearch } from "./HeaderSearch";
import { filterPlaces } from "../places";
import type { Place } from "../places";
import { TERMS, id4 } from "../terms";
import { useEvents } from "../useEvents";
import { useMatrix } from "../useMatrix";
import { useWarpTile } from "../warpTile";
import { HeaderChoice } from "./HeaderChoice";
import { useHeaderPatch } from "./useHeaderPatch";
import { DoneResult, PartialResult } from "./WizardResult";
import { WizardFrame } from "./WizardFrame";
import type { WizardStep } from "./WizardFrame";
import { MAP_LAST_TILE, emptyRoomForm, internalName, isLateFailure, matrixTiles, newHeaderForm, newHeaderValue, mapUnderTile, overviewNote, partialText, roomResult, roomSpec, roomSummary, templateWarp, warpOnTile } from "./wizardSpecs";
import type { HeaderChoiceValue, RoomForm } from "./wizardSpecs";

interface Props {
  headers: HeaderSummary[];
  links: WarpLink[];
  counts: MemberCounts;
  matrices: MatrixSummary[];
  /** The overview was read: until then `matrices` (and so the places) tell outside headers from rooms by nothing. */
  matricesLoaded: boolean;
  /** The latest read of the overview failed. */
  matricesFailed: boolean;
  /** The matrix the scene shows (null: none): `lastTile` is a tile of it. */
  shownMatrix: number | null;
  /** The headers grouped by place, as the world explorer shows them: the rooms among them are the templates. */
  places: Place[];
  /** The header outside the entrance is made in, and the room to copy, when the request already knows them. */
  prefill?: { outsideHeader?: number; template?: number };
  /** The tile the pointer was last over in the shown matrix (null: none yet): the door outside starts there when the outside header uses that matrix. */
  lastTile: () => { x: number; z: number } | null;
  /** The wizard is left; `written` says whether it changed the project (the app then reads it again). */
  onClose: (written: boolean) => void;
  /** "Zeigen": the app reads the project again and shows the new room. */
  onCreated: (report: RoomReport, spec: RoomSpec) => Promise<void>;
  /** The Dynamic Headers patch was applied: the app reads the header table and list again. */
  onHeaderPatch: () => Promise<void>;
  onError: (e: unknown) => void;
  /** "Hier spielen" on the result: after it was shown, the app saves, builds and runs. */
  onPlay?: () => void;
}

type Outcome = { kind: "done"; report: RoomReport; spec: RoomSpec } | { kind: "partial"; message: string };

/** The most template rooms the list shows; a longer list asks for a narrower search. */
const SHOWN = 60;
/** The last tile of the largest matrix (255 maps of 32 tiles). */
const MATRIX_LAST_TILE = 255 * 32 - 1;

const row: CSSProperties = { display: "flex", alignItems: "center", gap: 8, margin: "6px 0" };
const note: CSSProperties = { color: "#6b7280", margin: "6px 0" };
const heading: CSSProperties = { fontWeight: 600, marginBottom: 4 };
const warn: CSSProperties = { color: "#b45309", margin: "6px 0" };

/** The form with `template` as the room to copy: its area data is offered, and a copy of an event file follows the template. */
function withTemplate(form: RoomForm, template: HeaderSummary): RoomForm {
  return {
    ...form,
    template: template.id,
    templateMap: null,
    areaDataId: template.area_data_id,
    insideX: emptyRoomForm.insideX,
    insideZ: emptyRoomForm.insideZ,
    eventFile: typeof form.eventFile === "object" ? { Copy: template.event_file_id } : form.eventFile,
  };
}

const clampTile = (n: number) => Math.min(MAP_LAST_TILE, Math.max(0, n));

/** A typed tile number as it is, also when it lies off the map (the step then says why it cannot be used); null: no whole number. */
function parseTile(text: string): number | null {
  const n = Number(text.trim());
  return text.trim() !== "" && Number.isInteger(n) ? n : null;
}

/** "Raum anlegen": copies a room into a new matrix, gives it to a header and, if asked, puts a door pair between it and an outside header. */
export function NewRoomWizard({ headers, links, counts, matrices, matricesLoaded, matricesFailed, shownMatrix, places, prefill, lastTile, onClose, onCreated, onHeaderPatch, onError, onPlay }: Props) {
  // Where the pointer was when the wizard opened; the door outside starts there, if that tile is in the outside header's matrix.
  const [hovered] = useState(lastTile);
  const [form, setForm] = useState<RoomForm>(() => {
    const start: RoomForm = { ...emptyRoomForm, outsideHeader: prefill?.outsideHeader ?? null };
    const template = prefill?.template === undefined ? undefined : headers.find((h) => h.id === prefill.template);
    return template ? withTemplate(start, template) : start;
  });
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // The refusal of the last "Anlegen" (nothing was written), shown inside the dialog.
  const [refusal, setRefusal] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  // The door inside follows the template's own door until the user typed one.
  const [insideEdited, setInsideEdited] = useState(false);

  const waiting = overviewNote(matricesLoaded, matricesFailed);
  const find = (id: number | null) => (id === null ? null : (headers.find((h) => h.id === id) ?? null));
  const template = find(form.template);
  const roomHeader = find(form.header);
  const outsideHeader = find(form.outsideHeader);

  // The room is a copy of the first map of the template's matrix.
  const templateMatrix = useMatrix(template?.matrix_id ?? null, 0, onError);
  const templateMap = templateMatrix?.maps[0] ?? null;
  const picture = useThumbnail(thumbnails, templateMap, template?.area_data_id ?? 0);

  const templateEvents = useEvents(template?.event_file_id ?? null, onError);
  const door = templateWarp(templateEvents);
  const [doorX, doorZ] = [door?.x, door?.z];
  useEffect(() => {
    if (!insideEdited && doorX !== undefined && doorZ !== undefined) setForm((f) => ({ ...f, insideX: clampTile(doorX), insideZ: clampTile(doorZ) }));
  }, [doorX, doorZ, insideEdited, form.template]);

  // An entrance writes a warp into the event file of the header outside and into the room's: a locked one cannot take it.
  const keptFile = form.eventFile === "Keep" ? (roomHeader?.event_file_id ?? null) : null;
  const outsideEvents = useEvents(outsideHeader?.event_file_id ?? null, onError);
  const keptEvents = useEvents(keptFile, onError);
  // A copy and, for a new header, "keep" (it shares the template's file) both put the warp into the template's event file or a copy of it.
  const sharesTemplateFile = form.newHeader !== null && form.eventFile === "Keep";
  const copiedLocked = typeof form.eventFile === "object" || sharesTemplateFile ? (templateEvents?.locked ?? null) : null;
  let blocked: string | null = null;
  if (outsideHeader && outsideEvents?.locked) blocked = `${TERMS.eventFile.label} ${id4(outsideHeader.event_file_id)} (${TERMS.outside.label}) ist ${lockedText(outsideEvents.locked)}`;
  else if (copiedLocked && template) blocked = `Die Vorlage ${TERMS.eventFile.label} ${id4(template.event_file_id)} ist ${lockedText(copiedLocked)}`;
  else if (keptEvents?.locked && roomHeader) blocked = `${TERMS.eventFile.label} ${id4(roomHeader.event_file_id)} des ${TERMS.room.label}s ist ${lockedText(keptEvents.locked)}`;

  const connect = form.connect && blocked === null;
  // The door outside: what the user typed, else the pointer's last tile when the outside header uses the shown matrix. Anything else stays open.
  const outsideMatrix = outsideHeader ? (matrices.find((m) => m.index === outsideHeader.matrix_id) ?? null) : null;
  const fromScene = outsideHeader && hovered && outsideHeader.matrix_id === shownMatrix ? hovered : null;
  const outsideX = form.x ?? fromScene?.x ?? null;
  const outsideZ = form.z ?? fromScene?.z ?? null;
  const sceneDefault = fromScene !== null && (form.x === null || form.z === null);
  const outsideSize = outsideMatrix ? matrixTiles(outsideMatrix) : null;
  // The cells of the outside matrix: the door must lie in a cell with a map that belongs to the outside header.
  const outsideCells = useMatrix(outsideHeader?.matrix_id ?? null, 0, onError);
  // The event file the room keeps ("behalten"): its own, or for a new header the template's, which it then shares.
  const keptWarps =
    form.eventFile !== "Keep"
      ? null
      : form.newHeader !== null
        ? template && templateEvents && { file: template.event_file_id, warps: templateEvents.warps }
        : roomHeader && keptEvents && { file: roomHeader.event_file_id, warps: keptEvents.warps };
  // A copy of the template's event file brings its warp on the inside tile: the backend makes that one the way back.
  const copiesTemplate = typeof form.eventFile === "object" && template !== null && form.eventFile.Copy === template.event_file_id;
  const reused = copiesTemplate ? warpOnTile(templateEvents?.warps, form.insideX, form.insideZ) : null;
  // What the two door tiles mean for a warp: outside the map under the tile, inside the template's map (the room is a copy).
  const outsideMap = outsideX !== null && outsideZ !== null ? mapUnderTile(outsideCells, outsideX, outsideZ) : null;
  const outsideTile = useWarpTile(outsideMap, outsideX === null ? null : outsideX % 32, outsideZ === null ? null : outsideZ % 32);
  const insideTile = useWarpTile(templateMap, form.insideX, form.insideZ);
  // The room is the template's map as derived above (not stored in the form): it follows the template even when it is clicked again.
  // A new header is a copy of the template room's header.
  const newHeader = form.newHeader && { ...form.newHeader, template: form.template ?? 0 };
  const patch = useHeaderPatch(onHeaderPatch);
  const built = roomSpec({ ...form, newHeader, templateMap, x: outsideX, z: outsideZ, connect }, headers.length, {
    outsideMatrix: outsideCells ?? outsideMatrix,
    outsideWarps: outsideEvents?.warps,
    keptWarps,
  });

  const candidates = useMemo(() => {
    const outside = places.flatMap((p) => p.headers).filter((h) => h.kind === "outside" || h.header.id === prefill?.outsideHeader);
    return outside.map((h) => h.header).filter((h) => h.id !== form.header);
  }, [places, prefill?.outsideHeader, form.header]);

  // The rooms matching the search, by place, and how many there are (the list shows the first `SHOWN`).
  const { templates, templateCount } = useMemo(() => {
    const rooms = filterPlaces(places, query)
      .map((p) => ({ ...p, headers: p.headers.filter((h) => h.kind === "inside") }))
      .filter((p) => p.headers.length > 0);
    let room = 0;
    const shown = rooms
      .map((p) => ({ ...p, headers: p.headers.filter(() => room++ < SHOWN) }))
      .filter((p) => p.headers.length > 0);
    return { templates: shown, templateCount: room };
  }, [places, query]);

  function chooseTemplate(chosen: HeaderSummary) {
    if (chosen.id === form.template) return;
    setForm((f) => withTemplate(f, chosen));
    setInsideEdited(false);
  }

  function chooseHeader(value: HeaderChoiceValue) {
    setForm((f) => ({ ...f, ...value, newHeader: newHeaderForm(value.newHeader, f.name), outsideHeader: value.header !== null && value.header === f.outsideHeader ? null : f.outsideHeader }));
  }

  const setTile = (change: Partial<RoomForm>, inside = false) => {
    setForm((f) => ({ ...f, ...change }));
    if (inside) setInsideEdited(true);
  };
  const tileField = (label: string, value: number | null, max: number, key: "x" | "z" | "insideX" | "insideZ") => (
    <label style={row}>
      {label}
      <DraftInput
        label={label}
        type="number"
        min={0}
        max={max}
        value={value === null ? "" : String(value)}
        onCommit={(text) => {
          const n = parseTile(text);
          if (n !== null) setTile({ [key]: n }, key.startsWith("inside"));
        }}
        style={{ width: 80, textAlign: "right", fontFamily: "ui-monospace, monospace" }}
      />
    </label>
  );

  async function create() {
    if ("error" in built) return;
    setBusy(true);
    setRefusal(null);
    try {
      setOutcome({ kind: "done", report: await createRoom(built.spec), spec: built.spec });
    } catch (e) {
      const text = errorText(e);
      // Something stayed behind: the result step says what, the wizard cannot repair it.
      if (isLateFailure(e)) setOutcome({ kind: "partial", message: partialText(text) });
      else setRefusal(text);
    } finally {
      setBusy(false);
    }
  }

  async function show(report: RoomReport, spec: RoomSpec) {
    setBusy(true);
    try {
      await onCreated(report, spec);
    } finally {
      setBusy(false);
    }
  }

  /** "Hier spielen": shows what was made, then plays. */
  async function showAndPlay(report: RoomReport, spec: RoomSpec) {
    await show(report, spec);
    onPlay?.();
  }

  const headerValue: HeaderChoiceValue = { header: form.header, newHeader: newHeaderValue(form.newHeader, form.name, form.template ?? 0), areaDataId: form.areaDataId, eventFile: form.eventFile };

  const steps: WizardStep[] = [
    {
      id: "template",
      title: "Vorlage",
      canNext: template !== null && templateMap !== null,
      content: (
        <div style={{ display: "flex", gap: 16 }}>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={heading}>{TERMS.room.label} zum Kopieren wählen</div>
            <input
              type="search"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Ort oder Kürzel suchen"
              aria-label="Vorlage suchen"
              style={{ width: "100%", boxSizing: "border-box", padding: "2px 6px" }}
            />
            <div style={{ maxHeight: 260, overflowY: "auto", border: "1px solid #e5e7eb", marginTop: 4 }}>
              {waiting !== null && <div style={{ padding: "2px 6px", color: "#6b7280" }}>{waiting}</div>}
              {waiting === null && templates.map((place) => {
                return (
                  <div key={place.key}>
                    <div style={{ padding: "2px 6px", background: "#f3f4f6", fontSize: 12, color: "#4b5563" }}>{place.name}</div>
                    {place.headers.map(({ header }) => (
                      <button
                        key={header.id}
                        type="button"
                        onClick={() => chooseTemplate(header)}
                        style={{ all: "unset", boxSizing: "border-box", display: "block", width: "100%", padding: "1px 6px", cursor: "pointer", fontWeight: header.id === form.template ? 700 : 400 }}
                      >
                        <span style={{ fontFamily: "ui-monospace, monospace" }}>{id4(header.id)}</span> {headerLabel(header)}
                        {header.location_name ? <span style={{ color: "#6b7280" }}> ({header.internal_name})</span> : null}
                      </button>
                    ))}
                  </div>
                );
              })}
              {waiting === null && templateCount === 0 && <div style={{ padding: "2px 6px", color: "#6b7280" }}>Kein {TERMS.room.label} gefunden</div>}
              {waiting === null && templateCount > SHOWN && <div style={{ padding: "2px 6px", color: "#6b7280" }}>{templateCount - SHOWN} weitere, Suche eingrenzen</div>}
            </div>
            <label style={row}>
              Name
              <DraftInput label={`Name des neuen ${TERMS.room.label}s`} value={form.name} onCommit={(name) => setForm((f) => ({ ...f, name }))} style={{ flex: 1 }} />
            </label>
            <label style={row}>
              <input type="checkbox" checked={form.clearBuildings} onChange={(e) => setForm((f) => ({ ...f, clearBuildings: e.target.checked }))} />
              ohne Gebäude
            </label>
          </div>
          <div style={{ width: 200, flexShrink: 0 }}>
            <div style={heading}>Gewählt</div>
            {template === null ? (
              <div style={note}>Noch keine Vorlage gewählt.</div>
            ) : (
              <>
                <div>
                  {TERMS.header.label} {id4(template.id)} „{headerLabel(template)}“
                </div>
                {templateMatrix !== null && templateMap === null ? (
                  <div style={{ color: "#b91c1c" }}>Das {TERMS.matrix.label} dieses {TERMS.room.label}s hat keine {TERMS.map.label} zum Kopieren.</div>
                ) : typeof picture === "string" ? (
                  <img src={picture} alt="Vorschau der Vorlage" style={{ width: 160, height: 160, marginTop: 6, border: "1px solid #d1d5db" }} />
                ) : (
                  <div style={note}>{templateMap === null || picture === undefined ? "Lade Vorschau …" : "Keine Vorschau."}</div>
                )}
              </>
            )}
          </div>
        </div>
      ),
    },
    {
      id: "header",
      title: TERMS.header.label,
      canNext: form.header !== null || form.newHeader !== null,
      content: (
        <HeaderChoice
          headers={headers}
          links={links}
          counts={counts}
          value={headerValue}
          copySource={template?.event_file_id ?? null}
          onChange={chooseHeader}
          patch={patch.patch}
          patchError={patch.error}
          patchBusy={patch.busy}
          onApplyPatch={patch.apply}
          onRepairPatch={patch.repair}
          template={form.template ?? 0}
          defaultName={internalName(form.name)}
        />
      ),
    },
    {
      id: "entrance",
      title: "Eingang",
      canNext: "spec" in built,
      content: (
        <div>
          <label style={row}>
            <input type="checkbox" aria-label="später verbinden" checked={!connect} disabled={blocked !== null} onChange={(e) => setForm((f) => ({ ...f, connect: !e.target.checked }))} />
            später verbinden (keinen Eingang anlegen)
          </label>
          {blocked !== null && form.connect && <div style={{ color: "#b45309", margin: "6px 0" }}>Der Eingang lässt sich jetzt nicht anlegen: {blocked}</div>}
          {/* The lock follows the chosen header outside: picking another one is always possible. */}
          <fieldset disabled={!form.connect} style={{ border: "none", padding: 0, margin: 0 }}>
            <div style={heading}>{TERMS.header.label} außen</div>
            {waiting !== null ? (
              <div style={note}>{waiting}</div>
            ) : (
              <HeaderSearch headers={candidates} current={form.outsideHeader ?? -1} disabled={!form.connect} onPick={(id) => setForm((f) => ({ ...f, outsideHeader: id }))} />
            )}
            <div style={note}>
              {outsideHeader ? `Gewählt: ${id4(outsideHeader.id)} „${headerLabel(outsideHeader)}“` : `Kein ${TERMS.header.label} außen gewählt.`}
            </div>
          </fieldset>
          <fieldset disabled={!connect} style={{ border: "none", padding: 0, margin: 0 }}>
            <div style={heading}>Tür außen (Kachel im {TERMS.matrix.label} des {TERMS.header.label}s)</div>
            {outsideMatrix && outsideSize && (
              <div style={note}>
                {TERMS.matrix.label} {id4(outsideMatrix.index)}: {outsideSize.x} × {outsideSize.z} Kacheln (x 0 bis {outsideSize.x - 1}, z 0 bis {outsideSize.z - 1}).
              </div>
            )}
            {tileField("Außen: Kachel x", outsideX, outsideSize ? outsideSize.x - 1 : MATRIX_LAST_TILE, "x")}
            {tileField("Außen: Kachel z", outsideZ, outsideSize ? outsideSize.z - 1 : MATRIX_LAST_TILE, "z")}
            {sceneDefault && <div style={note}>Kachel zuletzt in der Szene gewählt (über der die Maus zuletzt war).</div>}
            {outsideHeader && outsideX === null && outsideZ === null && (
              <div style={note}>
                Noch keine Kachel: die Maus war zuletzt nicht über diesem {TERMS.matrix.label}; x und z eintragen.
              </div>
            )}
            <div style={heading}>Tür innen, Rückweg (Kachel 0 bis {MAP_LAST_TILE} im neuen {TERMS.room.label})</div>
            {tileField("Innen: Kachel x", form.insideX, MAP_LAST_TILE, "insideX")}
            {tileField("Innen: Kachel z", form.insideZ, MAP_LAST_TILE, "insideZ")}
            {!insideEdited && door && <div style={note}>aus der Vorlage übernommen (ihr erster Warp, Kachel x {door.x}, z {door.z}).</div>}
            {!insideEdited && door && (door.x > MAP_LAST_TILE || door.z > MAP_LAST_TILE) && (
              <div style={{ color: "#b45309", margin: "6px 0" }}>
                Dieser Warp liegt außerhalb der ersten {TERMS.map.label} der Vorlage und wurde auf Kachel x {form.insideX}, z {form.insideZ} begrenzt: prüfen.
              </div>
            )}
            {!insideEdited && !door && <div style={note}>Standardwert: die Vorlage hat keinen Warp.</div>}
            {reused && (
              <div style={note}>
                Die kopierte {TERMS.eventFile.label} hat hier schon {TERMS.warp.tech} {reused.index} (Ziel {TERMS.header.label} {id4(reused.dest_header_id)}): er wird zum Rückweg, es kommt kein zweiter dazu.
              </div>
            )}
            <div style={heading}>Tür-Kacheln</div>
            {outsideTile && <div style={outsideTile.fires ? note : warn}>Außen: {outsideTile.text}</div>}
            {insideTile && <div style={insideTile.fires ? note : warn}>Innen: {insideTile.text}</div>}
            <label style={row}>
              <input type="checkbox" checked={form.doorTiles} onChange={(e) => setForm((f) => ({ ...f, doorTiles: e.target.checked }))} />
              Kacheln, auf denen ein {TERMS.warp.tech} nicht auslöst, zur Tür machen (Verhalten 0x69, Kollision 0x80)
            </label>
          </fieldset>
          {"error" in built && <div style={{ color: "#b91c1c", marginTop: 8 }}>{built.error}</div>}
        </div>
      ),
    },
    {
      id: "summary",
      title: "Zusammenfassung",
      canNext: "spec" in built,
      content: (
        <div>
          {"spec" in built ? (
            roomSummary(built.spec, headers, form.connect && blocked !== null ? blocked : undefined, links, { reused, outsideTile, insideTile }).map((line) => (
              <p key={line} style={{ margin: "6px 0" }}>
                {line}
              </p>
            ))
          ) : (
            <p style={{ color: "#b91c1c" }}>{built.error}</p>
          )}
        </div>
      ),
    },
  ];

  let result = null;
  let onShow: (() => void) | undefined;
  let play: (() => void) | undefined;
  if (outcome?.kind === "done") {
    const { report, spec } = outcome;
    result = <DoneResult title={`${TERMS.room.label} angelegt`} {...roomResult(report, spec)} />;
    onShow = () => void show(report, spec);
    if (onPlay) play = () => void showAndPlay(report, spec);
  } else if (outcome?.kind === "partial") {
    result = <PartialResult message={outcome.message} />;
  }

  return (
    <WizardFrame
      title={`${TERMS.room.label} anlegen`}
      icon="room"
      steps={steps}
      step={step}
      onStep={(i) => {
        setRefusal(null);
        setStep(i);
      }}
      submitLabel="Anlegen"
      onSubmit={() => void create()}
      onCancel={() => onClose(outcome !== null)}
      onShow={onShow}
      onPlay={play}
      busy={busy || patch.busy}
      result={result}
      error={refusal}
      summary={"spec" in built ? roomSummary(built.spec, headers, undefined, links)[0] : undefined}
      irreversible
    />
  );
}
