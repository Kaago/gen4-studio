import type { KeyboardEvent, ReactNode } from "react";
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

const words: Record<StoryNodeKind, { word: string; icon: IconName }> = {
  start: { word: "Start", icon: "play" },
  dialog: { word: "Dialog", icon: "stories" },
  condition: { word: "Bedingung", icon: "tree" },
  battle: { word: "Kampf", icon: "trigger" },
  reward: { word: "Belohnung", icon: "items" },
  progress: { word: "Fortschritt", icon: "flag" },
  appearance: { word: "Erscheinen/Verschwinden", icon: "figure" },
  value: { word: "Wert", icon: "thread" },
  place: { word: "Ort", icon: "place" },
};

/** Start, Dialog, Bedingung, Kampf, Belohnung, Fortschritt, Erscheinen/Verschwinden, Wert, Ort. */
export function storyKindWord(kind: StoryNodeKind): string {
  return words[kind].word;
}

/** A node of a story's or a thread's graph: the kind as symbol and word, the title, a second line, and its states as plaketten. */
export function StoryNode({ kind, title, subtitle, selected, notedOnly, loose, onOpen, action, dataAttrs }: StoryNodeProps) {
  const { word, icon } = words[kind];
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
          {word}
        </span>
        {notedOnly && <Badge kind="neutral">nur notiert</Badge>}
        {loose && <Badge kind="neutral">ohne Wert</Badge>}
        {!onOpen && action !== undefined && <span className={styles.action}>{action}</span>}
      </span>
      <span className={styles.title}>{title}</span>
      {subtitle !== undefined && <span className={styles.subtitle}>{subtitle}</span>}
    </div>
  );
}
