import { useEffect, useMemo, useState } from "react";
import { errorText, nameTables, recipeApply } from "../../api";
import type { DraftDto, HeaderSummary, MatrixSummary, NameTablesDto, NewTrainerHereInput, NewTrainerHereReport, ProjectInfo, RecipeOutcome } from "../../api";
import { NAME_MAX, PARTY_MAX, className, classOptions, nameCount, nameError } from "../../data/trainer/trainerModel";
import type { Depth } from "../../ui/frame/shellState";
import { Badge, Button, Notice, NumberField, Picker, PokemonCard, Segmented, SelectField, TextField } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import { headerLabel } from "../headers";
import { FACINGS } from "../inspector/eventFields";
import { FigureLookField } from "../inspector/FigureLookField";
import { MessageEditor } from "../text/MessageEditor";
import { DraftNotes } from "./DraftNotes";
import { PlaceField, TileFields, placeTiles } from "./PlaceField";
import { TextPreview } from "./TextPreview";
import { useDraftPlan } from "./useDraftPlan";
import { DoneResult, PartialResult } from "./WizardResult";
import { WizardFrame } from "./WizardFrame";
import type { WizardStep } from "./WizardFrame";
import { defaultLook, isLateFailure, trainerResult } from "./wizardSpecs";
import parts from "./WizardStep.module.css";
import figure from "./NewFigureWizard.module.css";
import styles from "./NewTrainerHereWizard.module.css";

export interface NewTrainerHereWizardProps {
  depth: Depth;
  headers: HeaderSummary[];
  matrices: readonly MatrixSummary[];
  /** The open project: its family names the default look of a trainer. */
  project: ProjectInfo | null;
  /** Where the trainer stands when the wizard opens: the selected place, the tile last chosen in the scene. */
  prefill?: { header?: number; tile?: { x: number; z: number } };
  onDraft: (draft: DraftDto | null) => void;
  /** The trainer stands (a new trainer behind the barrier, then one undo step): the app reads what the recipe changed again. */
  onDone: (outcome: RecipeOutcome<NewTrainerHereReport>, input: NewTrainerHereInput) => Promise<void>;
  /** „Zeigen“ on the result: the app shows the place of the trainer. */
  onShow?: (outcome: RecipeOutcome<NewTrainerHereReport>, input: NewTrainerHereInput) => void;
  onStep?: (id: string, outcome: RecipeOutcome<NewTrainerHereReport>, input: NewTrainerHereInput) => void;
  /** The wizard is left; `written` says it changed the project without the app having read it again (the trainer of a partial failure). */
  onCancel: (written: boolean) => void;
  onError: (e: unknown) => void;
}

/** One Pokémon of the team as the wizard asks for it; the species is null until one is picked. */
interface TeamRow {
  species: number | null;
  level: number;
}

interface TrainerForm {
  cls: number;
  name: string;
  party: TeamRow[];
  texts: { before: string; lose: string; after: string };
  header: number | null;
  x: number | null;
  z: number | null;
  dir: number;
  sight: number;
  /** Null: the family's default trainer look (the kernel's). */
  look: number | null;
}

type Outcome = { kind: "done"; outcome: RecipeOutcome<NewTrainerHereReport>; input: NewTrainerHereInput } | { kind: "partial"; message: string };

/** The level a new row of the team starts with. */
const FIRST_LEVEL = 5;
/** The sight of a new trainer (the kernel's default, `DEFAULT_SIGHT` of `trainerScript.ts`), and its bounds. */
const SIGHT = { start: 3, min: 1, max: 15 };
/** The clear code a text before the battle ends on (▼): its box waits for a button. */
const CLEAR = "\\r";
const FACING_OPTIONS = FACINGS.map((f) => ({ value: String(f.value), label: f.label }));

/** The text before the battle is fine when it is empty (nothing is written) or ends on ▼. */
const beforeOk = (text: string) => text === "" || text.endsWith(CLEAR);

