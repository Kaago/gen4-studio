import { useEffect, useId, useRef, useState } from "react";
import i18next from "i18next";
import { useTranslation } from "react-i18next";
import { errorText, worldAdopt, worldBake, worldToBinary } from "../../api";
import type { WorldAdoptReport, WorldBakeReport } from "../../api";
import { t } from "../../i18n";
import { Button, Notice, Section } from "../../ui/primitives";
import { useSectionOpen } from "./foldState";
import styles from "./WorldSourceSection.module.css";

/** Why „Karte als Quelle übernehmen“ is locked outside a text project. */
export const adoptNeedsTextProject = (): string => t("workbench:worldSourceSection.needsTextProject");

/** A number with the thousands separator of the language that is shown. */
const number = (n: number) => n.toLocaleString(i18next.language);

/** The notice after „Karte als Quelle übernehmen“: how many cells the world's whole steps put off the sampled height. */
export function adoptedText(report: WorldAdoptReport): string {
  const n = report.deviations.length;
  if (n === 0) return t("workbench:worldSourceSection.adopted");
  return t("workbench:worldSourceSection.adoptedDeviations", { count: n, n: number(n) });
}

/** The notice after „Als glTF übernehmen (Form 2)“: the polygons of the vanilla model and the faces the baker drew. */
export function adoptedGltfText(report: WorldAdoptReport): string {
  const f = report.form2;
  if (f == null) return adoptedText(report);
  return t("workbench:worldSourceSection.adoptedGltf", { polygons: number(f.vanilla.polygons), faces: number(f.baked.faces) });
}

/** The bake as read: the report, or the command's refusal (a world whose rows do not read). */
type BakeRead = { map: number; revision: number; bake: WorldBakeReport | null; error: string | null };

/** The bake line of a world map: model and terrain sizes (the BDHC payload, as „Budget“ and the check `world.budget` count it) when the baker took it, else the sentence of its first error. */
function BakeLine({ read }: { read: BakeRead | undefined }) {
  if (read === undefined) return <p className={styles.note}>{t("workbench:worldSourceSection.reading")}</p>;
  if (read.error !== null) {
    return (
      <div data-world-bake="refused">
        <Notice kind="error">{read.error}</Notice>
      </div>
    );
  }
  const bake = read.bake;
  if (bake === null) return null;
  const first = bake.messages.find((m) => m.level === "error");
  if (!bake.ok || first !== undefined) {
    return (
      <div data-world-bake="refused">
        <Notice kind="error">{first?.message ?? t("workbench:worldSourceSection.bakeRefused")}</Notice>
      </div>
    );
  }
  return (
    <p className={styles.text} data-world-bake="ok">
      {t("workbench:worldSourceSection.baked", { model: number(bake.model_bytes), terrain: number(bake.bdhc_payload) })}
    </p>
  );
}

/**
 * The section „Quelle“ of a map cell: a binary map (from the ROM or DSPRE) offers „Karte als Quelle übernehmen“ (Form 1, one undo step;
 * locked outside a text project with the reason) and „Als glTF übernehmen (Form 2)“ (its refusal is a notice here), a world map shows its
 * bake and offers „Als binäre Karte übernehmen“. The bake is read
 * (`world_bake`, what the store holds) when the map or `revision` changes, never per render. After either command `onChanged` lets the
 * app catch up (scene, pictures, dirty count); a refusal goes to `onError`.
 */
export function WorldSourceSection({
  map,
  world,
  textProject,
  revision = 0,
  onChanged,
  onError,
}: {
  map: number;
  world: boolean;
  textProject: boolean;
  revision?: number;
  onChanged?: (map: number) => void | Promise<void>;
  onError?: (e: unknown) => void;
}) {
  // Renders the section again when the language changes; the words are read at render time.
  useTranslation("workbench");
  const [open, toggle] = useSectionOpen("matrixCell:source", true);
  const [working, setWorking] = useState(false);
  // The notice of the last adoption, for the map it belongs to.
  const [adopted, setAdopted] = useState<{ map: number; text: string } | null>(null);
  // The refusal of Form 2, for the map it belongs to (Form 1's refusal goes to `onError`).
  const [refused, setRefused] = useState<{ map: number; text: string } | null>(null);
  const [read, setRead] = useState<BakeRead | undefined>();
  const reasonId = useId();
  const report = useRef(onError);
  report.current = onError;

  useEffect(() => {
    if (!world) return;
    let current = true;
    worldBake(map).then(
      (bake) => current && setRead({ map, revision, bake, error: null }),
      (e: unknown) => current && setRead({ map, revision, bake: null, error: errorText(e) }),
    );
    return () => {
      current = false;
    };
  }, [map, world, revision]);

  // The notice belongs to the world it made: an undo (or any way back to binary) takes it along.
  useEffect(() => {
    if (!world) setAdopted(null);
  }, [world]);

  async function run(op: () => Promise<unknown>, refusal?: (e: unknown) => void) {
    if (working) return;
    setWorking(true);
    setRefused(null);
    try {
      await op();
      await onChanged?.(map);
    } catch (e) {
      (refusal ?? report.current)?.(e);
    } finally {
      setWorking(false);
    }
  }

  const adopt = () =>
    run(async () => {
      const done = await worldAdopt(map);
      setAdopted({ map, text: adoptedText(done) });
    });
  const adoptGltf = () =>
    run(
      async () => {
        const done = await worldAdopt(map, "gltf");
        setAdopted({ map, text: adoptedGltfText(done) });
      },
      (e) => setRefused({ map, text: errorText(e) }),
    );
  const toBinary = () =>
    run(async () => {
      await worldToBinary(map);
      setAdopted(null);
    });

  const notice = adopted?.map === map ? adopted.text : null;
  const shown = read?.map === map ? read : undefined;
  return (
    <div data-world-source={world ? "world" : "binary"}>
      <Section title={t("workbench:worldSourceSection.title")} open={open} onToggle={toggle}>
        <div className={styles.body}>
          {world ? (
            <>
              <p className={styles.text}>{t("workbench:worldSourceSection.worldMap")}</p>
              <BakeLine read={shown} />
              <div className={styles.actions}>
                <Button kind="secondary" size="s" aria-disabled={working || undefined} onClick={() => void toBinary()}>
                  {t("workbench:worldSourceSection.toBinary")}
                </Button>
              </div>
            </>
          ) : (
            <>
              <p className={styles.text}>{t("workbench:worldSourceSection.binaryMap")}</p>
              <div className={styles.actions}>
                <Button
                  kind="secondary"
                  size="s"
                  data-world-adopt=""
                  disabled={!textProject}
                  aria-disabled={(textProject && working) || undefined}
                  aria-describedby={textProject ? undefined : reasonId}
                  onClick={() => textProject && void adopt()}
                >
                  {t("workbench:worldSourceSection.adopt")}
                </Button>
                <Button
                  kind="secondary"
                  size="s"
                  data-world-adopt="gltf"
                  disabled={!textProject}
                  aria-disabled={(textProject && working) || undefined}
                  aria-describedby={textProject ? undefined : reasonId}
                  onClick={() => textProject && void adoptGltf()}
                >
                  {t("workbench:worldSourceSection.adoptGltf")}
                </Button>
              </div>
              {!textProject && (
                <p id={reasonId} className={styles.note}>
                  {adoptNeedsTextProject()}
                </p>
              )}
            </>
          )}
          {notice !== null && <Notice kind="success">{notice}</Notice>}
          {refused?.map === map && <Notice kind="error">{refused.text}</Notice>}
        </div>
      </Section>
    </div>
  );
}
