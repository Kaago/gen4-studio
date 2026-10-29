import { useState } from "react";
import type { CreatePhase, Created, PreviewDto, RomInfo } from "../../../api";
import { DropHint, ProjectStart } from "../../../project/ProjectStart";
import type { StartWizardRequest } from "../../../project/ProjectStart";
import { StartServiceContext } from "../../../project/startService";
import type { ProjectPreviewState } from "../../../project/useProjectPreview";
import { PreviewSection } from "../../../technik/TechnikArea";
import type { StartService } from "../../../project/startService";
import { StartPane } from "../../frame/StartPane";
import { Button } from "../../primitives";
import styles from "../Gallery.module.css";
import { drawExamplePreviews } from "./examplePreviews";

const noop = () => {};
const HOUR = 60 * 60 * 1000;
/** How long each phase of the example takes, so the progress can be watched. */
const PHASE_MS = 700;

const EXAMPLE_ROM: RomInfo = {
  title: "POKEMON PL",
  game_code: "CPUD",
  family: "pt",
  language: "german",
  revision: 0,
  size: 134_217_728,
  sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4",
  known_base: "Platin DE (Testprojekt)",
  support: { kind: "verified" },
};

const DAY = 24 * HOUR;
const NO_PREVIEW: PreviewDto = { source: null, png: null, standardMap: null };

const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));
const PHASES: CreatePhase[] = ["read", "extract", "template", "check"];

/** Example answers instead of the backend: every ROM is Platin DE, every folder is free, creating takes a few seconds. */
const EXAMPLE: StartService = {
  inspectRom: async () => {
    await wait(300);
    return EXAMPLE_ROM;
  },
  defaultProjectParent: async () => ({ parent: "/Users/du/Documents/Gen4 Studio", dspre_suffix: true }),
  checkProjectDest: async (_dest, romSize) => ({ state: "missing", free_bytes: 48_000_000_000, needed_bytes: Math.round(romSize * 2.5), ok: true }),
  createProject: async (_rom, dest, template, textProject, git, onProgress): Promise<Created> => {
    const phases: CreatePhase[] = [...PHASES, ...(textProject ? (["text_project"] as const) : []), ...(git ? (["git"] as const) : [])];
    for (const phase of phases) {
      onProgress({ phase, done: 0, total: phase === "template" ? 0 : 1 });
      await wait(PHASE_MS);
    }
    return { root: dest, game_code: EXAMPLE_ROM.game_code, template, sha1: EXAMPLE_ROM.sha1, phases: phases.map((phase) => ({ phase, ms: PHASE_MS })) };
  },
  projectPreviewGet: async () => NO_PREVIEW,
  projectPreviewSet: async () => NO_PREVIEW,
};

/** The project cards in every state: own picture (and Zuletzt), standard picture, placeholder with the name, not found. */
const CARDS = [
  { path: "/Projekte/platin-hack", name: "platin-hack", game: "Platin, DE", age: 20 * 60 * 1000 },
  { path: "/Projekte/johto-neu", name: "johto-neu", game: "HeartGold, DE", age: 3 * HOUR },
  { path: "/Projekte/erster-versuch", name: "erster-versuch", game: "Platin, EN", age: 2 * DAY },
  { path: "/Projekte/alte-kopie", name: "alte-kopie", game: "Diamant, DE", age: 9 * DAY },
];
/** The cards that have a standard picture (their start town): removing the own picture shows it again. */
const WITH_STANDARD = new Set(["/Projekte/platin-hack", "/Projekte/johto-neu"]);
const GONE = new Set(["/Projekte/alte-kopie"]);

/**
 * The project cards with example pictures the gallery draws itself; the card menu works on them: "Vorschaubild wählen…" puts the own
 * example picture on the card, "Vorschaubild entfernen" brings back the standard picture or the name.
 */
function ProjectCards({ now }: { now: number }) {
  const [pictures] = useState(drawExamplePreviews);
  const standard = (path: string): PreviewDto | null =>
    WITH_STANDARD.has(path) && pictures.standard !== null ? { source: "standard", png: pictures.standard, standardMap: null } : null;
  const own: PreviewDto | null = pictures.own === null ? null : { source: "eigen", png: pictures.own, standardMap: null };
  const [previews, setPreviews] = useState<ReadonlyMap<string, PreviewDto>>(() => {
    const start = new Map<string, PreviewDto>();
    if (own) start.set("/Projekte/platin-hack", own);
    const std = standard("/Projekte/johto-neu");
    if (std) start.set("/Projekte/johto-neu", std);
    return start;
  });
  const put = (path: string, dto: PreviewDto | null) =>
    setPreviews((before) => {
      const next = new Map(before);
      if (dto === null) next.delete(path);
      else next.set(path, dto);
      return next;
    });
  return (
    <StartPane
      recent={CARDS.map(({ age, ...card }) => ({ ...card, openedAt: now - age }))}
      missing={GONE}
      previews={previews}
      onOpen={noop}
      onOpenRecent={noop}
      onForget={noop}
      onChoosePreview={(path) => put(path, own)}
      onRemovePreview={(path) => put(path, standard(path))}
      onNewGame={noop}
    />
  );
}

