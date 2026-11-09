import { useTranslation } from "react-i18next";
import { t } from "../i18n";
import { IconButton } from "../ui/primitives";
import { toast } from "../ui/frame/toasts";
import styles from "./EntityIdLine.module.css";

/** Copies `id` to the clipboard and says so; outside a secure context (a browser on another host) it says why it cannot. */
function copy(id: string) {
  const clipboard = navigator.clipboard as Clipboard | undefined;
  if (clipboard === undefined) {
    toast.error(t("textproject:entityIdLine.noClipboard"));
    return;
  }
  clipboard.writeText(id).then(
    () => toast.info(t("textproject:entityIdLine.copied", { id })),
    () => toast.error(t("textproject:entityIdLine.copyFailed")),
  );
}

/** The quiet id line of a head in a text project: the entity's id in mono, and a small button that copies it. */
export function EntityIdLine({ id }: { id: string }) {
  const { t } = useTranslation("textproject");
  return (
    <span className={styles.line}>
      <code className={styles.id} data-entity-id="">
        {id}
      </code>
      <IconButton label={t("entityIdLine.copy")} icon="copy" onClick={() => copy(id)} />
    </span>
  );
}
