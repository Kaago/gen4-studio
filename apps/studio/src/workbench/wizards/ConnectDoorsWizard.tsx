import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText, recipeApply } from "../../api";
import type { ConnectDoorsInput, ConnectDoorsReport, DraftDto, HeaderSummary, MatrixSummary, RecipeOutcome } from "../../api";
import type { Depth } from "../../ui/frame/shellState";
import { t as translate } from "../../i18n";
import { Button, Checkbox, Notice, Picker, Segmented } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import { headerLabel } from "../headers";
import { doorText } from "../inspector/WarpField";
import { useEvents } from "../useEvents";
import { DraftNotes } from "./DraftNotes";
import { PlaceField, TileFields, placeTiles } from "./PlaceField";
import { useDraftPlan } from "./useDraftPlan";
import { DoneResult, PartialResult } from "./WizardResult";
import { WizardFrame } from "./WizardFrame";
import type { WizardStep } from "./WizardFrame";
import { doorEnd, doorsResult, isLateFailure, warpOnTile } from "./wizardSpecs";
import parts from "./WizardStep.module.css";

export interface ConnectDoorsWizardProps {
  depth: Depth;
  headers: HeaderSummary[];
  matrices: readonly MatrixSummary[];
  /** Door A when the wizard opens: the door the inspector shows („Tür verbinden…“), or the selected place with the tile last chosen. */
  prefill?: { a?: { header: number; warp?: number; tile?: { x: number; z: number } } };
  /** Both ends as planned (null: nothing planned): the scene draws the end of the shown matrix. */
  onDraft: (draft: DraftDto | null) => void;
  /** The doors are connected (one undo step): the app reads what the recipe changed again. */
  onDone: (outcome: RecipeOutcome<ConnectDoorsReport>, input: ConnectDoorsInput) => Promise<void>;
  /** „Zeigen“ on the result: the app shows the place of door A. */
  onShow?: (outcome: RecipeOutcome<ConnectDoorsReport>, input: ConnectDoorsInput) => void;
  onStep?: (id: string, outcome: RecipeOutcome<ConnectDoorsReport>, input: ConnectDoorsInput) => void;
  /** The wizard is left; `written` says it changed the project without the app having read it again. */
  onCancel: (written: boolean) => void;
  onError: (e: unknown) => void;
}

/** One end as the wizard asks for it: its place, and a door there („Vorhandene Tür“) or a tile for a new one („Neue Kachel“). */
export interface EndForm {
  header: number | null;
  mode: "door" | "tile";
  warp: number | null;
  x: number | null;
  z: number | null;
}

type Outcome = { kind: "done"; outcome: RecipeOutcome<ConnectDoorsReport>; input: ConnectDoorsInput } | { kind: "partial"; message: string };

/** The kinds of an end, in the language that is shown when called. */
const modes = () => [
  { value: "door" as const, label: translate("workbench:connectDoorsWizard.existingDoor") },
  { value: "tile" as const, label: translate("workbench:connectDoorsWizard.newTile") },
];

function startEnd(prefill: { header: number; warp?: number; tile?: { x: number; z: number } } | undefined): EndForm {
  if (!prefill) return { header: null, mode: "door", warp: null, x: null, z: null };
  if (prefill.tile && prefill.warp === undefined) return { header: prefill.header, mode: "tile", warp: null, x: prefill.tile.x, z: prefill.tile.z };
  return { header: prefill.header, mode: "door", warp: prefill.warp ?? null, x: null, z: null };
}

/** One end in the words of the summary: „Tür 2 in Zweiblattdorf“, „neue Tür auf Kachel x 5, z 6 in Spielerhaus“. */
function endText(end: EndForm, headers: readonly HeaderSummary[]): string {
  const header = headers.find((h) => h.id === end.header);
  const place = header ? headerLabel(header) : "?";
  return end.mode === "door"
    ? translate("workbench:connectDoorsWizard.endDoor", { warp: end.warp, place })
    : translate("workbench:connectDoorsWizard.endTile", { x: end.x, z: end.z, place });
}

