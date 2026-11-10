import type { KeyboardEvent, ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { t as translate } from "../../i18n";
import { Badge } from "./Badge";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import type { RowDataAttrs } from "./ListRow";
import styles from "./StoryNode.module.css";

/** The seven kinds of a story's step, a value of a thread without a step, and a place that sets or checks the value. */
export type StoryNodeKind = "start" | "dialog" | "condition" | "battle" | "reward" | "progress" | "appearance" | "value" | "place";

export type StoryNodeProps = {
  kind: StoryNodeKind;
  /** The step's name, the value, or the place's name. */
  title: string;
  /** A second line: the value of a step („Wert 3“), the source of a place („Scriptdatei 0101“). */
  subtitle?: string;
  selected?: boolean;
  /** A value only the story names, not the game: dashed, with the plakette „nur notiert“. */
  notedOnly?: boolean;
  /** A place whose references carry no value: no edge, the plakette „ohne Wert“. */
  loose?: boolean;
  /** Makes the node a button (click, Enter, Space). A node in a graph leaves it out: the graph selects. */
  onOpen?: () => void;
  /** A control at the end of the head (the graph's „Öffnen“ in a selected place); a node with `onOpen` takes none. */
  action?: ReactNode;
  dataAttrs?: RowDataAttrs;
};

/** The symbol of each kind; its word is `storyNode.kinds.<kind>` in the `ui` catalog. */
const icons: Record<StoryNodeKind, IconName> = {
  start: "play",
  dialog: "stories",
  condition: "tree",
  battle: "trigger",
  reward: "items",
  progress: "flag",
  appearance: "figure",
  value: "thread",
  place: "place",
};

/** Start, Dialog, Bedingung, Kampf, Belohnung, Fortschritt, Erscheinen/Verschwinden, Wert, Ort (German); read when asked, in the language shown. */
export function storyKindWord(kind: StoryNodeKind): string {
  return translate(`ui:storyNode.kinds.${kind}`);
}

/**
 * A node of a story's or a thread's graph: the kind as symbol and word (the head, beside a selected place's control), the title, and a
 * last line with the second line and the states as plaketten (there, never beside the word: „Erscheinen/Verschwinden“ fills the head).
 */
export function StoryNode({ kind, title, subtitle, selected, notedOnly, loose, onOpen, action, dataAttrs }: StoryNodeProps) {
  const { t } = useTranslation("ui");
  const word = t(`storyNode.kinds.${kind}`);
  const icon = icons[kind];
  const className = [styles.node, kind === "place" ? styles.place : styles.value, selected ? styles.selected : "", notedOnly ? styles.notedOnly : "", loose ? styles.loose : ""]
    .filter(Boolean)
    .join(" ");
  const name = `${word}: ${title}${subtitle ? `, ${subtitle}` : ""}`;
  const onKeyDown = onOpen
    ? (event: KeyboardEvent<HTMLDivElement>) => {
        if (event.target !== event.currentTarget || (event.key !== "Enter" && event.key !== " ")) return;
        event.preventDefault();
        onOpen();
      }
    : undefined;
  return (
    <div
      {...dataAttrs}
      data-story-node=""
      data-kind={kind}
      data-selected={selected ? "" : undefined}
      data-noted-only={notedOnly ? "" : undefined}
      data-loose={loose ? "" : undefined}
      className={className}
      title={name}
      role={onOpen ? "button" : undefined}
      tabIndex={onOpen ? 0 : undefined}
      aria-label={onOpen ? name : undefined}
      onClick={onOpen}
      onKeyDown={onKeyDown}
    >
      <span className={styles.head}>
        <span className={styles.kind}>
          <Icon name={icon} />
          <span className={styles.word}>{word}</span>
        </span>
        {!onOpen && action !== undefined && <span className={styles.action}>{action}</span>}
      </span>
      <span className={styles.title}>{title}</span>
      {(subtitle !== undefined || notedOnly || loose) && (
        <span className={styles.foot}>
          {subtitle !== undefined && <span className={styles.subtitle}>{subtitle}</span>}
          {notedOnly && <Badge kind="neutral">{t("storyNode.notedOnly")}</Badge>}
          {loose && <Badge kind="neutral">{t("storyNode.noValue")}</Badge>}
        </span>
      )}
    </div>
  );
}
