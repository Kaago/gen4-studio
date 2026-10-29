import { IconButton } from "../ui/primitives";
import { toast } from "../ui/frame/toasts";
import styles from "./EntityIdLine.module.css";

/** Copies `id` to the clipboard and says so; outside a secure context (a browser on another host) it says why it cannot. */
function copy(id: string) {
  const clipboard = navigator.clipboard as Clipboard | undefined;
  if (clipboard === undefined) {
    toast.error("Die Zwischenablage ist hier nicht verfügbar. Markiere die ID und kopiere sie selbst.");
    return;
  }
  clipboard.writeText(id).then(
    () => toast.info(`ID kopiert: ${id}`),
    () => toast.error("Die ID ließ sich nicht kopieren. Markiere sie und kopiere sie selbst."),
  );
}

/** The quiet id line of a head in a text project: the entity's id in mono, and a small button that copies it. */
export function EntityIdLine({ id }: { id: string }) {
  return (
    <span className={styles.line}>
      <code className={styles.id} data-entity-id="">
        {id}
      </code>
      <IconButton label="ID kopieren" icon="copy" onClick={() => copy(id)} />
    </span>
  );
}
