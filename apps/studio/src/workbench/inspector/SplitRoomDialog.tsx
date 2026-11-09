import { useState } from "react";
import { useTranslation } from "react-i18next";
import { splitRoom } from "../../api";
import type { SplitReport } from "../../api";
import { t } from "../../i18n";
import { Badge, Dialog } from "../../ui/primitives";
import styles from "./SplitRoomDialog.module.css";

/** What splitting does to the room `name` with `maps` maps (null: not known, so no number), and that the copies stay. */
function splitText(name: string, maps: number | null): string {
  return maps === null ? t("workbench:splitRoomDialog.textUnknown", { name }) : t("workbench:splitRoomDialog.text", { name, count: maps });
}

export interface SplitRoomDialogProps {
  open: boolean;
  /** The room's header. */
  header: number;
  /** The room's name, as the world tree calls it (also the label of the undo step). */
  name: string;
  /** How many maps the room has (each gets a copy); null when that is not known: the text names no number. */
  maps: number | null;
  onClose: () => void;
  /** The room has copies of its maps on a matrix of its own (`report`); the dialog has closed. */
  onSplit: (report: SplitReport) => void | Promise<void>;
  /** A refusal of the kernel (the room shares nothing, a world map) or a failure; the dialog stays open. */
  onError: (e: unknown) => void;
}

/** „Raum abspalten?“: gives a shared room copies of its maps; the copies cannot be undone. Mounted anew for every opening. */
export function SplitRoomDialog(props: SplitRoomDialogProps) {
  return props.open ? <SplitForm {...props} /> : null;
}

function SplitForm({ header, name, maps, onClose, onSplit, onError }: SplitRoomDialogProps) {
  // Renders the dialog again when the language changes (`splitText` reads the catalog at render time).
  useTranslation("workbench");
  const [working, setWorking] = useState(false);

  async function split() {
    if (working) return;
    setWorking(true);
    let report: SplitReport;
    try {
      report = await splitRoom(header, name);
    } catch (e) {
      setWorking(false);
      onError(e);
      return;
    }
    onClose();
    try {
      await onSplit(report);
    } catch (e) {
      onError(e);
    }
  }
  const leave = () => {
    if (!working) onClose();
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) leave();
      }}
      title={t("workbench:splitRoomDialog.title")}
      actions={[
        { label: t("workbench:splitRoomDialog.cancel"), onClick: leave },
        { label: t("workbench:splitRoomDialog.split"), kind: "primary", onClick: () => void split(), disabled: working },
      ]}
    >
      <div className={styles.form} data-split-room="" aria-busy={working}>
        <Badge kind="irreversible" />
        <p className={styles.text}>{splitText(name, maps)}</p>
      </div>
    </Dialog>
  );
}