/** The input of `new_trainer_here` the form makes; null while something is missing or refused. */
function trainerInput(form: TrainerForm): NewTrainerHereInput | null {
  const party = form.party.flatMap((row) => (row.species === null ? [] : [{ species: row.species, level: row.level }]));
  if (form.name.trim() === "" || nameError(form.name) !== null) return null;
  if (party.length === 0 || party.length !== form.party.length || !beforeOk(form.texts.before)) return null;
  if (form.header === null || form.x === null || form.z === null) return null;
  return { header: form.header, x: form.x, z: form.z, dir: form.dir, sight: form.sight, class: form.cls, name: form.name, party, texts: form.texts, graphics_id: form.look };
}

/** A species by its name from the name table, else its number. */
const speciesName = (names: NameTablesDto | null, species: number) => names?.species[species]?.trim() || `Pokémon ${species}`;

/**
 * „Neuer Trainer hier“ in Erzählen: the trainer („Trainer“: class and name), its team („Team“: one to six Pokémon by species and level),
 * its battle texts („Texte“), where it stands („Wo“: place, tile, facing, sight, look) and the summary with the barrier; „Aufstellen“
 * applies the recipe `new_trainer_here`. The trainer itself cannot be taken back (the kernel's barrier „Trainer N angelegt“); its team,
 * texts and figure are one undo step behind it.
 */
