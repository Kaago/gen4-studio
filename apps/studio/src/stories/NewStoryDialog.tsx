import { useEffect, useRef, useState } from "react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import { errorText, freeFlag, isCmdError, recipeApply, storyCreate } from "../api";
import type { FlagKind, NewStoryInput, NewStoryReport, Progress, ProjectInfo, StoryReport } from "../api";
import { FlagField } from "../flags/FlagField";
import { kindWord } from "../flags/flagWords";
import { useFlagList } from "../flags/useFlagList";
import { Dialog, Notice, Segmented, TextField } from "../ui/primitives";
import { flagIdOf, freeVarsText } from "./storyWords";
import styles from "./StoriesSurface.module.css";

/**
 * What „Geschichte anlegen“ asks for: a variable story is the recipe „Neue Geschichte“ (`new_story`: it names the variable `VAR_STORY_…`
 * and writes the step „Anfang“), a flag story `story_create` as in 3d.6.
 */
export type NewStoryRequest = { via: "recipe"; input: NewStoryInput } | { via: "story"; req: { name: string; progress: Progress; notes?: string } };

/**
 * Creates the story a request asks for: a variable story by the recipe (its outcome as the report of a create: id, number, files and
 * undo step; a recipe has no timing and no warnings), a flag story by `story_create`. Resolves to the report and the story's name.
 */
export async function createStory(request: NewStoryRequest): Promise<{ report: StoryReport; name: string }> {
  if (request.via === "recipe") {
    const outcome = await recipeApply<NewStoryReport>("new_story", request.input);
    const { story: id, number, files } = outcome.report;
    return { report: { id, number, files, change: outcome.change, ms: 0, warnings: [] }, name: request.input.name };
  }
  return { report: await storyCreate(request.req), name: request.req.name };
}

/** The pool's answer when the dialog opened: the smallest free variable and how many are free, or why there is none (`empty`: the pool has
 * nothing to give, the recipe cannot run). */
type Free = { number: number; poolFree: number } | { refusal: string; empty: boolean };

/** The kinds of progress, in the order the switch offers them; the words come from `kindWord`. */
const KINDS = ["var", "flag"] as const;

export type NewStoryDialogProps = {
  open: boolean;
  onClose: () => void;
  /** Creates the story (`story_create`, seconds under load); a refusal rejects with the kernel's sentence and the dialog stays. */
  onCreate: (req: NewStoryRequest) => Promise<void>;
  project: ProjectInfo | null;
  revision: number;
};


/**
 * „Neue Geschichte“: the name (required), the progress as a variable or a flag (the field of 3d.1 with its picker and „Freie Variable
 * nehmen“; 0 is nothing chosen) and notes. A variable comes from the pool: opening takes its smallest free one into the field (it stays
 * changeable) and says how many are free („Noch 20 Variablen frei“); with none free it says so and „Geschichte anlegen“ stays locked for a
 * variable. „Geschichte anlegen“ is locked without a name or a progress and while the create runs (seconds: no second one); a refusal of the
 * backend stays in the dialog as an error, the dialog open. A variable goes by its number to the recipe; a flag by its id (its own name,
 * else its pret name, else its fallback id: each resolves to the number in the kernel).
 */
