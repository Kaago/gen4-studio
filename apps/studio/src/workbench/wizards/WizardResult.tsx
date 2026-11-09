import type { NextStep } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { Notice, RecipeButton } from "../../ui/primitives";
import type { Area, IconName } from "../../ui/primitives";
import { RECIPE_BAR } from "../recipes";
import { termLabel } from "../terms";
import styles from "./WizardResult.module.css";

interface DoneProps {
  /** What was made, e.g. "Gebiet angelegt". */
  title: string;
  lines: string[];
  next: string[];
  /** The recipes that make sense next (the kernel's table): a button each, a locked one says why. */
  steps?: NextStep[];
  /** A step's button was pressed (locked ones have no click). */
  onStep?: (id: string) => void;
}

/** The symbol and area of a step that is no recipe of the bar (3d.5): the dialog editor, a story, playing. */
const OTHER_STEPS: Record<string, { icon: IconName; area: Area }> = {
  adjust_terrain: { icon: "paint", area: "welt" },
  edit_dialog: { icon: "texts", area: "texte" },
  story_join: { icon: "stories", area: "geschichten" },
  play_here: { icon: "play", area: "welt" },
};

/** A step's symbol and area: the bar's for its recipe, else its own (`OTHER_STEPS`), else the world's with a plus. */
function stepLook(id: string): { icon: IconName; area: Area } {
  const entry = RECIPE_BAR.find((r) => r.id === id);
  return entry ?? OTHER_STEPS[id] ?? { icon: "plus", area: "welt" };
}

/**
 * The result step of a wizard that created everything: what it made and what to do next, the recipes as buttons first, the lines of text
 * after them (the frame adds "Zeigen" and "Hier spielen").
 */
export function DoneResult({ title, lines, next, steps = [], onStep }: DoneProps) {
  return (
    <div className={styles.result}>
      <Notice kind="success" title={title}>
        {lines.map((line) => (
          <p key={line} className={styles.line}>
            {line}
          </p>
        ))}
      </Notice>
      <h3 className={styles.heading}>Als Nächstes</h3>
      {steps.length > 0 && (
        <div className={styles.steps} data-next-steps="">
          {steps.map((s) => (
            <RecipeButton
              key={s.id}
              id={s.id}
              label={s.label}
              icon={stepLook(s.id).icon}
              area={stepLook(s.id).area}
              reason={s.available ? null : (s.reason ?? "Noch nicht verfügbar")}
              onClick={() => onStep?.(s.id)}
              size="s"
            />
          ))}
        </div>
      )}
      <ul className={styles.next}>
        {next.map((line) => (
          <li key={line}>{line}</li>
        ))}
      </ul>
    </div>
  );
}

/** The result step of a wizard that failed after it had created members: what the backend says stayed behind, and where to finish by hand. */
export function PartialResult({ message, depth = "tuefteln" }: { message: string; depth?: Depth }) {
  return (
    <div className={styles.result}>
      <Notice kind="warning" title="Anlegen unterbrochen">
        <p className={styles.line}>{message}</p>
      </Notice>
      <p className={styles.line}>
        Was schon angelegt ist, bleibt bestehen und lässt sich nicht rückgängig machen.{" "}
        {!depthAtLeast(depth, "tuefteln") ? "Den Rest in der Welt von Hand fertigstellen." : `Den Rest im ${termLabel("matrix")}-Tab und im ${termLabel("header")}-Editor von Hand fertigstellen.`}
      </p>
    </div>
  );
}
