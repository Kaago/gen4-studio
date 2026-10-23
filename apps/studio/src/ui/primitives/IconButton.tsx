import { Toggle } from "@base-ui/react/toggle";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import { Kbd } from "./Kbd";
import { Tooltip } from "./Tooltip";
import styles from "./IconButton.module.css";

export type IconButtonProps = {
  /** Required: becomes the accessible name and the tooltip. */
  label: string;
  icon: IconName;
  shortcut?: string;
  /** Present for toggle buttons: sets `aria-pressed`. */
  pressed?: boolean;
  disabled?: boolean;
  onClick: () => void;
};

export function IconButton({ label, icon, shortcut, pressed, disabled, onClick }: IconButtonProps) {
  const tip = (
    <>
      {label}
      {shortcut && (
        <>
          {" "}
          <Kbd keys={shortcut} />
        </>
      )}
    </>
  );
  const glyph = <Icon name={icon} weight={pressed ? "fill" : "regular"} />;
  const button =
    pressed === undefined ? (
      <button type="button" className={styles.button} aria-label={label} disabled={disabled} onClick={onClick}>
        {glyph}
      </button>
    ) : (
      <Toggle
        className={styles.button}
        aria-label={label}
        pressed={pressed}
        disabled={disabled}
        onClick={onClick}
        render={<button type="button" />}
      >
        {glyph}
      </Toggle>
    );
  return <Tooltip content={tip}>{button}</Tooltip>;
}
