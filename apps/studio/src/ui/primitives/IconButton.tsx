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
  /** Blocked for a moment (a build runs): looks disabled through `aria-disabled` but keeps keyboard focus; a click does nothing. */
  busy?: boolean;
  onClick: () => void;
};

export function IconButton({ label, icon, shortcut, pressed, disabled, busy = false, onClick }: IconButtonProps) {
  const click = () => {
    if (!busy) onClick();
  };
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
      <button type="button" className={styles.button} aria-label={label} aria-disabled={busy || undefined} disabled={disabled} onClick={click}>
        {glyph}
      </button>
    ) : (
      <Toggle
        className={styles.button}
        aria-label={label}
        pressed={pressed}
        aria-disabled={busy || undefined}
        disabled={disabled}
        onClick={click}
        render={<button type="button" />}
      >
        {glyph}
      </Toggle>
    );
  return <Tooltip content={tip}>{button}</Tooltip>;
}