export function NewTrainerHereWizard({ depth, headers, matrices, project, prefill, onDraft, onDone, onShow, onStep, onCancel, onError }: NewTrainerHereWizardProps) {
  const [form, setForm] = useState<TrainerForm>(() => ({
    cls: 0,
    name: "",
    party: [{ species: null, level: FIRST_LEVEL }],
    texts: { before: "", lose: "", after: "" },
    header: prefill?.header ?? null,
    x: prefill?.tile?.x ?? null,
    z: prefill?.tile?.z ?? null,
    dir: 1,
    sight: SIGHT.start,
    look: null,
  }));
  const [names, setNames] = useState<NameTablesDto | null>(null);
  const [picking, setPicking] = useState<number | null>(null);
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const set = (change: Partial<TrainerForm>) => setForm((f) => ({ ...f, ...change }));
  const setRow = (i: number, change: Partial<TeamRow>) => setForm((f) => ({ ...f, party: f.party.map((row, j) => (j === i ? { ...row, ...change } : row)) }));
  const setText = (key: keyof TrainerForm["texts"], text: string) => setForm((f) => ({ ...f, texts: { ...f.texts, [key]: text } }));

  useEffect(() => {
    let live = true;
    nameTables().then(
      (tables) => live && setNames(tables),
      (e) => live && onError(e),
    );
    return () => {
      live = false;
    };
  }, []);

  const classes = names?.classes ?? [];
  const species = useMemo<PickerItem[]>(
    () => (picking === null || names === null ? [] : names.species.flatMap((name, i) => (i === 0 || name.trim() === "" ? [] : [{ id: i, label: name.trim(), tech: String(i), area: "pokemon" as const }]))),
    [picking, names],
  );
  const place = form.header === null ? null : (headers.find((h) => h.id === form.header) ?? null);
  const input = trainerInput(form);
  const plan = useDraftPlan("new_trainer_here", outcome === null ? input : null, onDraft);
  const look = form.look ?? defaultLook(project?.family, "trainer");
  const tooLong = nameError(form.name);
  const located = place !== null && form.x !== null && form.z !== null;
  const teamDone = form.party.every((row) => row.species !== null);

  async function apply() {
    if (input === null) return;
    setBusy(true);
    setRefusal(null);
    try {
      const result = await recipeApply<NewTrainerHereReport>("new_trainer_here", input);
      setOutcome({ kind: "done", outcome: result, input });
      try {
        await onDone(result, input);
      } catch (e) {
        onError(e);
      }
    } catch (e) {
      // After the trainer was created: it stays (behind the barrier), the wizard says what failed.
      if (isLateFailure(e)) setOutcome({ kind: "partial", message: errorText(e) });
      else setRefusal(errorText(e));
    } finally {
      setBusy(false);
    }
  }

  const team = form.party.map((row) => (row.species === null ? "?" : `${speciesName(names, row.species)} Lv. ${row.level}`)).join(", ");
  const who = `${form.name.trim() || "Ohne Namen"} (${className(classes, form.cls)})`;
  const summaryLines = [
    `${who} mit ${team}`,
    located ? `„${headerLabel(place)}“, Kachel x ${form.x}, z ${form.z}, Blick ${FACINGS.find((f) => f.value === form.dir)?.label ?? form.dir}, Sichtweite ${form.sight}` : "Noch kein Ort und keine Kachel.",
    form.texts.before === "" ? "Sagt vor dem Kampf nichts." : "Sagt vor dem Kampf etwas und kämpft, sobald er dich sieht.",
  ];

  const steps: WizardStep[] = [
    {
      id: "trainer",
      title: "Trainer",
      canNext: form.name.trim() !== "" && tooLong === null,
      content: (
        <section className={parts.column}>
          <h3 className={parts.heading}>Wer</h3>
          <div className={parts.field}>
            <SelectField label="Klasse" showLabel value={form.cls} options={classOptions(classes, form.cls)} onChange={(cls) => set({ cls })} />
          </div>
          <div className={parts.field}>
            <TextField label="Name" showLabel value={form.name} onChange={(name) => set({ name })} onCommit={(name) => set({ name })} error={tooLong ?? undefined} hint={tooLong === null ? `${nameCount(form.name)} (höchstens ${NAME_MAX})` : undefined} />
          </div>
        </section>
      ),
    },
    {
      id: "team",
      title: "Team",
      canNext: teamDone,
      content: (
        <section className={`${parts.column} ${parts.wide}`}>
          <h3 className={parts.heading}>Team (1 bis {PARTY_MAX})</h3>
          <div className={styles.team}>
            {form.party.map((row, i) => (
              <div key={i} className={styles.row} data-team-row={i}>
                <PokemonCard
                  state={row.species === null ? "empty" : "idle"}
                  species={row.species ?? undefined}
                  name={row.species === null ? undefined : speciesName(names, row.species)}
                  level={row.species === null ? undefined : row.level}
                  slot={i + 1}
                  ariaLabel={row.species === null ? `Platz ${i + 1}: leer` : `Platz ${i + 1}: ${speciesName(names, row.species)}, Level ${row.level}`}
                  onClick={() => setPicking(i)}
                />
                <Button kind="secondary" size="s" onClick={() => setPicking(i)}>
                  Art wählen…
                </Button>
                <div className={styles.level}>
                  <NumberField label="Level" min={1} max={100} value={row.level} onCommit={(level) => setRow(i, { level })} />
                </div>
                <Button kind="quiet" size="s" aria-label={`Pokémon ${i + 1} entfernen`} disabled={form.party.length === 1} onClick={() => set({ party: form.party.filter((_, j) => j !== i) })}>
                  Entfernen
                </Button>
              </div>
            ))}
          </div>
          <Button kind="secondary" size="s" icon="plus" disabled={form.party.length >= PARTY_MAX} onClick={() => set({ party: [...form.party, { species: null, level: FIRST_LEVEL }] })}>
            Pokémon hinzufügen
          </Button>
          {!teamDone && <p className={parts.note}>Jede Zeile braucht eine Art.</p>}
          <Picker
            open={picking !== null}
            onOpenChange={(open) => !open && setPicking(null)}
            title="Art wählen"
            items={species}
            placeholder="Art oder Nummer suchen"
            emptyText={names === null ? "Lädt …" : "Keine Art gefunden."}
            onPick={(item) => picking !== null && setRow(picking, { species: Number(item.id) })}
          />
        </section>
      ),
    },
    {
      id: "texts",
      title: "Texte",
      canNext: beforeOk(form.texts.before),
      content: (
        <>
          <section className={`${parts.column} ${parts.wide}`}>
            <h3 className={parts.heading}>Vor dem Kampf</h3>
            <div className={figure.text}>
              <MessageEditor label="Vor dem Kampf" value={form.texts.before} onCommit={async () => true} onDraft={(text) => setText("before", text)} />
            </div>
            <p className={beforeOk(form.texts.before) ? parts.note : parts.warn}>Der Text vor dem Kampf endet auf ▼ (Umschalt+Enter), sonst schließt seine Box sofort.</p>
            <h3 className={parts.heading}>Verloren</h3>
            <div className={figure.text}>
              <MessageEditor label="Verloren" value={form.texts.lose} onCommit={async () => true} onDraft={(text) => setText("lose", text)} />
            </div>
            <h3 className={parts.heading}>Nach dem Kampf</h3>
            <div className={figure.text}>
              <MessageEditor label="Nach dem Kampf" value={form.texts.after} onCommit={async () => true} onDraft={(text) => setText("after", text)} />
            </div>
          </section>
          <section className={parts.column}>
            <h3 className={parts.heading}>Vorschau</h3>
            <TextPreview text={form.texts.before} label="Vor dem Kampf" />
          </section>
        </>
      ),
    },
    {
      id: "where",
      title: "Wo",
      canNext: input !== null,
      content: (
        <>
          <section className={parts.column}>
            <h3 className={parts.heading}>Wo</h3>
            <PlaceField headers={headers} value={form.header} onChange={(header) => set({ header })} depth={depth} />
            <TileFields x={form.x} z={form.z} size={placeTiles(place, matrices)} onChange={(tile) => set(tile)} />
          </section>
          <section className={parts.column}>
            <h3 className={parts.heading}>Wie</h3>
            <FigureLookField value={look} dir={form.dir} onCommit={(graphicsId) => set({ look: graphicsId })} />
            <div className={parts.group}>
              <span className={parts.label}>Blickrichtung</span>
              <Segmented ariaLabel="Blickrichtung" size="s" options={FACING_OPTIONS} value={String(form.dir)} onChange={(dir) => set({ dir: Number(dir) })} />
            </div>
            <div className={parts.field}>
              <NumberField label="Sichtweite" showLabel min={SIGHT.min} max={SIGHT.max} value={form.sight} hint={`${SIGHT.min} bis ${SIGHT.max} Kacheln`} onCommit={(sight) => set({ sight })} />
            </div>
          </section>
        </>
      ),
    },
    {
      id: "summary",
      title: "Zusammenfassung",
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
          <p className={parts.loss} data-trainer-barrier="">
            <Badge kind="irreversible" />
            <span>Trainer anlegen lässt sich nicht rückgängig machen; Team, Texte und Figur nimmt ein Schritt zurück.</span>
          </p>
          <DraftNotes draft={plan.draft} error={plan.error} depth={depth} />
        </section>
      ),
    },
  ];

  let result = null;
  let show: (() => void) | undefined;
  if (outcome?.kind === "done") {
    const done = outcome;
    result = <DoneResult title="Trainer aufgestellt" {...trainerResult(done.outcome)} onStep={onStep && ((id) => onStep(id, done.outcome, done.input))} />;
    if (onShow) show = () => onShow(done.outcome, done.input);
  } else if (outcome?.kind === "partial") {
    result = <PartialResult message={outcome.message} depth={depth} />;
  }

  return (
    <WizardFrame
      name="new_trainer_here"
      title="Neuer Trainer hier"
      icon="pokemon"
      steps={steps}
      step={step}
      onStep={(i) => {
        setRefusal(null);
        setStep(i);
      }}
      submitLabel="Aufstellen"
      onSubmit={() => void apply()}
      onCancel={() => onCancel(outcome?.kind === "partial")}
      onShow={show}
      busy={busy}
      result={result}
      error={refusal}
      summary={`Neuer Trainer ${who}`}
      irreversible
    />
  );
}
