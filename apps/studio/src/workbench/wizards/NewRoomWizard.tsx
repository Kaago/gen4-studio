import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { createRoom, errorText } from "../../api";
import type { DraftDto, HeaderFamily, HeaderSummary, MatrixSummary, MemberCounts, RoomReport, RoomSpec, WarpLink } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { Button, Checkbox, ListRow, Notice, NumberField, Reference, TextField, useRovingList } from "../../ui/primitives";
import { lockedText } from "../../world/events";
import { thumbnails } from "../../world/thumbnailStore";
import { useThumbnail } from "../../world/useThumbnail";
import { headerLabel } from "../headers";
import { HeaderPicker } from "../inspector/HeaderPicker";
import { filterPlaces } from "../places";
import type { Place } from "../places";
import { id4, termLabel } from "../terms";
import { useEvents } from "../useEvents";
import { useMatrix } from "../useMatrix";
import { useWarpTile } from "../warpTile";
import { DraftNotes } from "./DraftNotes";
import { HeaderChoice } from "./HeaderChoice";
import { PatchConsent, patchReady } from "./PatchConsent";
import { PlaceStep } from "./PlaceStep";
import type { PlaceStepValue } from "./PlaceStep";
import { useDraftPlan } from "./useDraftPlan";
import { useHeaderPatch } from "./useHeaderPatch";
import { DoneResult, PartialResult } from "./WizardResult";
import { WizardFrame } from "./WizardFrame";
import type { WizardStep } from "./WizardFrame";
import { MAP_LAST_TILE, emptyRoomForm, outsideArea, internalName, isLateFailure, matrixTiles, newHeaderForm, newHeaderValue, mapUnderTile, overviewNote, partialText, placeError, placeForm, planSpec, roomPlace, roomResult, roomSpec, roomSummary, templateWarp, warpOnTile } from "./wizardSpecs";
import type { HeaderChoiceValue, RoomForm } from "./wizardSpecs";
import parts from "./WizardStep.module.css";
import styles from "./NewRoomWizard.module.css";

interface Props {
  /** Bauen gives the new room a new header with a place and a look (step „Ort“); Tüfteln lets the user choose the header. */
  depth: Depth;
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
  /** A repair of the patch went in (the app closes the toast that offered it). */
  onHeaderPatchRepaired?: () => void;
  onError: (e: unknown) => void;
  /** "Hier spielen" on the result: after it was shown, the app saves, builds and runs. */
  onPlay?: () => void;
  /** A next step's button on the result (its recipe runs): the app shows what was made and opens that recipe's wizard. */
  onStep?: (id: string, report: RoomReport, spec: RoomSpec) => void;
  /** The draft of the room as planned (null: nothing planned): the scene and the world map draw it until the wizard closes or creates. */
  onDraft: (draft: DraftDto | null) => void;
}

type Outcome = { kind: "done"; report: RoomReport; spec: RoomSpec } | { kind: "partial"; message: string };

/** The most template rooms the list shows; a longer list asks for a narrower search. */
const SHOWN = 60;
/** The last tile of the largest matrix (255 maps of 32 tiles). */
const MATRIX_LAST_TILE = 255 * 32 - 1;

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

