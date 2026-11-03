import { useEffect, useId, useMemo, useRef } from "react";
import type { FigureRef } from "../../api";
import type { StoryRecipeRequest } from "../../stories/recipes/storyRecipeModel";
import { NOT_A_TEXT_PROJECT } from "../../flags/flagWords";
import { useTextProject } from "../../textproject/TextProjectContext";
import { Button, DSScreen, Tooltip } from "../../ui/primitives";
import { pages } from "../text/textModel";
import { dialogSummary } from "../dialog/dialogModel";
import type { DialogTarget } from "../dialog/dialogModel";
import { useDialog } from "../dialog/useDialog";
import styles from "./DialogSection.module.css";

/**
 * The field „Dialog“ of a figure in Erzählen (a sign's is „Text“): the first page of its first message on the DS screen, a line with
 * what it says, and the way into the dialog editor. A common script, a trainer's battle and a script that cannot be read say where they
 * are edited instead. A figure without a script gets one with „Dialog anlegen…“: the recipe „Gespräch schaltet weiter“ with the figure
 * (`figure` and `onStoryRecipe` given; its story is chosen in the recipe's dialog). Outside a text project the recipe cannot go in: the
 * button is locked with the kernel's sentence (tooltip and description).
 */
export function DialogSection({
  target,
  revision,
  onEdit,
  onError,
  figure,
  onStoryRecipe,
}: {
  target: DialogTarget;
  revision: number;
  onEdit: (t: DialogTarget) => void;
  onError: (e: unknown) => void;
  /** The figure as the story recipes name it (an object's place and local id); absent: a sign, or no figure known. */
  figure?: FigureRef;
  /** Opens a step recipe (the app's dialog); absent: no „Dialog anlegen…“. */
  onStoryRecipe?: (request: StoryRecipeRequest) => void;
}) {
  const { dialog, error } = useDialog(target, revision);
  const variant = target.kind === "bg" ? "sign" : "textbox";
  const first = dialog?.messages[0]?.text ?? null;
  const shown = useMemo(() => (first === null ? [] : pages(first)), [first]);
  // A read that failed is said once per failure, like the editor's own read.
  const report = useRef(onError);
  report.current = onError;
  useEffect(() => {
    if (error !== null) report.current(error);
  }, [error]);

  const source = dialog?.source.kind ?? null;
  const textProject = useTextProject().active;
  const reasonId = useId();
  const create = (
    <Button
      kind="quiet"
      size="s"
      disabled={!textProject}
      aria-describedby={textProject ? undefined : reasonId}
      onClick={() => figure !== undefined && onStoryRecipe?.({ recipe: "talk_advances", prefill: { figure } })}
    >
      Dialog anlegen…
    </Button>
  );
  return (
    <section className={styles.section} data-dialog-section="">
      <h3 className={styles.title}>{target.kind === "bg" ? "Text" : "Dialog"}</h3>
      <div className={styles.preview} data-dialog-preview={variant}>
        <DSScreen variant={first === null ? "empty" : variant} pages={shown} page={0} />
      </div>
      <p className={styles.summary}>{error !== null ? "Dialog nicht lesbar." : dialog === null ? "Lese …" : dialogSummary(dialog)}</p>
      {source === "local" && (
        <div className={styles.actions}>
          <Button kind="quiet" size="s" onClick={() => onEdit(target)}>
            Dialog bearbeiten…
          </Button>
        </div>
      )}
      {source === "none" && figure !== undefined && onStoryRecipe !== undefined && (
        <div className={styles.actions}>
          {textProject ? (
            create
          ) : (
            <>
              <Tooltip content={NOT_A_TEXT_PROJECT}>
                <span className={styles.lockedWrap}>{create}</span>
              </Tooltip>
              <span id={reasonId} hidden>
                {NOT_A_TEXT_PROJECT}
              </span>
            </>
          )}
        </div>
      )}
    </section>
  );
}
