import { useState } from "react";
import type { CreatePhase, Created, RomInfo } from "../../../api";
import { DropHint, ProjectStart } from "../../../project/ProjectStart";
import type { StartWizardRequest } from "../../../project/ProjectStart";
import { StartServiceContext } from "../../../project/startService";
import type { StartService } from "../../../project/startService";
import { StartPane } from "../../frame/StartPane";
import { Button } from "../../primitives";
import styles from "../Gallery.module.css";

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
  createProject: async (_rom, dest, template, onProgress): Promise<Created> => {
    for (const phase of PHASES) {
      onProgress({ phase, done: 0, total: phase === "template" ? 0 : 1 });
      await wait(PHASE_MS);
    }
    return { root: dest, game_code: EXAMPLE_ROM.game_code, template, sha1: EXAMPLE_ROM.sha1, phases: PHASES.map((phase) => ({ phase, ms: PHASE_MS })) };
  },
};

/** "Neues Spiel aus ROM": the start with its main action, the drop frame, and the wizard in the start's own deck with example data. */
export function ProjectPage() {
  const [now] = useState(() => Date.now());
  // Closed at first: an open wizard takes the focus (and would scroll the page to it).
  const [wizard, setWizard] = useState<StartWizardRequest | null>(null);
  const [opened, setOpened] = useState<string | null>(null);
  const recent = [{ path: "/Projekte/platin-hack", name: "platin-hack", game: "Platin, DE", openedAt: now - HOUR }];
  return (
    <div className={styles.stack}>
      <section className={styles.block}>
        <h2 className={styles.blockTitle}>Start mit Hauptknopf</h2>
        <p className={styles.note}>„Neues Spiel aus ROM…“ ist die primäre Taste, „Projekt öffnen“ sekundär; darunter, was man mitbringen muss.</p>
        <div className={styles.startFrame}>
          <StartPane recent={recent} missing={new Set()} onOpen={noop} onOpenRecent={noop} onForget={noop} onNewGame={noop} />
        </div>
      </section>

      <section className={styles.block}>
        <h2 className={styles.blockTitle}>ROM ablegen</h2>
        <p className={styles.note}>Solange eine Datei über den Start gezogen wird, ist die ganze Fläche der Schacht, in den sie fällt.</p>
        <div className={styles.dropFrame}>
          <StartPane recent={recent} missing={new Set()} onOpen={noop} onOpenRecent={noop} onForget={noop} onNewGame={noop} />
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