/** Door A or B: the place, the kind of end, and the door (by the picker of the door field) or the tile. */
function DoorEndField({ which, end, onChange, headers, matrices, depth, onError }: { which: "a" | "b"; end: EndForm; onChange: (change: Partial<EndForm>) => void; headers: HeaderSummary[]; matrices: readonly MatrixSummary[]; depth: Depth; onError: (e: unknown) => void }) {
  const { t } = useTranslation("workbench");
  const [picking, setPicking] = useState(false);
  const header = end.header === null ? null : (headers.find((h) => h.id === end.header) ?? null);
  const events = useEvents(header?.event_file_id ?? null, onError);
  const chosen = end.warp === null ? undefined : events?.warps.find((w) => w.index === end.warp);
  const items = useMemo<PickerItem[]>(() => (picking && events ? events.warps.map((w) => ({ id: w.index, label: doorText(w), area: "welt" as const })) : []), [picking, events]);
  // A tile with a door on it is that door: the kernel refuses a second one there.
  const taken = end.mode === "tile" && end.x !== null && end.z !== null ? warpOnTile(events?.warps, end.x, end.z) : null;
  const name = which.toUpperCase();
  return (
    <section className={parts.column} data-door-end={which}>
      <h3 className={parts.heading}>{t("connectDoorsWizard.doorName", { name })}</h3>
      <PlaceField headers={headers} value={end.header} onChange={(h) => onChange({ header: h, warp: null })} depth={depth} />
      <Segmented ariaLabel={t("connectDoorsWizard.doorName", { name })} size="s" options={modes()} value={end.mode} onChange={(mode) => onChange({ mode })} />
      {end.mode === "door" ? (
        <div className={parts.group}>
          <span className={parts.label}>{t("connectDoorsWizard.door")}</span>
          <p className={parts.text}>{chosen ? doorText(chosen) : end.warp !== null ? t("warpField.doorNumber", { value: end.warp }) : t("connectDoorsWizard.noDoor")}</p>
          <Button kind="secondary" size="s" disabled={header === null} onClick={() => setPicking(true)}>
            {t("connectDoorsWizard.chooseDoor")}
          </Button>
          {events !== null && events.warps.length === 0 && <p className={parts.note}>{t("connectDoorsWizard.noDoorsTakeTile")}</p>}
        </div>
      ) : (
        <>
          <TileFields x={end.x} z={end.z} size={placeTiles(header, matrices)} onChange={(tile) => onChange(tile)} />
          {taken && (
            <>
              <p className={parts.warn}>{t("connectDoorsWizard.tileTaken", { x: taken.x, z: taken.z, index: taken.index })}</p>
              <Button kind="secondary" size="s" onClick={() => onChange({ mode: "door", warp: taken.index })}>
                {t("connectDoorsWizard.takeDoor", { index: taken.index })}
              </Button>
            </>
          )}
        </>
      )}
      <Picker
        open={picking}
        onOpenChange={setPicking}
        title={t("connectDoorsWizard.chooseDoorName", { name })}
        items={items}
        placeholder={t("warpField.searchDoor")}
        emptyText={events === null ? t("inspectorField.loading") : t("connectDoorsWizard.noDoors")}
        onPick={(item) => onChange({ warp: Number(item.id) })}
      />
    </section>
  );
}

/**
 * „Tür verbinden“ in Erzählen: door A (a door of a place or a tile for a new one), door B (the same, in any place) with „Rückweg anlegen“
 * and „Türkacheln setzen“, and the summary; „Verbinden“ applies the recipe `connect_doors` (one undo step over both event files). The
 * scene draws the end on the shown matrix while the wizard is open.
 */
