import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText, gitInit } from "../api";
import type { GitStatus, InitPhase, InitReport } from "../api";
import { toast } from "../ui/frame/toasts";
import { CheckList, Dialog, Notice } from "../ui/primitives";
import type { CheckRow, DialogAction } from "../ui/primitives";
import { gitFoundText, initDoneText, isNotInstalled, templatesOnlyText } from "./gitWords";
import styles from "./GitInitDialog.module.css";

/** What the `.gitignore` of each layout keeps out: the key of its line in the catalog. */
const IGNORED = { dspre: "gitInitDialog.ignored.dspre", text_project: "gitInitDialog.ignored.textProject" } as const satisfies Record<GitStatus["layout"], string>;

export type GitInitDialogProps = {
  open: boolean;
  /** The Git state as last read (layout, the `git` found, the templates there); null while it is read. */
  status: GitStatus | null;
  /** Unsaved edits of the store or the open scripts: the first commit takes the saved state, so they lock „Einrichten“. */
  unsaved: boolean;
  /** „Abbrechen“, Escape, „Schließen“, and after it went through. */
  onClose: () => void;
  /** It ran (the repository, or at least the templates, are there): the app reads the status again. Also when the commit failed. */
  onDone: (report: InitReport) => void | Promise<void>;
  /** Runs it while nothing else may write (the app's session is busy meanwhile); by default it just runs. */
  exclusive?: <T>(work: () => Promise<T>) => Promise<T>;
  /** Where focus goes once the dialog has closed (`Dialog.finalFocus`), asked then; null or absent: back to the element that had it. */
  finalFocus?: () => HTMLElement | null;
};

/**
 * „Git einrichten?“: what the set-up does and keeps out, the legal note, what is checked before (everything saved, the `git` found, the
 * templates already there), then on „Einrichten“ the phases. While it runs the dialog has no button and stays. A refusal shows its
 * sentence and the dialog can try again; a commit that failed (signing, a hook) shows Git's reason and leaves only „Schließen“, since the
 * repository and the templates are there.
 */
export function GitInitDialog({ open, status, unsaved, onClose, onDone, exclusive = (work) => work(), finalFocus }: GitInitDialogProps) {
  const { t } = useTranslation("git");
  const [running, setRunning] = useState(false);
  const [phase, setPhase] = useState<InitPhase | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  // Read by a click that comes before the render of the one before (a double click).
  const busy = useRef(false);

  // Each opening starts over.
  useEffect(() => {
    if (!open) return;
    setPhase(null);
    setError(null);
    setFailed(null);
  }, [open]);

  const layout = status?.layout ?? "dspre";
  const git = status === null ? undefined : status.cli;
  const rows: CheckRow[] = [
    unsaved ? { id: "saved", state: "error", text: t("shared.saveFirst") } : { id: "saved", state: "ok", text: t("shared.allSaved") },
    git === undefined
      ? { id: "git", state: "info", text: t("shared.lookingForGit") }
      : git === null
        ? { id: "git", state: "warning", text: t("gitInitDialog.gitMissingTemplatesOnly") }
        : { id: "git", state: "ok", text: gitFoundText(git.version) },
    { id: "ignored", state: "info", text: t(IGNORED[layout]) },
    ...(status?.templates ?? []).map((file): CheckRow => ({ id: `template-${file}`, state: "info", text: t("gitInitDialog.templateExists", { file }) })),
  ];
  const ready = !unsaved && git !== undefined;

  const run = async () => {
    if (!ready || busy.current) return;
    busy.current = true;
    setRunning(true);
    setPhase(null);
    setError(null);
    try {
      const report = await exclusive(() => gitInit({}, (p) => setPhase(p.phase)));
      await onDone(report);
      if (report.committed) {
        toast.info(initDoneText(report));
        onClose();
      } else if (isNotInstalled(report)) {
        toast.info(templatesOnlyText());
        onClose();
      } else {
        setFailed(report.reason ?? t("gitInitDialog.failed"));
      }
    } catch (e) {
      setError(errorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  const actions: DialogAction[] = running
    ? []
    : failed !== null
      ? [{ label: t("shared.close"), onClick: onClose }]
      : [
          { label: t("shared.cancel"), onClick: onClose },
          { label: git === null ? t("gitInitDialog.writeTemplates") : t("gitInitDialog.setUp"), kind: "primary", onClick: () => void run(), disabled: !ready },
        ];
  // Before the first phase reports (always in the dev bridge): the first phase of the layout.
  const shownPhase = phase ?? (layout === "dspre" ? "unpack" : "templates");

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next && !busy.current) onClose();
      }}
      dismissible={!running}
      title={t("gitInitDialog.title")}
      description={t("gitInitDialog.description")}
      data={{ "data-git-init-dialog": "" }}
      finalFocus={finalFocus}
      actions={actions}
    >
      <div className={styles.content}>
        <Notice kind="warning">{t("gitInitDialog.legalNote")}</Notice>
        {failed === null && <CheckList ariaLabel={t("gitInitDialog.beforeSetUp")} rows={rows} />}
        {running && (
          <p role="status" className={styles.progress} data-git-init-progress="">
            {t(`gitInitDialog.phase.${shownPhase}`)}
          </p>
        )}
        {failed !== null && <Notice kind="warning">{failed}</Notice>}
        {error !== null && <Notice kind="error">{error}</Notice>}
      </div>
    </Dialog>
  );
}