/** The examples of Technik's section „Vorschaubild“: a project with its own picture, one with the standard picture, one without any. */
const TECHNIK_EXAMPLES = [
  { name: "platin-hack", start: "eigen", town: "Zweiblattdorf" },
  { name: "platin-neu", start: "standard", town: "Zweiblattdorf" },
  // Diamant has no recipe: no standard picture, and no town to name.
  { name: "alte-kopie", start: null, town: null },
] as const;

/**
 * Technik › Projekt › „Vorschaubild“ in its three states, with the example pictures (no backend): „Vorschaubild wählen…“ puts the own
 * example picture on, „Entfernen“ brings back the standard picture (or none, without a start town).
 */
function TechnikPreviews() {
  const [pictures] = useState(drawExamplePreviews);
  const dto = (source: PreviewDto["source"]): PreviewDto => {
    const png = source === "eigen" ? pictures.own : source === "standard" ? pictures.standard : null;
    return png === null ? NO_PREVIEW : { source, png, standardMap: null };
  };
  const [sources, setSources] = useState<PreviewDto["source"][]>(() => TECHNIK_EXAMPLES.map((e) => e.start));
  const put = (index: number, source: PreviewDto["source"]) => setSources((before) => before.map((s, i) => (i === index ? source : s)));
  return (
    <div className={styles.columns}>
      {TECHNIK_EXAMPLES.map((example, index) => {
        const state: ProjectPreviewState = {
          preview: dto(sources[index] ?? null),
          busy: false,
          change: (how) => put(index, how === "choose" ? "eigen" : example.town === null ? null : "standard"),
        };
        return <PreviewSection key={example.name} name={example.name} preview={state} town={example.town} />;
      })}
    </div>
  );
}

/** "Neues Spiel aus ROM": the start with its main action, the drop frame, and the wizard in the start's own deck with example data. */
export function ProjectPage() {
  const [now] = useState(() => Date.now());
  // Closed at first: an open wizard takes the focus (and would scroll the page to it).
  const [wizard, setWizard] = useState<StartWizardRequest | null>(null);
  const [opened, setOpened] = useState<string | null>(null);
  const recent = [{ path: "/Projekte/platin-hack", name: "platin-hack", game: "Platin, DE", openedAt: now - HOUR }];
  const cardProps = { previews: new Map<string, PreviewDto>(), onChoosePreview: noop, onRemovePreview: noop };
  return (
    <div className={styles.stack}>
      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Projektkarte</h2>
        <p className={styles.note}>
          Von links: eigenes Bild (und „Zuletzt“), Standardbild (das Kartenbild der Startstadt), ohne Bild der Name auf dem leeren Bildschirm,
          „nicht gefunden“ grau und ohne Farbe. Das Menü „…“ wählt oder entfernt das Bild; die Beispielbilder zeichnet die Galerie aus den
          Szenenfarben.
        </p>
        <div className={styles.startFrame}>
          <ProjectCards now={now} />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Technik: Vorschaubild</h2>
        <p className={styles.note}>
          Der Abschnitt in Technik › Projekt: von links mit eigenem Bild (dann auch „Entfernen“), mit dem Standardbild der Startstadt, ohne Bild
          (der Name auf dem leeren Bildschirm, der Satz ohne Ortsnamen). Die Knöpfe wechseln zwischen den Beispielbildern.
        </p>
        <TechnikPreviews />
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Start mit Hauptknopf</h2>
        <p className={styles.note}>„Neues Spiel aus ROM…“ ist die primäre Taste, „Projekt öffnen“ sekundär; darunter, was man mitbringen muss.</p>
        <div className={styles.startFrame}>
          <StartPane recent={recent} missing={new Set()} {...cardProps} onOpen={noop} onOpenRecent={noop} onForget={noop} onNewGame={noop} />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>ROM ablegen</h2>
        <p className={styles.note}>Solange eine Datei über den Start gezogen wird, ist die ganze Fläche der Schacht, in den sie fällt.</p>
        <div className={styles.dropFrame}>
          <StartPane recent={recent} missing={new Set()} {...cardProps} onOpen={noop} onOpenRecent={noop} onForget={noop} onNewGame={noop} />
          <DropHint />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Assistent im eigenen Deck</h2>
        <p className={styles.note}>
          Mit Beispieldaten: jede ROM ist Platin DE, jeder Ordner frei; Anlegen zeigt den Ablauf, danach das Ergebnis. {opened ? `Geöffnet: ${opened}` : ""}
        </p>
        <div className={styles.row}>
          <Button kind="secondary" icon="game" onClick={() => setWizard({ rom: "/Users/du/Roms/Platin.nds" })}>
            Mit abgelegter Beispiel-ROM öffnen
          </Button>
        </div>
        <div className={styles.projectFrame}>
          <StartServiceContext.Provider value={EXAMPLE}>
            <ProjectStart
              recent={recent}
              missing={new Set()}
              onOpen={noop}
              onOpenRecent={noop}
              onForget={noop}
              {...cardProps}
              wizard={wizard}
              onWizard={setWizard}
              onOpenCreated={(created) => {
                setOpened(created.root);
                setWizard(null);
              }}
            />
          </StartServiceContext.Provider>
        </div>
      </section>
    </div>
  );
}