export function ConnectDoorsWizard({ depth, headers, matrices, prefill, onDraft, onDone, onShow, onStep, onCancel, onError }: ConnectDoorsWizardProps) {
  const { t } = useTranslation("workbench");
  const [a, setA] = useState<EndForm>(() => startEnd(prefill?.a));
  const [b, setB] = useState<EndForm>(() => startEnd(undefined));
  const [back, setBack] = useState(true);
  const [doorTiles, setDoorTiles] = useState(true);
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);

  const endA = doorEnd(a);
  const endB = doorEnd(b);
  const oneWayNew = !back && b.mode === "tile";
  const same = endA !== null && endB !== null && JSON.stringify(endA) === JSON.stringify(endB);
  const input: ConnectDoorsInput | null = endA !== null && endB !== null && !oneWayNew && !same ? { a: endA, b: endB, back, door_tiles: doorTiles } : null;
  const plan = useDraftPlan("connect_doors", outcome === null ? input : null, onDraft);

  async function apply() {
    if (input === null) return;
    setBusy(true);
    setRefusal(null);
    try {
      const result = await recipeApply<ConnectDoorsReport>("connect_doors", input);
      setOutcome({ kind: "done", outcome: result, input });
      try {
        await onDone(result, input);
      } catch (e) {
        onError(e);
      }
    } catch (e) {
      if (isLateFailure(e)) setOutcome({ kind: "partial", message: errorText(e) });
      else setRefusal(errorText(e));
    } finally {
      setBusy(false);
    }
  }

  const summaryLines =
    endA !== null && endB !== null
      ? [
          t("connectDoorsWizard.endLine", { name: "A", end: endText(a, headers) }),
          t("connectDoorsWizard.endLine", { name: "B", end: endText(b, headers) }),
          back ? t("wizardSpecs.doorBack") : t("connectDoorsWizard.oneWay"),
          doorTiles ? t("connectDoorsWizard.doorTiles") : t("connectDoorsWizard.noDoorTiles"),
        ]
      : [];

  const steps: WizardStep[] = [
    {
      id: "a",
      title: t("connectDoorsWizard.doorName", { name: "A" }),
      canNext: endA !== null,
      content: <DoorEndField which="a" end={a} onChange={(change) => setA((e) => ({ ...e, ...change }))} headers={headers} matrices={matrices} depth={depth} onError={onError} />,
    },
    {
      id: "b",
      title: t("connectDoorsWizard.doorName", { name: "B" }),
      canNext: input !== null,
      content: (
        <>
          <DoorEndField which="b" end={b} onChange={(change) => setB((e) => ({ ...e, ...change }))} headers={headers} matrices={matrices} depth={depth} onError={onError} />
          <section className={parts.column}>
            <h3 className={parts.heading}>{t("connectDoorsWizard.connection")}</h3>
            <Checkbox label={t("connectDoorsWizard.back")} checked={back} onChange={setBack} />
            <Checkbox label={t("connectDoorsWizard.setDoorTiles")} checked={doorTiles} onChange={setDoorTiles} />
            {/* The kernel's rule (R23): a new door needs a way back. */}
            {oneWayNew && <Notice kind="warning">{t("connectDoorsWizard.oneWayNew")}</Notice>}
            {same && <Notice kind="warning">{t("connectDoorsWizard.same")}</Notice>}
          </section>
        </>
      ),
    },
    {
      id: "summary",
      title: t("connectDoorsWizard.summary"),
      canNext: input !== null,
      content: (
        <section className={parts.column}>
          <Notice kind="info">
            <div className={parts.lines}>
              {summaryLines.map((line) => (
                <p key={line}>{line}</p>
              ))}
            </div>
          </Notice>
          <DraftNotes draft={plan.draft} error={plan.error} depth={depth} />
        </section>
      ),
    },
  ];

  let result = null;
  let show: (() => void) | undefined;
  if (outcome?.kind === "done") {
    const done = outcome;
    result = <DoneResult title={t("connectDoorsWizard.done")} {...doorsResult(done.outcome)} onStep={onStep && ((id) => onStep(id, done.outcome, done.input))} />;
    if (onShow) show = () => onShow(done.outcome, done.input);
  } else if (outcome?.kind === "partial") {
    result = <PartialResult message={outcome.message} depth={depth} />;
  }

  return (
    <WizardFrame
      name="connect_doors"
      title={t("connectDoorsWizard.title")}
      icon="door"
      steps={steps}
      step={step}
      onStep={(i) => {
        setRefusal(null);
        setStep(i);
      }}
      submitLabel={t("connectDoorsWizard.submit")}
      onSubmit={() => void apply()}
      onCancel={() => onCancel(outcome?.kind === "partial")}
      onShow={show}
      busy={busy}
      result={result}
      error={refusal}
      summary={endA !== null && endB !== null ? t("connectDoorsWizard.summaryLine", { a: endText(a, headers), b: endText(b, headers) }) : undefined}
    />
  );
}