/** „Raum anlegen“: copies a room into a new matrix, gives it to a header and, if asked, puts a door pair between it and an outside header. */
export function NewRoomWizard({ depth, headers, links, counts, matrices, matricesLoaded, matricesFailed, shownMatrix, places, prefill, lastTile, onClose, onCreated, onHeaderPatch, onHeaderPatchRepaired, onError, onPlay, onStep, onDraft }: Props) {
  const { t } = useTranslation("workbench");
  // Where the pointer was when the wizard opened; the door outside starts there, if that tile is in the outside header's matrix.
  const [hovered] = useState(lastTile);
  const [stored, setForm] = useState<RoomForm>(() => {
    const start: RoomForm = { ...emptyRoomForm, outsideHeader: prefill?.outsideHeader ?? null };
    const template = prefill?.template === undefined ? undefined : headers.find((h) => h.id === prefill.template);
    return template ? withTemplate(start, template) : start;
  });
  const building = !depthAtLeast(depth, "tuefteln");
  // Bauen always makes a new header with an empty event file: what Tüfteln chose does not count there.
  const form = useMemo<RoomForm>(() => (building ? { ...stored, header: null, newHeader: null, areaDataId: stored.lookAreaData, eventFile: "New" } : stored), [building, stored]);
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  // The refusal of the last „Anlegen“ (nothing was written), shown inside the dialog.
  const [refusal, setRefusal] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  // The door inside follows the template's own door until the user typed one.
  const [insideEdited, setInsideEdited] = useState(false);
  const [pickingOutside, setPickingOutside] = useState(false);
  const templateList = useRef<HTMLDivElement>(null);
  useRovingList(templateList);

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
  if (outsideHeader && outsideEvents?.locked) blocked = t("newRoomWizard.lockedOutside", { id: id4(outsideHeader.event_file_id), locked: lockedText(outsideEvents.locked) });
  else if (copiedLocked && template) blocked = t("newRoomWizard.lockedTemplate", { id: id4(template.event_file_id), locked: lockedText(copiedLocked) });
  else if (keptEvents?.locked && roomHeader) blocked = t("newRoomWizard.lockedRoom", { id: id4(roomHeader.event_file_id), locked: lockedText(keptEvents.locked) });

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
  const patch = useHeaderPatch(onHeaderPatch, { repaired: onHeaderPatchRepaired });
  // Bauen: the place and the look follow their defaults (the place at the entrance, the template's look) until chosen. An entrance
  // that cannot be made leaves the template's place.
  const placeDefault = roomPlace(null, connect ? outsideHeader : null, template);
  const place = form.place ?? placeDefault;
  const look = form.lookAreaData ?? template?.area_data_id ?? 0;
  // The consent counts only while the patch is missing.
  const patchConsent = form.patchConsent && patch.patch?.applied === false;
  const built = roomSpec({ ...form, newHeader, templateMap, x: outsideX, z: outsideZ, connect, place, patchConsent }, headers.length, depth, {
    outsideMatrix: outsideCells ?? outsideMatrix,
    outsideWarps: outsideEvents?.warps,
    keptWarps,
  });
  // What the room will be, planned by the kernel from the same spec „Anlegen“ sends (the patch assumed while it is missing, see
  // `planSpec`); gone once it is made.
  const plan = useDraftPlan("create_room", outcome === null && "spec" in built ? planSpec(built.spec, patch.patch?.applied === false) : null, onDraft);

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
  // A tile beyond `max` is refused by the field with the bound named; it never becomes the nearest tile.
  const tileField = (label: string, value: number | null, max: number, key: "x" | "z" | "insideX" | "insideZ", disabled: boolean) => (
    <div className={parts.field}>
      <NumberField label={label} showLabel min={0} max={max} value={value} disabled={disabled} hint={t("inspectorField.range", { max })} onCommit={(n) => setTile({ [key]: n }, key.startsWith("inside"))} />
    </div>
  );

  /** The app reads the header table and list again after a create that applied the patch; a failed read goes to `onError`, the result stays. */
  async function rereadPatch() {
    try {
      await onHeaderPatch();
    } catch (e) {
      onError(e);
    }
  }

  async function create() {
    if ("error" in built) return;
    setBusy(true);
    setRefusal(null);
    const patching = built.spec.header.allow_patch === true;
    try {
      const report = await createRoom(built.spec);
      setOutcome({ kind: "done", report, spec: built.spec });
      // The call applied the patch: the app reads the header table and list again, as after „Patch anwenden“.
      if (report.patched) await rereadPatch();
    } catch (e) {
      const text = errorText(e);
      // Something stayed behind: the result step says what, the wizard cannot repair it. A patch asked for may be in by then.
      if (isLateFailure(e)) {
        setOutcome({ kind: "partial", message: partialText(text, depth, "room") });
        if (patching) await rereadPatch();
      } else setRefusal(text);
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

  /** „Hier spielen“: shows what was made, then plays. */
  async function showAndPlay(report: RoomReport, spec: RoomSpec) {
    await show(report, spec);
    onPlay?.();
  }

  const headerValue: HeaderChoiceValue = { header: form.header, newHeader: newHeaderValue(form.newHeader, form.name, form.template ?? 0), areaDataId: form.areaDataId, eventFile: form.eventFile };

  const summaryLines = "spec" in built ? roomSummary(built.spec, headers, form.connect && blocked !== null ? blocked : undefined, links, { reused, outsideTile, insideTile }, depth) : null;

  /** A choice of the step „Ort“: a default is kept as null, so it follows the entrance and the template. */
  function choosePlace(value: PlaceStepValue) {
    setForm((f) => ({ ...f, place: placeForm(value.place, placeDefault), lookAreaData: value.areaDataId === (template?.area_data_id ?? 0) ? null : value.areaDataId }));
  }
  // Only Diamond/Pearl lacks the patch, and only it has no new places: the step asks for nothing else.
  const family: HeaderFamily = patch.patch?.supported === false ? "DP" : "Pt";
  // Bauen speaks of the area at the entrance and its tiles, Tüfteln of the header outside and its raster.
  const outsideWord = building ? outsideArea() : t("newRoomWizard.outsideHeader");
  // Bauen asks for the place and the look; Tüfteln for the header itself.
  const headerStep: WizardStep = building
    ? {
        id: "place",
        title: t("newRoomWizard.stepPlace"),
        canNext: placeError(place) === null,
        content: <PlaceStep value={{ place: place ?? { New: form.name }, areaDataId: look }} onChange={choosePlace} headers={headers} template={template} family={family} name={form.name} />,
      }
    : {
        id: "header",
        title: termLabel("header"),
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
      };

  const steps: WizardStep[] = [
    {
      id: "template",
      title: t("newRoomWizard.stepTemplate"),
      canNext: template !== null && templateMap !== null,
      content: (
        <>
          <section className={`${parts.column} ${parts.wide}`}>
            <h3 className={parts.heading}>{t("newRoomWizard.chooseRoom")}</h3>
            <div className={styles.search}>
              <TextField type="search" label={t("newRoomWizard.searchTemplate")} placeholder={t("newRoomWizard.searchPlaceholder")} value={query} onChange={setQuery} onCommit={setQuery} />
            </div>
            {/* The notes stand beside the listbox, inside the same scroll box: a listbox holds only its groups of options. */}
            <div className={styles.list}>
              {waiting !== null && <p className={styles.empty}>{waiting}</p>}
              <div ref={templateList} role="listbox" aria-label={t("newRoomWizard.templates")} className={styles.options}>
                {waiting === null &&
                  templates.map((place) => (
                    <div key={place.key} role="group" aria-label={place.name}>
                      <div className={styles.place} aria-hidden="true">
                        {place.name}
                      </div>
                      {place.headers.map(({ header }) => (
                        <ListRow
                          key={header.id}
                          label={headerLabel(header)}
                          tech={header.location_name ? `${id4(header.id)} ${header.internal_name}` : id4(header.id)}
                          selected={header.id === form.template}
                          onSelect={() => chooseTemplate(header)}
                          dataAttrs={{ "data-header": header.id }}
                        />
                      ))}
                    </div>
                  ))}
              </div>
              {waiting === null && templateCount === 0 && <p className={styles.empty}>{t("newRoomWizard.noRoom")}</p>}
              {waiting === null && templateCount > SHOWN && <p className={styles.empty}>{t("newRoomWizard.more", { n: templateCount - SHOWN })}</p>}
            </div>
          </section>
          <section className={parts.column}>
            <h3 className={parts.heading}>{t("newRoomWizard.newRoom")}</h3>
            <div className={parts.field}>
              <TextField label={t("newRoomWizard.name")} showLabel value={form.name} onCommit={(name) => setForm((f) => ({ ...f, name }))} />
            </div>
            <Checkbox label={t("newRoomWizard.withoutBuildings")} checked={form.clearBuildings} onChange={(clearBuildings) => setForm((f) => ({ ...f, clearBuildings }))} />
          </section>
          <section className={parts.column}>
            <h3 className={parts.heading}>{t("newRoomWizard.chosen")}</h3>
            {template === null ? (
              <p className={parts.note}>{t("newRoomWizard.noTemplate")}</p>
            ) : (
              <>
                <p className={parts.text}>
                  {building ? t("wizardSpecs.quoted", { name: headerLabel(template) }) : t("newRoomWizard.templateHeader", { id: id4(template.id), name: headerLabel(template) })}
                </p>
                {templateMatrix !== null && templateMap === null ? (
                  <Notice kind="error">{building ? t("newRoomWizard.noMapBuild") : t("newRoomWizard.noMapTinker")}</Notice>
                ) : typeof picture === "string" ? (
                  <img src={picture} alt={t("newRoomWizard.templatePreview")} className={styles.picture} />
                ) : (
                  <p className={parts.note}>{templateMap === null || picture === undefined ? t("newRoomWizard.loadingPreview") : t("newRoomWizard.noPreview")}</p>
                )}
              </>
            )}
          </section>
        </>
      ),
    },
    headerStep,
    {
      id: "entrance",
      title: t("newRoomWizard.stepEntrance"),
      canNext: "spec" in built,
      content: (
        <>
          <section className={parts.column}>
            <h3 className={parts.heading}>{outsideWord}</h3>
            <Checkbox label={t("newRoomWizard.connectLater")} checked={!connect} disabled={blocked !== null} onChange={(later) => setForm((f) => ({ ...f, connect: !later }))} />
            {blocked !== null && form.connect && <Notice kind="warning">{t("newRoomWizard.blocked", { why: blocked })}</Notice>}
            {/* The lock follows the chosen header outside: picking another one is always possible. */}
            {waiting !== null ? (
              <p className={parts.note}>{waiting}</p>
            ) : (
              <>
                {outsideHeader ? (
                  <div className={parts.chip}>
                    <Reference area="welt" label={headerLabel(outsideHeader)} tech={id4(outsideHeader.id)} onOpen={() => setPickingOutside(true)} disabled={!form.connect} dataAttrs={{ "data-header": outsideHeader.id }} />
                  </div>
                ) : (
                  <p className={parts.note}>{building ? t("newRoomWizard.noOutsideArea") : t("newRoomWizard.noOutsideHeader")}</p>
                )}
                <div className={parts.actions}>
                  <Button kind="secondary" size="s" icon="search" disabled={!form.connect} onClick={() => setPickingOutside(true)}>
                    {building ? t("newRoomWizard.chooseOutsideAreaButton") : t("newRoomWizard.chooseOutsideHeaderButton")}
                  </Button>
                </div>
                <HeaderPicker
                  open={pickingOutside}
                  onOpenChange={setPickingOutside}
                  headers={candidates}
                  onPick={(id) => setForm((f) => ({ ...f, outsideHeader: id }))}
                  title={building ? t("newRoomWizard.chooseOutsideArea") : t("newRoomWizard.chooseOutsideHeader")}
                />
              </>
            )}
            {"error" in built && <Notice kind="error">{built.error}</Notice>}
          </section>
          <section className={parts.column}>
            <h3 className={parts.heading}>{t("newRoomWizard.doorOutside")}</h3>
            <p className={parts.note}>
              {outsideMatrix && outsideSize
                ? building
                  ? t("newRoomWizard.tilesOutsideBuild", { x: outsideSize.x, z: outsideSize.z, maxX: outsideSize.x - 1, maxZ: outsideSize.z - 1 })
                  : t("newRoomWizard.tilesOutsideTinker", { matrix: id4(outsideMatrix.index), x: outsideSize.x, z: outsideSize.z, maxX: outsideSize.x - 1, maxZ: outsideSize.z - 1 })
                : building
                  ? t("newRoomWizard.tileOutsideBuild")
                  : t("newRoomWizard.tileOutsideTinker")}
            </p>
            {tileField(t("newRoomWizard.outsideTileX"), outsideX, outsideSize ? outsideSize.x - 1 : MATRIX_LAST_TILE, "x", !connect)}
            {tileField(t("newRoomWizard.outsideTileZ"), outsideZ, outsideSize ? outsideSize.z - 1 : MATRIX_LAST_TILE, "z", !connect)}
            {sceneDefault && <p className={parts.note}>{t("newRoomWizard.fromScene")}</p>}
            {outsideHeader && outsideX === null && outsideZ === null && (
              <p className={parts.note}>{building ? t("newRoomWizard.noTileBuild") : t("newRoomWizard.noTileTinker")}</p>
            )}
            {outsideTile && <p className={outsideTile.fires ? parts.note : parts.warn}>{t("newRoomWizard.outsideText", { text: outsideTile.text })}</p>}
          </section>
          <section className={parts.column}>
            <h3 className={parts.heading}>{t("newRoomWizard.doorInside")}</h3>
            <p className={parts.note}>{t("newRoomWizard.insideRange", { max: MAP_LAST_TILE })}</p>
            {tileField(t("newRoomWizard.insideTileX"), form.insideX, MAP_LAST_TILE, "insideX", !connect)}
            {tileField(t("newRoomWizard.insideTileZ"), form.insideZ, MAP_LAST_TILE, "insideZ", !connect)}
            {!insideEdited && door && <p className={parts.note}>{t("newRoomWizard.fromTemplate", { x: door.x, z: door.z })}</p>}
            {!insideEdited && door && (door.x > MAP_LAST_TILE || door.z > MAP_LAST_TILE) && (
              <Notice kind="warning">{t("newRoomWizard.clamped", { x: form.insideX, z: form.insideZ })}</Notice>
            )}
            {!insideEdited && !door && <p className={parts.note}>{t("newRoomWizard.noWarp")}</p>}
            {reused && <p className={parts.note}>{t("newRoomWizard.reused", { index: reused.index, target: id4(reused.dest_header_id) })}</p>}
            {insideTile && <p className={insideTile.fires ? parts.note : parts.warn}>{t("newRoomWizard.insideText", { text: insideTile.text })}</p>}
            <Checkbox
              label={t("newRoomWizard.doorTiles")}
              checked={form.doorTiles}
              disabled={!connect}
              onChange={(doorTiles) => setForm((f) => ({ ...f, doorTiles }))}
            />
          </section>
        </>
      ),
    },
    {
      id: "summary",
      title: t("newRoomWizard.summary"),
      canNext: "spec" in built && (!building || patchReady(patch.patch, form.patchConsent)),
      content: (
        <section className={parts.column}>
          {summaryLines ? (
            <Notice kind="info">
              <div className={parts.lines}>
                {summaryLines.map((line) => (
                  <p key={line}>{line}</p>
                ))}
              </div>
            </Notice>
          ) : (
            "error" in built && <Notice kind="error">{built.error}</Notice>
          )}
          <DraftNotes draft={plan.draft} error={plan.error} depth={depth} />
          {building && <PatchConsent patch={patch.patch} error={patch.error} checked={form.patchConsent} onChange={(patchConsent) => setForm((f) => ({ ...f, patchConsent }))} />}
        </section>
      ),
    },
  ];

  let result = null;
  let onShow: (() => void) | undefined;
  let play: (() => void) | undefined;
  if (outcome?.kind === "done") {
    const { report, spec } = outcome;
    result = <DoneResult title={t("newRoomWizard.done")} {...roomResult(report, spec, depth)} onStep={onStep && ((id) => onStep(id, report, spec))} />;
    onShow = () => void show(report, spec);
    if (onPlay) play = () => void showAndPlay(report, spec);
  } else if (outcome?.kind === "partial") {
    result = <PartialResult message={outcome.message} depth={depth} />;
  }

  return (
    <WizardFrame
      title={t("newRoomWizard.title")}
      icon="room"
      steps={steps}
      step={step}
      onStep={(i) => {
        setRefusal(null);
        setStep(i);
      }}
      submitLabel={t("newRoomWizard.submit")}
      onSubmit={() => void create()}
      onCancel={() => onClose(outcome !== null)}
      onShow={onShow}
      onPlay={play}
      busy={busy || patch.busy}
      result={result}
      error={refusal}
      summary={"spec" in built ? roomSummary(built.spec, headers, undefined, links, {}, depth)[0] : undefined}
      irreversible
    />
  );
}