export function NewStoryDialog({ open, onClose, onCreate, project, revision }: NewStoryDialogProps): JSX.Element {
  const { t } = useTranslation("stories");
  const [name, setName] = useState("");
  const [kind, setKind] = useState<FlagKind>("var");
  const [number, setNumber] = useState(0);
  const [notes, setNotes] = useState("");
  const [refusal, setRefusal] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [free, setFree] = useState<Free | null>(null);
  const busy = useRef(false);
  // The variant shown now: a pool's answer that comes after a switch to a flag does not fill the flag field.
  const kindNow = useRef(kind);
  kindNow.current = kind;
  // The names of the flags (the field reads the same list): the id a flag story keeps. A variable goes by its number.
  const { list } = useFlagList("flag", { project: open && kind === "flag" && number !== 0 ? project : null, revision });

  // Every opening starts empty.
  const [wasOpen, setWasOpen] = useState(open);
  if (open !== wasOpen) {
    setWasOpen(open);
    if (open) {
      setName("");
      setKind("var");
      setNumber(0);
      setNotes("");
      setRefusal(null);
      setFree(null);
    }
  }

  // Each opening asks the pool for its smallest free variable; the field takes it while it still shows none.
  useEffect(() => {
    if (!open) return;
    let live = true;
    freeFlag("var").then(
      (dto) => {
        if (!live) return;
        setFree({ number: dto.number, poolFree: dto.pool_free });
        if (kindNow.current === "var") setNumber((n) => (n === 0 ? dto.number : n));
      },
      (e: unknown) => live && setFree({ refusal: errorText(e), empty: isCmdError(e) && e.code === "pool_empty" }),
    );
    return () => {
      live = false;
    };
  }, [open]);

  const poolEmpty = kind === "var" && free !== null && "empty" in free && free.empty;
  const ready = name.trim() !== "" && number !== 0 && !poolEmpty;
  const submit = async () => {
    if (busy.current || !ready) return;
    busy.current = true;
    setRunning(true);
    setRefusal(null);
    const text = notes.trim();
    const withNotes = text === "" ? {} : { notes: text };
    const request: NewStoryRequest =
      kind === "var"
        ? { via: "recipe", input: { name: name.trim(), var: number, ...withNotes } }
        : { via: "story", req: { name: name.trim(), progress: { flag: flagIdOf("flag", number, list?.entries.find((e) => e.number === number)) }, ...withNotes } };
    try {
      await onCreate(request);
      onClose();
    } catch (e) {
      setRefusal(errorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next && !busy.current) onClose();
      }}
      dismissible={!running}
      title={t("newStoryDialog.title")}
      description={t("newStoryDialog.description")}
      data={{ "data-new-story-dialog": "" }}
      actions={[
        { label: t("newStoryDialog.cancel"), onClick: onClose, disabled: running },
        { label: t("newStoryDialog.create"), kind: "primary", onClick: () => void submit(), disabled: running || !ready },
      ]}
    >
      <div className={styles.dialog}>
        <TextField label={t("newStoryDialog.name")} showLabel value={name} onCommit={setName} onChange={setName} disabled={running} />
        <div className={styles.progress} role="group" aria-label={t("newStoryDialog.progress")}>
          <span className={styles.label}>{t("newStoryDialog.progress")}</span>
          <div data-progress-kind={kind}>
            <Segmented
              ariaLabel={t("newStoryDialog.progressKind")}
              size="s"
              options={KINDS.map((k) => ({ value: k, label: kindWord(k), disabled: running }))}
              value={kind}
              onChange={(next) => {
                setKind(next);
                // A variable starts at the pool's free one again, a flag at none.
                setNumber(next === "var" && free !== null && "number" in free ? free.number : 0);
              }}
            />
          </div>
          <FlagField bare kind={kind} label={t("newStoryDialog.progress")} value={number} onCommit={setNumber} disabled={running} depth="bauen" project={project} revision={revision} field="progress" />
          {kind === "var" && free !== null && (
            <p className={styles.note} data-pool-free="">
              {"number" in free ? t("newStoryDialog.stillFree", { free: freeVarsText(free.poolFree) }) : free.refusal}
            </p>
          )}
        </div>
        <TextField label={t("newStoryDialog.notes")} showLabel multiline value={notes} onCommit={setNotes} onChange={setNotes} disabled={running} />
        {running && (
          <p className={styles.note} role="status">
            {t("newStoryDialog.creating")}
          </p>
        )}
        {refusal !== null && <Notice kind="error">{refusal}</Notice>}
      </div>
    </Dialog>
  );
}
